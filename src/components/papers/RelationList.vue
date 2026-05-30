<script setup lang="ts">
import { ref, computed } from 'vue';
import type { Paper, Relation, RelationType } from '@/types';
import { relationTypes } from '@/types/relation';
import { useGraphStore } from '@/stores/graphStore';
import { usePaperStore } from '@/stores/paperStore';
import { useProjectStore } from '@/stores/projectStore';
import { aiRecommendRelations } from '@/api/aiApi';
import { useI18n } from 'vue-i18n';

const { t } = useI18n();

const props = defineProps<{
  paper: Paper;
}>();

const graphStore = useGraphStore();
const paperStore = usePaperStore();
const projectStore = useProjectStore();

const showAddForm = ref(false);
const isRecommending = ref(false);
const targetId = ref('');
const relationType = ref<RelationType>('supports');
const evidence = ref('');
const searchQuery = ref('');
const recommendations = ref<Array<{
  sourceId: string;
  targetId: string;
  type: RelationType;
  confidence: number;
  evidence: string;
}>>([]);

const isAnalyzing = computed(() =>
  graphStore.isAutoRecommending || paperStore.isAutoResolving
);

const relatedRelations = computed(() =>
  graphStore.relations.filter(
    (r) => r.sourceId === props.paper.id || r.targetId === props.paper.id
  )
);

const otherPapers = computed(() =>
  paperStore.papers.filter((p) => p.id !== props.paper.id)
);

const filteredPapers = computed(() => {
  if (!searchQuery.value) return otherPapers.value;
  const q = searchQuery.value.toLowerCase();
  return otherPapers.value.filter(
    (p) =>
      p.title.toLowerCase().includes(q) ||
      p.authors.some((a) => a.toLowerCase().includes(q))
  );
});

function getPaperTitle(id: string) {
  return paperStore.papers.find((p) => p.id === id)?.title ?? '未知文献';
}

function isSource(relation: Relation) {
  return relation.sourceId === props.paper.id;
}

function getOtherPaperId(relation: Relation) {
  return isSource(relation) ? relation.targetId : relation.sourceId;
}

async function handleAddRelation() {
  if (!targetId.value || !evidence.value.trim()) return;

  const now = new Date().toISOString();
  const relation: Relation = {
    id: crypto.randomUUID(),
    sourceId: props.paper.id,
    targetId: targetId.value,
    type: relationType.value,
    evidence: evidence.value.trim(),
    isManual: true,
    createdAt: now,
    updatedAt: now,
  };

  await graphStore.addRelation(relation);
  showAddForm.value = false;
  targetId.value = '';
  relationType.value = 'supports';
  evidence.value = '';
  searchQuery.value = '';
}

async function handleDeleteRelation(relationId: string) {
  await graphStore.deleteRelation(relationId);
}

async function handleRecommendRelations() {
  if (!projectStore.projectPath || isRecommending.value) return;
  isRecommending.value = true;
  try {
    recommendations.value = await aiRecommendRelations(projectStore.projectPath);
  } finally {
    isRecommending.value = false;
  }
}

async function handleApplyRecommendation(rec: typeof recommendations.value[0]) {
  const now = new Date().toISOString();
  const relation: Relation = {
    id: crypto.randomUUID(),
    sourceId: rec.sourceId,
    targetId: rec.targetId,
    type: rec.type,
    evidence: rec.evidence,
    isManual: false,
    confidence: rec.confidence,
    createdAt: now,
    updatedAt: now,
  };
  await graphStore.addRelation(relation);
  recommendations.value = recommendations.value.filter((r) => r !== rec);
}

async function handleApplyAll() {
  const now = new Date().toISOString();
  const toAdd: Relation[] = recommendations.value.map((rec) => ({
    id: crypto.randomUUID(),
    sourceId: rec.sourceId,
    targetId: rec.targetId,
    type: rec.type,
    evidence: rec.evidence,
    isManual: false,
    confidence: rec.confidence,
    createdAt: now,
    updatedAt: now,
  }));
  await graphStore.addRelationsBatch(toAdd);
  recommendations.value = [];
}

function selectTarget(id: string) {
  targetId.value = id;
  searchQuery.value = '';
}
</script>

