<script setup lang="ts">
import { ref, computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { useProjectStore } from '@/stores/projectStore';
import { useGraphStore } from '@/stores/graphStore';
import { usePaperStore } from '@/stores/paperStore';
import { relationTypes } from '@/types/relation';
import type { RelationType } from '@/types/relation';
import type { Relation } from '@/types';
import GraphCanvas from '@/components/graph/GraphCanvas.vue';
import GraphToolbar from '@/components/graph/GraphToolbar.vue';

const { t } = useI18n();
const projectStore = useProjectStore();
const graphStore = useGraphStore();
const paperStore = usePaperStore();

const graphCanvasRef = ref<InstanceType<typeof GraphCanvas>>();
const selectedNodeId = ref('');
const selectedEdgeId = ref('');

// --- Add relation dialog ---
const showAddDialog = ref(false);
const newSourceId = ref('');
const newTargetId = ref('');
const newType = ref<RelationType>('supports');
const newEvidence = ref('');
const sourceSearch = ref('');
const targetSearch = ref('');

const filteredSourcePapers = computed(() => {
  const q = sourceSearch.value.toLowerCase();
  if (!q) return paperStore.papers;
  return paperStore.papers.filter((p) =>
    p.title.toLowerCase().includes(q) || p.authors.some((a) => a.toLowerCase().includes(q))
  );
});

const filteredTargetPapers = computed(() => {
  const q = targetSearch.value.toLowerCase();
  if (!q) return paperStore.papers;
  return paperStore.papers.filter((p) =>
    p.title.toLowerCase().includes(q) || p.authors.some((a) => a.toLowerCase().includes(q))
  );
});

function openAddDialog() {
  newSourceId.value = selectedNodeId.value || '';
  newTargetId.value = '';
  newType.value = 'supports';
  newEvidence.value = '';
  sourceSearch.value = '';
  targetSearch.value = '';
  showAddDialog.value = true;
}

function closeAddDialog() {
  showAddDialog.value = false;
}

async function handleAddRelation() {
  if (!newSourceId.value || !newTargetId.value || newSourceId.value === newTargetId.value) return;
  const relation: Relation = {
    id: crypto.randomUUID(),
    sourceId: newSourceId.value,
    targetId: newTargetId.value,
    type: newType.value,
    evidence: newEvidence.value,
    isManual: true,
    createdAt: new Date().toISOString(),
    updatedAt: new Date().toISOString(),
  };
  await graphStore.addRelation(relation);
  showAddDialog.value = false;
}

// --- Node/edge selection ---
function handleNodeClick(id: string) {
  selectedNodeId.value = id;
  selectedEdgeId.value = '';
  if (id) {
    paperStore.selectPaper(id);
    graphStore.selectRelation(null);
  }
}

function handleEdgeClick(id: string) {
  selectedEdgeId.value = id;
  selectedNodeId.value = '';
  graphStore.selectRelation(id);
}

function handleLayout() {
  graphCanvasRef.value?.runLayout();
}

function handleExportPng() {
  graphCanvasRef.value?.exportPng();
}

function handleExportSvg() {
  graphCanvasRef.value?.exportSvg();
}

function getSelectedRelation() {
  return graphStore.relations.find((r) => r.id === selectedEdgeId.value);
}

function getSelectedNode() {
  return paperStore.papers.find((p) => p.id === selectedNodeId.value);
}

function getPaperTitle(id: string) {
  return paperStore.papers.find((p) => p.id === id)?.title ?? '未知文献';
}

function getPaperAuthor(id: string) {
  const paper = paperStore.papers.find((p) => p.id === id);
  if (!paper) return '';
  return paper.authors[0] ?? '';
}

function getRelationLabel(type: string) {
  return t('relations.' + type);
}

function closeDetail() {
  selectedEdgeId.value = '';
  selectedNodeId.value = '';
  graphStore.selectRelation(null);
}

async function handleDeleteRelation(id: string) {
  await graphStore.deleteRelation(id);
  selectedEdgeId.value = '';
}

async function handleDeletePaper(id: string) {
  if (!window.confirm(t('library.deleteConfirm'))) return;
  await paperStore.deletePaper(id);
  selectedNodeId.value = '';
}
</script>

<template>
  <div class="graph-view">
    <div v-if="!projectStore.hasProject" class="placeholder">
      <p>{{ t('graph.noProject') }}</p>
    </div>
    <div v-else class="graph-container">
      <GraphToolbar
        @layout="handleLayout"
        @export-png="handleExportPng"
        @export-svg="handleExportSvg"
      />
      <div class="graph-body">
        <GraphCanvas
          ref="graphCanvasRef"
          @node-click="handleNodeClick"
          @edge-click="handleEdgeClick"
        />

        <!-- Add relation FAB -->
        <button class="add-relation-btn" @click="openAddDialog" :title="t('graph.addRelation')">
          <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
            <line x1="8" y1="3" x2="8" y2="13" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
            <line x1="3" y1="8" x2="13" y2="8" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
          </svg>
        </button>

        <!-- Node detail panel -->
        <div v-if="selectedNodeId && getSelectedNode()" class="detail-panel">
          <div class="detail-header">
            <h4 class="detail-title">{{ getSelectedNode()!.title }}</h4>
            <button class="detail-close" @click="closeDetail">×</button>
          </div>
          <p class="detail-meta">
            {{ getSelectedNode()!.authors.join(', ') }}
            <template v-if="getSelectedNode()!.year">
              ({{ getSelectedNode()!.year }})
            </template>
          </p>
          <p v-if="getSelectedNode()!.metadata?.coreClaim" class="detail-claim">
            {{ getSelectedNode()!.metadata!.coreClaim }}
          </p>
          <div class="detail-actions">
            <button class="detail-action-btn" @click="openAddDialog">
              {{ t('graph.addRelation') }}
            </button>
            <button class="detail-action-btn danger" @click="handleDeletePaper(selectedNodeId)">
              {{ t('library.deletePaper') }}
            </button>
          </div>
        </div>

        <!-- Edge detail panel -->
        <div v-if="selectedEdgeId && getSelectedRelation()" class="detail-panel edge-detail-panel">
          <div class="detail-header">
            <span class="edge-type-badge" :class="getSelectedRelation()!.type">
              {{ getRelationLabel(getSelectedRelation()!.type) }}
            </span>
            <button class="detail-close" @click="closeDetail">×</button>
          </div>
          <div class="edge-papers">
            <div class="edge-paper source-paper">
              <span class="edge-paper-label">{{ t('graph.sourcePaper') }}</span>
              <span class="edge-paper-title">{{ getPaperTitle(getSelectedRelation()!.sourceId) }}</span>
              <span class="edge-paper-author">{{ getPaperAuthor(getSelectedRelation()!.sourceId) }}</span>
            </div>
            <div class="edge-arrow">→</div>
            <div class="edge-paper target-paper">
              <span class="edge-paper-label">{{ t('graph.targetPaper') }}</span>
              <span class="edge-paper-title">{{ getPaperTitle(getSelectedRelation()!.targetId) }}</span>
              <span class="edge-paper-author">{{ getPaperAuthor(getSelectedRelation()!.targetId) }}</span>
            </div>
          </div>
          <div v-if="getSelectedRelation()!.confidence" class="edge-confidence">
            <span class="confidence-label">{{ t('graph.confidence') }}</span>
            <div class="confidence-bar">
              <div
                class="confidence-fill"
                :style="{ width: (getSelectedRelation()!.confidence! * 100) + '%' }"
              />
            </div>
            <span class="confidence-value">{{ Math.round(getSelectedRelation()!.confidence! * 100) }}%</span>
          </div>
          <div class="edge-evidence-section">
            <span class="evidence-label">{{ t('graph.evidence') }}</span>
            <p class="edge-evidence-text">{{ getSelectedRelation()!.evidence }}</p>
          </div>
          <div class="edge-actions">
            <button class="edge-delete-btn" @click="handleDeleteRelation(getSelectedRelation()!.id)">
              {{ t('common.delete') }}
            </button>
          </div>
        </div>
      </div>
    </div>

    <!-- Add relation dialog -->
    <div v-if="showAddDialog" class="dialog-overlay" @click.self="closeAddDialog">
      <div class="dialog">
        <h3 class="dialog-title">{{ t('graph.addRelation') }}</h3>

        <div class="form-field">
          <label>{{ t('graph.sourcePaper') }}</label>
          <div class="select-wrapper">
            <input
              v-model="sourceSearch"
              class="search-input"
              :placeholder="t('graph.sourcePaper') + '...'"
            />
            <div class="select-list">
              <div
                v-for="paper in filteredSourcePapers"
                :key="paper.id"
                class="select-item"
                :class="{ selected: newSourceId === paper.id }"
                @click="newSourceId = paper.id; sourceSearch = ''"
              >
                <span class="select-item-title">{{ paper.title }}</span>
                <span class="select-item-meta">{{ paper.authors[0] || '' }} {{ paper.year || '' }}</span>
              </div>
            </div>
          </div>
        </div>

        <div class="form-field">
          <label>{{ t('graph.targetPaper') }}</label>
          <div class="select-wrapper">
            <input
              v-model="targetSearch"
              class="search-input"
              :placeholder="t('graph.targetPaper') + '...'"
            />
            <div class="select-list">
              <div
                v-for="paper in filteredTargetPapers"
                :key="paper.id"
                class="select-item"
                :class="{ selected: newTargetId === paper.id }"
                @click="newTargetId = paper.id; targetSearch = ''"
              >
                <span class="select-item-title">{{ paper.title }}</span>
                <span class="select-item-meta">{{ paper.authors[0] || '' }} {{ paper.year || '' }}</span>
              </div>
            </div>
          </div>
        </div>

        <div class="form-field">
          <label>{{ t('graph.relationType') }}</label>
          <div class="type-options">
            <label
              v-for="rt in relationTypes"
              :key="rt"
              class="type-option"
              :class="{ active: newType === rt }"
            >
              <input type="radio" :value="rt" v-model="newType" />
              <span>{{ getRelationLabel(rt) }}</span>
            </label>
          </div>
        </div>

        <div class="form-field">
          <label>{{ t('graph.evidence') }}</label>
          <textarea
            v-model="newEvidence"
            class="evidence-input"
            :placeholder="t('graph.evidence') + '...'"
            rows="3"
          />
        </div>

        <div class="dialog-actions">
          <button class="btn-secondary" @click="closeAddDialog">{{ t('common.cancel') }}</button>
          <button
            class="btn-primary"
            :disabled="!newSourceId || !newTargetId || newSourceId === newTargetId"
            @click="handleAddRelation"
          >
            {{ t('common.confirm') }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<style lang="scss" scoped>
.graph-view {
  height: 100%;
  display: flex;
  flex-direction: column;
}

.placeholder {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 100%;
  color: $color-text-disabled;
}

.graph-container {
  flex: 1;
  display: flex;
  flex-direction: column;
}

.graph-body {
  flex: 1;
  display: flex;
  position: relative;
  overflow: hidden;
}

// Add relation FAB
.add-relation-btn {
  position: absolute;
  top: $spacing-lg;
  right: $spacing-lg;
  width: 32px;
  height: 32px;
  border: 1px solid $color-border;
  background: $color-bg;
  color: $color-text-secondary;
  border-radius: $radius-sm;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  box-shadow: $shadow-sm;
  z-index: 10;

  &:hover {
    background: $color-panel;
    color: $color-text-primary;
    border-color: $color-node-border;
  }
}

// Detail panels
.detail-panel {
  position: absolute;
  bottom: $spacing-lg;
  left: $spacing-lg;
  max-width: 380px;
  background: $color-bg;
  border: 1px solid $color-border;
  border-radius: $radius-sm;
  padding: $spacing-md;
  box-shadow: $shadow-sm;
}

.detail-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: $spacing-sm;
  margin-bottom: $spacing-sm;
}

.detail-title {
  font-size: 14px;
  font-weight: 500;
  line-height: 1.4;
}

.detail-close {
  flex-shrink: 0;
  width: 20px;
  height: 20px;
  border: none;
  background: none;
  color: $color-text-disabled;
  cursor: pointer;
  font-size: 16px;
  line-height: 1;
  border-radius: $radius-sm;
  display: flex;
  align-items: center;
  justify-content: center;

  &:hover {
    background: $color-panel;
    color: $color-text-primary;
  }
}

.detail-meta {
  font-size: 12px;
  color: $color-text-secondary;
  margin-bottom: $spacing-sm;
}

.detail-claim {
  font-size: 12px;
  color: $color-text-primary;
  line-height: 1.5;
}

.detail-actions {
  margin-top: $spacing-sm;
  padding-top: $spacing-sm;
  border-top: 1px solid $color-border;
}

.detail-action-btn {
  border: none;
  background: none;
  color: $color-text-secondary;
  font-size: 12px;
  cursor: pointer;
  font-family: $font-family;
  padding: 2px 8px;
  border-radius: $radius-sm;

  &:hover {
    color: $color-text-primary;
    background: $color-panel;
  }

  &.danger:hover {
    color: var(--color-error);
    background: var(--hover-bg);
  }
}

// Edge detail
.edge-detail-panel {
  max-width: 400px;
}

.edge-type-badge {
  font-size: 13px;
  font-weight: 600;
  padding: 2px 10px;
  border-radius: 2px;
  background: $color-panel;

  &.supports { color: var(--color-relation-supports); }
  &.opposes { color: var(--color-relation-opposes); background: var(--hover-bg); }
  &.modifies { color: var(--color-relation-modifies); }
  &.adopts { color: var(--color-relation-adopts); }
  &.reinterprets { color: var(--color-relation-reinterprets); }
}

.edge-papers {
  display: flex;
  align-items: stretch;
  gap: $spacing-sm;
  padding: $spacing-sm 0;
  border-bottom: 1px solid $color-border;
  margin-bottom: $spacing-sm;
}

.edge-paper {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}

.edge-paper-label {
  font-size: 10px;
  color: $color-text-disabled;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.edge-paper-title {
  font-size: 12px;
  color: $color-text-primary;
  line-height: 1.4;
  overflow: hidden;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
}

.edge-paper-author {
  font-size: 11px;
  color: $color-text-disabled;
}

.edge-arrow {
  flex-shrink: 0;
  display: flex;
  align-items: center;
  color: $color-text-disabled;
  font-size: 14px;
  padding-top: 14px;
}

.edge-confidence {
  display: flex;
  align-items: center;
  gap: $spacing-sm;
  margin-bottom: $spacing-sm;
}

.confidence-label {
  font-size: 11px;
  color: $color-text-disabled;
  flex-shrink: 0;
}

.confidence-bar {
  flex: 1;
  height: 4px;
  background: $color-border;
  border-radius: 2px;
  overflow: hidden;
}

.confidence-fill {
  height: 100%;
  background: $color-text-primary;
  border-radius: 2px;
  transition: width 0.3s ease;
}

.confidence-value {
  font-size: 11px;
  color: $color-text-secondary;
  flex-shrink: 0;
  min-width: 28px;
  text-align: right;
}

.edge-evidence-section {
  margin-bottom: $spacing-sm;
}

.evidence-label {
  display: block;
  font-size: 11px;
  color: $color-text-disabled;
  margin-bottom: 4px;
}

.edge-evidence-text {
  font-size: 12px;
  color: $color-text-primary;
  line-height: 1.6;
  margin: 0;
}

.edge-actions {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  padding-top: $spacing-xs;
  border-top: 1px solid $color-border;
}

.edge-delete-btn {
  border: none;
  background: none;
  color: $color-text-disabled;
  font-size: 11px;
  cursor: pointer;
  font-family: $font-family;
  padding: 2px 8px;
  border-radius: $radius-sm;

  &:hover {
    color: var(--color-error);
    background: var(--hover-bg);
  }
}

// Dialog
.dialog-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.3);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 100;
}

