import { defineStore } from 'pinia';
import { ref, computed } from 'vue';
import type { Paper } from '@/types';
import * as paperApi from '@/api/paperApi';
import * as aiApi from '@/api/aiApi';
import { useProjectStore } from './projectStore';
import { useGraphStore } from './graphStore';
import { useChatStore } from './chatStore';

export const usePaperStore = defineStore('paper', () => {
  const papers = ref<Paper[]>([]);
  const selectedPaperId = ref<string | null>(null);
  const isLoading = ref(false);
  const isAutoResolving = ref(false);
  const parsingPaperIds = ref(new Set<string>());
  const pendingPaperIds = ref(new Set<string>());
  const pdfScrollPositions = ref<Record<string, number>>({});

  const selectedPaper = computed(() =>
    papers.value.find((p) => p.id === selectedPaperId.value) ?? null
  );

  function loadFromProject(projectPapers: Paper[]) {
    papers.value = projectPapers;
    if (
      selectedPaperId.value &&
      !papers.value.find((p) => p.id === selectedPaperId.value)
    ) {
      selectedPaperId.value = null;
    }
  }

  function selectPaper(id: string | null) {
    selectedPaperId.value = id;
  }

  async function importPdfs(filePaths: string[]) {
    const projectStore = useProjectStore();
    if (!projectStore.projectPath) return [];

    isLoading.value = true;
    try {
      const newPapers = await paperApi.importPdfs(
        projectStore.projectPath,
        filePaths
      );
      papers.value.push(...newPapers);
      if (projectStore.currentProject) {
        projectStore.currentProject.papers = papers.value;
        projectStore.scheduleAutoSave();
      }

      // Auto-sync embeddings if knowledge base is built
      useChatStore().autoSyncEmbeddings(projectStore.projectPath);

      // Parse metadata then update relations (background, non-blocking)
      resolveAndRecommendRelations(newPapers);

      return newPapers;
    } finally {
      isLoading.value = false;
    }
  }

  async function parseOnePaper(paper: Paper, projectPath: string) {
    parsingPaperIds.value.add(paper.id);
    try {
      const metadata = await aiApi.aiParsePdf(projectPath, paper.id);
      const idx = papers.value.findIndex((p) => p.id === paper.id);
      if (idx !== -1) {
        if (metadata.title) papers.value[idx].title = metadata.title;
        if (metadata.authors?.length) papers.value[idx].authors = metadata.authors;
        if (metadata.year) papers.value[idx].year = metadata.year;
        if (metadata.abstract) papers.value[idx].abstract = metadata.abstract;
        papers.value[idx].metadata = {
          ...metadata,
          isAiGenerated: true,
          source: 'ai',
        };
        papers.value[idx].updatedAt = new Date().toISOString();
        await paperApi.updatePaper(projectPath, papers.value[idx]);
        return true;
      }
    } catch {
      // Individual parse failed, skip
    } finally {
      parsingPaperIds.value.delete(paper.id);
    }
    return false;
  }

  async function resolveAndRecommendRelations(newPapers: Paper[]) {
    const projectStore = useProjectStore();
    if (!projectStore.projectPath) return;

    const settings = await aiApi.getAiSettingsMasked();
    const hasAi = settings.activeProvider
      || settings.openaiCompatible?.enabled
      || settings.anthropic?.enabled;
    if (!hasAi) {
      console.warn('[paperStore] No AI provider configured, skipping auto-parse');
      return;
    }

    isAutoResolving.value = true;
    const parsedIds: string[] = [];

    // Read advanced settings
    const concurrency = settings.advanced?.concurrency ?? 3;
    const autoParse = settings.advanced?.autoParse ?? true;
    const retryCount = settings.advanced?.retryCount ?? 1;

    // Mark all new papers as pending
    for (const p of newPapers) {
      pendingPaperIds.value.add(p.id);
    }

    try {
      if (autoParse) {
        // Batch parse with configured concurrency
        for (let i = 0; i < newPapers.length; i += concurrency) {
          const batch = newPapers.slice(i, i + concurrency);
          const results = await Promise.all(
            batch.map(async (p) => {
              for (let attempt = 0; attempt <= retryCount; attempt++) {
                if (await parseOnePaper(p, projectStore.projectPath!)) return true;
                if (attempt < retryCount) {
                  await new Promise(r => setTimeout(r, 1000 * (attempt + 1)));
                }
              }
              return false;
            })
          );
          batch.forEach((p, j) => {
            if (results[j]) parsedIds.push(p.id);
            pendingPaperIds.value.delete(p.id);
          });
        }
      }

      if (projectStore.currentProject) {
        projectStore.currentProject.papers = papers.value;
        projectStore.scheduleAutoSave();
      }

      // Recommend relations even if parsing partially failed — use all papers with metadata
      const graphStore = useGraphStore();
      await graphStore.autoRecommendRelations(
        papers.value.length,
        true,
        parsedIds.length > 0 ? parsedIds : undefined
      );
    } catch (e) {
      console.error('[paperStore] resolveAndRecommendRelations failed:', e);
    } finally {
      isAutoResolving.value = false;
      pendingPaperIds.value.clear();
    }
  }

  async function updatePaper(paper: Paper) {
    const projectStore = useProjectStore();
    if (!projectStore.projectPath) return paper;

    const updated = await paperApi.updatePaper(
      projectStore.projectPath,
      paper
    );
    const idx = papers.value.findIndex((p) => p.id === paper.id);
    if (idx !== -1) {
      papers.value[idx] = updated;
    }
    if (projectStore.currentProject) {
      projectStore.currentProject.papers = papers.value;
      projectStore.scheduleAutoSave();
    }
    return updated;
  }

  async function deletePaper(paperId: string) {
    const projectStore = useProjectStore();
    if (!projectStore.projectPath) return;

    await paperApi.deletePaper(projectStore.projectPath, paperId);
    papers.value = papers.value.filter((p) => p.id !== paperId);
    if (selectedPaperId.value === paperId) {
      selectedPaperId.value = null;
    }

    // Remove relations involving the deleted paper (backend does the same)
    const graphStore = useGraphStore();
    graphStore.removeRelationsForPaper(paperId);

    if (projectStore.currentProject) {
      projectStore.currentProject.papers = papers.value;
      projectStore.currentProject.relations = graphStore.relations;
      projectStore.scheduleAutoSave();
    }

    // Auto-sync embeddings if knowledge base is built
    useChatStore().autoSyncEmbeddings(projectStore.projectPath);
  }

  function isPaperParsing(id: string) {
    return parsingPaperIds.value.has(id);
  }

  function isPaperQueued(id: string) {
    return pendingPaperIds.value.has(id) && !parsingPaperIds.value.has(id);
  }

  function savePdfScrollPosition(paperId: string, scrollTop: number) {
    pdfScrollPositions.value[paperId] = scrollTop;
  }

  function getPdfScrollPosition(paperId: string): number {
    return pdfScrollPositions.value[paperId] ?? 0;
  }

  return {
    papers,
    selectedPaperId,
    selectedPaper,
    isLoading,
    isAutoResolving,
    parsingPaperIds,
    isPaperParsing,
    isPaperQueued,
    pdfScrollPositions,
    loadFromProject,
    selectPaper,
    importPdfs,
    updatePaper,
    deletePaper,
    savePdfScrollPosition,
    getPdfScrollPosition,
  };
});