<template>
  <div class="relation-list">
    <div class="relation-header">
      <span class="relation-count">{{ t('graph.relationCount', { count: relatedRelations.length }) }}</span>
      <div class="header-actions">
        <button class="action-btn" @click="showAddForm = !showAddForm">
          {{ showAddForm ? t('common.cancel') : t('graph.addRelation') }}
        </button>
        <button
          class="action-btn"
          :disabled="isRecommending"
          @click="handleRecommendRelations"
        >
          {{ isRecommending ? t('graph.recommendLoading') : t('graph.recommendAgain') }}
        </button>
      </div>
    </div>

    <!-- Add relation form -->
    <div v-if="showAddForm" class="add-form">
      <div class="form-field">
        <label>{{ t('graph.targetPaper') }}</label>
        <input
          v-model="searchQuery"
          class="form-input"
          :placeholder="t('graph.searchPaper')"
        />
        <div v-if="searchQuery && !targetId" class="search-results">
          <button
            v-for="p in filteredPapers.slice(0, 5)"
            :key="p.id"
            class="search-result"
            @click="selectTarget(p.id)"
          >
            <span class="result-title">{{ p.title }}</span>
            <span class="result-author">{{ p.authors[0] ?? '' }}</span>
          </button>
        </div>
        <div v-if="targetId" class="selected-target">
          {{ getPaperTitle(targetId) }}
          <button class="clear-btn" @click="targetId = ''">×</button>
        </div>
      </div>
      <div class="form-field">
        <label>{{ t('graph.relationType') }}</label>
        <select v-model="relationType" class="form-select">
          <option v-for="rt in relationTypes" :key="rt" :value="rt">
            {{ t('relations.' + rt) }}
          </option>
        </select>
      </div>
      <div class="form-field">
        <label>{{ t('graph.evidence') }}</label>
        <textarea
          v-model="evidence"
          class="form-textarea"
          :placeholder="t('graph.relationEvidencePlaceholder')"
        />
      </div>
      <button
        class="submit-btn"
        :disabled="!targetId || !evidence.trim()"
        @click="handleAddRelation"
      >
        {{ t('common.save') }}
      </button>
    </div>

    <!-- AI recommendations -->
    <div v-if="recommendations.length" class="recommendations">
      <div class="section-label">
        {{ t('graph.recommendedRelations') }}
        <button v-if="recommendations.length > 1" class="apply-btn apply-all" @click="handleApplyAll">
          {{ t('graph.applyAll') }}
        </button>
      </div>
      <div
        v-for="(rec, i) in recommendations"
        :key="i"
        class="recommendation"
      >
        <div class="rec-info">
          <span class="rec-type">{{ t('relations.' + rec.type) }}</span>
          <span class="rec-target">
            → {{ getPaperTitle(rec.targetId) }}
          </span>
          <span class="rec-confidence">
            {{ Math.round(rec.confidence * 100) }}%
          </span>
        </div>
        <p class="rec-evidence">{{ rec.evidence }}</p>
        <button class="apply-btn" @click="handleApplyRecommendation(rec)">
          {{ t('graph.applyRelation') }}
        </button>
      </div>
    </div>

    <!-- Existing relations -->
    <div class="relations">
      <!-- Loading indicator while analyzing -->
      <div v-if="isAnalyzing" class="analyzing-indicator">
        <div class="spinner" />
        <span>{{ t('graph.analyzing') }}</span>
      </div>
      <div
        v-for="relation in relatedRelations"
        :key="relation.id"
        class="relation-item"
      >
        <div class="relation-main">
          <span class="relation-type" :class="relation.type">
            {{ isSource(relation) ? '→' : '←' }}
            {{ t('relations.' + relation.type) }}
          </span>
          <span class="relation-target">
            {{ getPaperTitle(getOtherPaperId(relation)) }}
          </span>
        </div>
        <p class="relation-evidence">{{ relation.evidence }}</p>
        <button
          class="delete-btn"
          @click="handleDeleteRelation(relation.id)"
        >
          {{ t('common.delete') }}
        </button>
      </div>
      <div v-if="!isAnalyzing && relatedRelations.length === 0" class="empty-relations">
        {{ t('graph.noRelations') }}
      </div>
    </div>
  </div>
</template>

<style lang="scss" scoped>
.relation-list {
  display: flex;
  flex-direction: column;
  gap: $spacing-md;
}

.relation-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.relation-count {
  font-size: 13px;
  color: $color-text-secondary;
}

.header-actions {
  display: flex;
  gap: $spacing-sm;
}

.action-btn {
  padding: 4px 10px;
  border: 1px solid $color-border;
  border-radius: $radius-sm;
  background: $color-bg;
  color: $color-text-primary;
  font-size: 12px;
  cursor: pointer;
  font-family: $font-family;

  &:hover:not(:disabled) {
    background: $color-panel;
  }

  &:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
}

.add-form {
  border: 1px solid $color-border;
  border-radius: $radius-sm;
  padding: $spacing-md;
  display: flex;
  flex-direction: column;
  gap: $spacing-md;
}