.dialog {
  background: $color-bg;
  border-radius: $radius-md;
  padding: $spacing-xl;
  width: 420px;
  max-height: 80vh;
  overflow-y: auto;
  box-shadow: $shadow-md;
}

.dialog-title {
  font-size: 16px;
  font-weight: 500;
  margin-bottom: $spacing-lg;
}

.form-field {
  margin-bottom: $spacing-md;

  label {
    display: block;
    font-size: 12px;
    color: $color-text-secondary;
    margin-bottom: $spacing-xs;
  }
}

.select-wrapper {
  border: 1px solid $color-border;
  border-radius: $radius-sm;
  overflow: hidden;
}

.search-input {
  width: 100%;
  padding: $spacing-sm $spacing-md;
  border: none;
  border-bottom: 1px solid $color-border;
  font-size: 13px;
  font-family: $font-family;
  color: $color-text-primary;
  background: $color-bg;

  &:focus {
    outline: none;
  }

  &::placeholder {
    color: $color-text-disabled;
  }
}

.select-list {
  max-height: 120px;
  overflow-y: auto;
}

.select-item {
  padding: $spacing-xs $spacing-md;
  cursor: pointer;
  display: flex;
  flex-direction: column;
  gap: 1px;

  &:hover {
    background: $color-panel;
  }

  &.selected {
    background: $color-panel;
  }
}

