import { defineStore } from 'pinia';
import { ref } from 'vue';
import type { Relation, GraphLayout, RelationRecommendation } from '@/types';
import * as graphApi from '@/api/graphApi';
import * as aiApi from '@/api/aiApi';
import { useProjectStore } from './projectStore';

export const useGraphStore = defineStore('graph', () => {
  const relations = ref<Relation[]>([]);
  const graphLayout = ref<GraphLayout>({ locked: false, positions: {} });
  const selectedRelationId = ref<string | null>(null);
  const pendingRecommendations = ref<RelationRecommendation[]>([]);
  const isAutoRecommending = ref(false);

  let lastRelationCheck = 0;
  const RELATION_CHECK_COOLDOWN = 10000;

  function loadFromProject(
    projectRelations: Relation[],
    layout: GraphLayout
  ) {
    relations.value = projectRelations;
    graphLayout.value = layout;
  }

  async function addRelation(relation: Relation) {
    const projectStore = useProjectStore();
    if (!projectStore.projectPath) return relation;

    const added = await graphApi.addRelation(
      projectStore.projectPath,
      relation
    );
    relations.value.push(added);
    if (projectStore.currentProject) {
      projectStore.currentProject.relations = relations.value;
      projectStore.scheduleAutoSave();
    }
    return added;
  }

  async function addRelationsBatch(newRelations: Relation[]) {
    const projectStore = useProjectStore();
    if (!projectStore.projectPath || newRelations.length === 0) return;

    // Single batch API call instead of N individual calls
    const added = await graphApi.addRelationsBatch(projectStore.projectPath, newRelations);

    relations.value.push(...added);
    if (projectStore.currentProject) {
      projectStore.currentProject.relations = relations.value;
      projectStore.scheduleAutoSave();
    }
  }

  async function updateRelation(relation: Relation) {
    const projectStore = useProjectStore();
    if (!projectStore.projectPath) return relation;

    const updated = await graphApi.updateRelation(
      projectStore.projectPath,
      relation
    );
    const idx = relations.value.findIndex((r) => r.id === relation.id);
    if (idx !== -1) {
      relations.value[idx] = updated;
    }
    if (projectStore.currentProject) {
      projectStore.currentProject.relations = relations.value;
      projectStore.scheduleAutoSave();
    }
    return updated;
  }

  async function deleteRelation(relationId: string) {
    const projectStore = useProjectStore();
    if (!projectStore.projectPath) return;

    await graphApi.deleteRelation(projectStore.projectPath, relationId);
    relations.value = relations.value.filter((r) => r.id !== relationId);
    if (selectedRelationId.value === relationId) {
      selectedRelationId.value = null;
    }
    if (projectStore.currentProject) {
      projectStore.currentProject.relations = relations.value;
      projectStore.scheduleAutoSave();
    }
  }

  async function saveLayout() {
    const projectStore = useProjectStore();
    if (!projectStore.projectPath) return;

    await graphApi.saveGraphLayout(
      projectStore.projectPath,
      graphLayout.value
    );
    if (projectStore.currentProject) {
      projectStore.currentProject.graphLayout = graphLayout.value;
      projectStore.scheduleAutoSave();
    }
  }

  function selectRelation(id: string | null) {
    selectedRelationId.value = id;
  }

  function removeRelationsForPaper(paperId: string) {
    relations.value = relations.value.filter(
      (r) => r.sourceId !== paperId && r.targetId !== paperId
    );
    const ps = useProjectStore();
    if (ps.currentProject) {
      ps.currentProject.relations = relations.value;
    }
  }

  async function autoRecommendRelations(paperCount: number, autoAccept = false, newPaperIds?: string[]) {
    const projectStore = useProjectStore();
    if (!projectStore.projectPath) return;
    if (paperCount < 2) return;
    if (isAutoRecommending.value) return;

    if (!autoAccept) {
      const now = Date.now();
      if (now - lastRelationCheck < RELATION_CHECK_COOLDOWN) return;
    }
    lastRelationCheck = Date.now();

    isAutoRecommending.value = true;
    try {
      // Timeout after 120s to prevent hanging
      const recommendations = await Promise.race([
        aiApi.aiRecommendRelations(projectStore.projectPath, newPaperIds),
        new Promise<[]>((_, reject) =>
          setTimeout(() => reject(new Error('AI recommend timeout after 120s')), 120000)
        ),
      ]);
      const newRecs = recommendations.filter((rec) => {
        return !relations.value.some(
          (r) =>
            r.sourceId === rec.sourceId &&
            r.targetId === rec.targetId &&
            r.type === rec.type
        );
      });

      if (autoAccept) {
        const toAdd: Relation[] = newRecs.map((rec) => ({
          id: crypto.randomUUID(),
          sourceId: rec.sourceId,
          targetId: rec.targetId,
          type: rec.type as Relation['type'],
          evidence: rec.evidence,
          isManual: false,
          confidence: rec.confidence,
          createdAt: new Date().toISOString(),
          updatedAt: new Date().toISOString(),
        }));
        await addRelationsBatch(toAdd);
      } else {
        pendingRecommendations.value = newRecs;
      }
    } catch (e) {
      console.error('[graphStore] autoRecommendRelations failed:', e);
    } finally {
      isAutoRecommending.value = false;
    }
  }

  async function acceptRecommendation(rec: RelationRecommendation) {
    const newRelation: Relation = {
      id: crypto.randomUUID(),
      sourceId: rec.sourceId,
      targetId: rec.targetId,
      type: rec.type as Relation['type'],
      evidence: rec.evidence,
      isManual: false,
      confidence: rec.confidence,
      createdAt: new Date().toISOString(),
      updatedAt: new Date().toISOString(),
    };
    await addRelation(newRelation);
    pendingRecommendations.value = pendingRecommendations.value.filter(
      (r) => r !== rec
    );
  }

  function dismissRecommendation(rec: RelationRecommendation) {
    pendingRecommendations.value = pendingRecommendations.value.filter(
      (r) => r !== rec
    );
  }

  return {
    relations,
    graphLayout,
    selectedRelationId,
    pendingRecommendations,
    isAutoRecommending,
    loadFromProject,
    addRelation,
    addRelationsBatch,
    updateRelation,
    deleteRelation,
    saveLayout,
    selectRelation,
    removeRelationsForPaper,
    autoRecommendRelations,
    acceptRecommendation,
    dismissRecommendation,
  };
});