.form-field {
  display: flex;
  flex-direction: column;
  gap: $spacing-xs;
  position: relative;

  label {
    font-size: 12px;
    color: $color-text-secondary;
  }
}

.form-input,
.form-select,
.form-textarea {
  padding: $spacing-sm $spacing-md;
  border: 1px solid $color-border;
  border-radius: $radius-sm;
  font-size: 13px;
  font-family: $font-family;

  &:focus {
    outline: none;
    border-color: $color-node-border;
  }
}

.form-textarea {
  min-height: 60px;
  resize: vertical;
}

.search-results {
  position: absolute;
  top: 100%;
  left: 0;
  right: 0;
  background: $color-bg;
  border: 1px solid $color-border;
  border-radius: $radius-sm;
  box-shadow: $shadow-md;
  z-index: 10;
  max-height: 200px;
  overflow-y: auto;
}

.search-result {
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding: $spacing-sm $spacing-md;
  border: none;
  background: none;
  text-align: left;
  cursor: pointer;
  width: 100%;
  font-family: $font-family;

  &:hover {
    background: $color-panel;
  }
}

.result-title {
  font-size: 12px;
  color: $color-text-primary;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.result-author {
  font-size: 11px;
  color: $color-text-disabled;
}

.selected-target {
  display: flex;
  align-items: center;
  gap: $spacing-sm;
  padding: $spacing-sm $spacing-md;
  background: $color-panel;
  border-radius: $radius-sm;
  font-size: 12px;
}

.clear-btn {
  margin-left: auto;
  border: none;
  background: none;
  color: $color-text-disabled;
  cursor: pointer;
  font-size: 14px;
}

.submit-btn {
  padding: $spacing-sm;
  background: $color-text-primary;
  color: $color-bg;
  border: none;
  border-radius: $radius-sm;
  font-size: 13px;
  cursor: pointer;
  font-family: $font-family;

  &:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
}

.recommendations {
  border: 1px solid $color-border;
  border-radius: $radius-sm;
  padding: $spacing-md;
}

.section-label {
  font-size: 12px;
  color: $color-text-disabled;
  margin-bottom: $spacing-sm;
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.apply-all {
  font-weight: 500;
  color: $color-text-primary;
}

.recommendation {
  padding: $spacing-sm 0;
  border-bottom: 1px solid $color-border;

  &:last-child {
    border-bottom: none;
  }
}

.rec-info {
  display: flex;
  align-items: center;
  gap: $spacing-sm;
}

.rec-type {
  font-size: 12px;
  font-weight: 500;
}

.rec-target {
  font-size: 12px;
  color: $color-text-secondary;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  flex: 1;
}

.rec-confidence {
  font-size: 11px;
  color: $color-text-disabled;
}

.rec-evidence {
  font-size: 12px;
  color: $color-text-secondary;
  margin: $spacing-xs 0;
  line-height: 1.5;
}

.apply-btn {
  padding: 2px 8px;
  border: 1px solid $color-border;
  border-radius: $radius-sm;
  background: $color-bg;
  font-size: 11px;
  cursor: pointer;
  font-family: $font-family;

  &:hover {
    background: $color-panel;
  }
}

.relations {
  display: flex;
  flex-direction: column;
}

.relation-item {
  padding: $spacing-md 0;
  border-bottom: 1px solid $color-border;

  &:last-child {
    border-bottom: none;
  }
}

.relation-main {
  display: flex;
  align-items: center;
  gap: $spacing-sm;
}

.relation-type {
  font-size: 12px;
  font-weight: 500;
  white-space: nowrap;
}

.relation-target {
  font-size: 12px;
  color: $color-text-primary;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.relation-evidence {
  font-size: 12px;
  color: $color-text-secondary;
  margin-top: $spacing-xs;
  line-height: 1.5;
}

.delete-btn {
  margin-top: $spacing-xs;
  padding: 2px 8px;
  border: none;
  background: none;
  color: $color-text-disabled;
  font-size: 11px;
  cursor: pointer;
  font-family: $font-family;

  &:hover {
    color: $color-text-primary;
  }
}

.empty-relations {
  font-size: 13px;
  color: $color-text-disabled;
  text-align: center;
  padding: $spacing-lg;
}

.analyzing-indicator {
  display: flex;
  align-items: center;
  gap: $spacing-sm;
  padding: $spacing-md;
  font-size: 12px;
  color: $color-text-secondary;
}

.spinner {
  width: 14px;
  height: 14px;
  border: 2px solid $color-border;
  border-top-color: $color-text-primary;
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}
</style>