.select-item-title {
  font-size: 12px;
  color: $color-text-primary;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.select-item-meta {
  font-size: 11px;
  color: $color-text-disabled;
}

.type-options {
  display: flex;
  flex-wrap: wrap;
  gap: $spacing-xs;
}

.type-option {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 4px 10px;
  border: 1px solid $color-border;
  border-radius: $radius-sm;
  cursor: pointer;
  font-size: 12px;
  color: $color-text-secondary;
  transition: all $transition-fast;

  input[type="radio"] {
    display: none;
  }

  &:hover {
    border-color: $color-node-border;
    color: $color-text-primary;
  }

  &.active {
    border-color: $color-node-border;
    color: $color-text-primary;
    background: $color-panel;
  }
}

.evidence-input {
  width: 100%;
  padding: $spacing-sm $spacing-md;
  border: 1px solid $color-border;
  border-radius: $radius-sm;
  font-size: 13px;
  font-family: $font-family;
  color: $color-text-primary;
  background: $color-bg;
  resize: vertical;
  line-height: 1.5;

  &:focus {
    outline: none;
    border-color: $color-node-border;
  }

  &::placeholder {
    color: $color-text-disabled;
  }
}

.dialog-actions {
  display: flex;
  gap: $spacing-sm;
  justify-content: flex-end;
  margin-top: $spacing-lg;
}

.btn-primary {
  padding: $spacing-sm $spacing-lg;
  background: $color-text-primary;
  color: $color-bg;
  border: none;
  border-radius: $radius-sm;
  cursor: pointer;
  font-size: 13px;
  font-family: $font-family;

  &:hover:not(:disabled) {
    opacity: 0.85;
  }

  &:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
}

.btn-secondary {
  padding: $spacing-sm $spacing-lg;
  background: $color-bg;
  color: $color-text-primary;
  border: 1px solid $color-border;
  border-radius: $radius-sm;
  cursor: pointer;
  font-size: 13px;
  font-family: $font-family;

  &:hover {
    background: $color-panel;
  }
}
</style>
