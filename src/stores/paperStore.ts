import { defineStore } from 'pinia';
import { ref, computed, watch } from 'vue';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { Paper } from '@/types';
import * as paperApi from '@/api/paperApi';
import * as aiApi from '@/api/aiApi';
import { useProjectStore } from './projectStore';
import { useGraphStore } from './graphStore';
import { useChatStore } from './chatStore';

interface ParseProgressEvent {
  paperId: string;
  success: boolean;
  error?: string;
  done: number;
  total: number;
}

type SortField = 'title' | 'year' | 'author' | 'added';
type SortDir = 'asc' | 'desc';

export const usePaperStore = defineStore('paper', () => {
  const papers = ref<Paper[]>([]);
  const selectedPaperId = ref<string | null>(null);
  const isLoading = ref(false);
  const isAutoResolving = ref(false);
  const parsingPaperIds = ref(new Set<string>());
  const pendingPaperIds = ref(new Set<string>());
  const pdfScrollPositions = ref<Record<string, number>>({});
  const pdfZoomLevels = ref<Record<string, number>>({});
  const parseProgress = ref<{ done: number; total: number } | null>(null);
  const parseErrors = ref<Array<{ paperId: string; paperTitle: string; error: string }>>([]);

  // Search / sort / filter state
  const searchQuery = ref('');
  const sortField = ref<SortField>((localStorage.getItem('gf_sort_field') as SortField) || 'added');
  const sortDir = ref<SortDir>((localStorage.getItem('gf_sort_dir') as SortDir) || 'desc');
  const activeTag = ref<string | null>(null);

  // Persist sort preferences
  watch(sortField, (v) => localStorage.setItem('gf_sort_field', v));
  watch(sortDir, (v) => localStorage.setItem('gf_sort_dir', v));

  const selectedPaper = computed(() =>
    papers.value.find((p) => p.id === selectedPaperId.value) ?? null
  );

  const allTags = computed(() => {
    const tagSet = new Set<string>();
    for (const p of papers.value) {
      for (const tag of p.tags) {
        tagSet.add(tag);
      }
    }
    return Array.from(tagSet).sort();
  });

  const filteredPapers = computed(() => {
    let result = papers.value;

    // Tag filter
    if (activeTag.value) {
      result = result.filter(p => p.tags.includes(activeTag.value!));
    }

    // Search
    const q = searchQuery.value.trim().toLowerCase();
    if (q) {
      result = result.filter(p => {
        const title = (p.title || '').toLowerCase();
        const authors = p.authors.join(' ').toLowerCase();
        const tags = p.tags.join(' ').toLowerCase();
        const abstract = (p.abstract || '').toLowerCase();
        return title.includes(q)
          || authors.includes(q)
          || tags.includes(q)
          || abstract.includes(q);
      });
    }

    // Sort
    const dir = sortDir.value === 'asc' ? 1 : -1;
    result = [...result].sort((a, b) => {
      switch (sortField.value) {
        case 'title':
          return (a.title || '').localeCompare(b.title || '') * dir;
        case 'year':
          return ((a.year || 0) - (b.year || 0)) * dir;
        case 'author':
          return (a.authors[0] || '').localeCompare(b.authors[0] || '') * dir;
        case 'added':
        default:
          return (a.createdAt.localeCompare(b.createdAt)) * dir;
      }
    });

    return result;
  });

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
    parseErrors.value = [];
    parseProgress.value = { done: 0, total: newPapers.length };

    // Read advanced settings
    const autoParse = settings.advanced?.autoParse ?? true;

    // Mark all new papers as pending + parsing
    for (const p of newPapers) {
      pendingPaperIds.value.add(p.id);
      parsingPaperIds.value.add(p.id);
    }

    // Listen for progress events
    let unlisten: UnlistenFn | null = null;
    try {
      unlisten = await listen<ParseProgressEvent>('parse_progress', (event) => {
        const { paperId, success, error, done, total } = event.payload;
        parseProgress.value = { done, total };
        // Remove from parsing set as each completes
        parsingPaperIds.value.delete(paperId);
        pendingPaperIds.value.delete(paperId);
        if (!success && error) {
          const paper = newPapers.find(p => p.id === paperId);
          parseErrors.value.push({
            paperId,
            paperTitle: paper?.title || paperId,
            error,
          });
        }
      });
    } catch {
      // Event listener setup failed — non-critical
    }

    try {
      if (autoParse) {
        // Batch parse — single project load/save on backend, all papers concurrent
        const paperIds = newPapers.map(p => p.id);
        const resultMap = await aiApi.aiParsePdfsBatch(projectStore.projectPath, paperIds);

        // Apply results to local papers (resultMap is paperId → metadata)
        for (const [paperId, metadata] of Object.entries(resultMap)) {
          const idx = papers.value.findIndex(p => p.id === paperId);
          if (idx === -1) continue;

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
          parsedIds.push(paperId);
        }

        if (projectStore.currentProject) {
          projectStore.currentProject.papers = papers.value;
          projectStore.scheduleAutoSave();
        }
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
      unlisten?.();
      isAutoResolving.value = false;
      pendingPaperIds.value.clear();
      parsingPaperIds.value.clear();
      parseProgress.value = null;
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

  function savePdfZoom(paperId: string, zoom: number) {
    pdfZoomLevels.value[paperId] = zoom;
  }

  function getPdfZoom(paperId: string): number {
    return pdfZoomLevels.value[paperId] ?? 1.2;
  }

  async function reparsePaper(paperId: string) {
    const projectStore = useProjectStore();
    if (!projectStore.projectPath) return;

    const idx = papers.value.findIndex(p => p.id === paperId);
    if (idx === -1) return;

    parsingPaperIds.value.add(paperId);
    try {
      const resultMap = await aiApi.aiParsePdfsBatch(projectStore.projectPath, [paperId]);
      const metadata = resultMap[paperId];
      if (metadata) {
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
        if (projectStore.currentProject) {
          projectStore.currentProject.papers = papers.value;
          projectStore.scheduleAutoSave();
        }
      }
    } finally {
      parsingPaperIds.value.delete(paperId);
    }
  }

  function clearParseErrors() {
    parseErrors.value = [];
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
    parseProgress,
    parseErrors,
    clearParseErrors,
    // Search / sort / filter
    searchQuery,
    sortField,
    sortDir,
    activeTag,
    allTags,
    filteredPapers,
    loadFromProject,
    selectPaper,
    importPdfs,
    updatePaper,
    deletePaper,
    reparsePaper,
    savePdfScrollPosition,
    getPdfScrollPosition,
    savePdfZoom,
    getPdfZoom,
  };
});
