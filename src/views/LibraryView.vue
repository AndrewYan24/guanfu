<script setup lang="ts">
import { ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { open } from '@tauri-apps/plugin-dialog';
import { useProjectStore } from '@/stores/projectStore';
import { usePaperStore } from '@/stores/paperStore';
import { useGraphStore } from '@/stores/graphStore';
import { readPdfFile } from '@/api/paperApi';
import PaperList from '@/components/papers/PaperList.vue';
import PdfReader from '@/components/pdf/PdfReader.vue';
import MetadataEditor from '@/components/papers/MetadataEditor.vue';
import RelationList from '@/components/papers/RelationList.vue';

const { t } = useI18n();
const projectStore = useProjectStore();
const paperStore = usePaperStore();
const graphStore = useGraphStore();
const isEditingName = ref(false);
const editingName = ref('');
const nameInputRef = ref<HTMLInputElement | null>(null);
const isDragging = ref(false);
const pendingImportPaths = ref<string[]>([]);
let notesSaveTimer: ReturnType<typeof setTimeout> | null = null;

function startEditName() {
  editingName.value = projectStore.currentProject?.name || '';
  isEditingName.value = true;
  setTimeout(() => {
    nameInputRef.value?.focus();
    nameInputRef.value?.select();
  }, 0);
}

async function confirmEditName() {
  const name = editingName.value.trim();
  isEditingName.value = false;
  if (name && name !== projectStore.currentProject?.name) {
    await projectStore.renameProject(name);
  }
}

function cancelEditName() {
  isEditingName.value = false;
}

type TabKey = 'pdf' | 'metadata' | 'relations' | 'notes';
const activeTab = ref<TabKey>('pdf');

const pdfData = ref<Uint8Array | null>(null);
const pdfLoading = ref(false);
const pdfError = ref('');

async function loadPdfData() {
  pdfError.value = '';

  const paper = paperStore.selectedPaper;
  const projectPath = projectStore.projectPath;

  if (!paper || !projectPath) {
    pdfData.value = null;
    return;
  }

  pdfLoading.value = true;
  try {
    const base64 = await readPdfFile(projectPath, paper.id);
    const binary = atob(base64);
    const bytes = new Uint8Array(binary.length);
    for (let i = 0; i < binary.length; i++) bytes[i] = binary.charCodeAt(i);
    pdfData.value = bytes;
  } catch (e) {
    pdfData.value = null;
    pdfError.value = e instanceof Error ? e.message : t('common.saveFailed');
  } finally {
    pdfLoading.value = false;
  }
}

// Load PDF when paper changes (also fires on mount if paper already selected)
// PdfReader handles scroll save/restore internally via its own scroll handler
watch(
  () => paperStore.selectedPaper?.id,
  () => {
    loadPdfData();
  },
  { immediate: true }
);

// Reload PDF when re-entering the PDF tab
watch(activeTab, (tab) => {
  if (tab === 'pdf' && paperStore.selectedPaper) {
    pdfData.value = null;
    loadPdfData();
  }
});

async function handleOpenProject() {
  const dir = await open({
    directory: true,
    multiple: false,
    title: t('library.openProject'),
  });
  if (!dir) return;
  try {
    const project = await projectStore.openProject(dir);
    if (project) {
      paperStore.loadFromProject(project.papers);
      graphStore.loadFromProject(project.relations, project.graphLayout);
    }
  } catch {
    // error handled by store
  }
}

function handleNewProject() {
  projectStore.showCreateDialog = true;
}

async function handleImportFiles(paths: string[]) {
  await paperStore.importPdfs(paths);
}

async function openImportDialog() {
  const selected = await open({
    multiple: true,
    filters: [{ name: 'PDF', extensions: ['pdf'] }],
  });
  if (selected) {
    const paths = Array.isArray(selected) ? selected : [selected];
    if (paths.length) {
      handleImportFiles(paths);
    }
  }
}

function handleNotesInput() {
  if (notesSaveTimer) clearTimeout(notesSaveTimer);
  notesSaveTimer = setTimeout(() => {
    if (paperStore.selectedPaper) {
      paperStore.updatePaper(paperStore.selectedPaper);
    }
  }, 1000);
}

function handleDragOver(e: DragEvent) {
  e.preventDefault();
  isDragging.value = true;
}

function handleDragLeave() {
  isDragging.value = false;
}

function handleDrop(e: DragEvent) {
  e.preventDefault();
  isDragging.value = false;
  const files = e.dataTransfer?.files;
  if (!files || files.length === 0) return;
  const paths: string[] = [];
  for (let i = 0; i < files.length; i++) {
    const f = files[i];
    if (f.name.toLowerCase().endsWith('.pdf')) {
      paths.push((f as any).path || f.name);
    }
  }
  if (paths.length > 0) {
    handleImportFiles(paths);
  }
}

function handleDropEmpty(e: DragEvent) {
  e.preventDefault();
  isDragging.value = false;
  const files = e.dataTransfer?.files;
  if (!files || files.length === 0) return;
  const paths: string[] = [];
  for (let i = 0; i < files.length; i++) {
    const f = files[i];
    if (f.name.toLowerCase().endsWith('.pdf')) {
      // Tauri adds path to File objects
      paths.push((f as any).path || f.name);
    }
  }
  if (paths.length === 0) return;
  pendingImportPaths.value = paths;
  projectStore.showCreateDialog = true;
}

// Auto-import pending files after project is created
watch(() => projectStore.hasProject, async (hasProject) => {
  if (hasProject && pendingImportPaths.value.length > 0) {
    const paths = [...pendingImportPaths.value];
    pendingImportPaths.value = [];
    await handleImportFiles(paths);
  }
});
</script>

<template>
  <div class="library-view">
    <!-- No project state -->
    <div
      v-if="!projectStore.hasProject"
      class="empty-state"
      :class="{ 'drag-over': isDragging }"
      @dragover="handleDragOver"
      @dragleave="handleDragLeave"
      @drop="handleDropEmpty"
    >
      <div class="empty-content">
        <h2 class="empty-title">观复</h2>
        <p class="empty-desc">{{ t('common.appSubtitle') }}</p>
        <div class="empty-actions">
          <button class="btn-primary" @click="handleNewProject">
            {{ t('library.createProject') }}
          </button>
          <button class="btn-secondary" @click="handleOpenProject">
            {{ t('library.openProject') }}
          </button>
        </div>
      </div>
    </div>

    <!-- Has project state -->
    <div v-else class="library-layout">
      <div
        class="library-sidebar"
        :class="{ 'drag-over': isDragging }"
        @dragover="handleDragOver"
        @dragleave="handleDragLeave"
        @drop="handleDrop"
      >
        <div class="library-header">
          <div class="header-top">
            <div v-if="isEditingName" class="name-edit-wrapper">
              <input
                ref="nameInputRef"
                v-model="editingName"
                class="name-edit-input"
                @blur="confirmEditName"
                @keyup.enter="confirmEditName"
                @keyup.escape="cancelEditName"
              />
            </div>
            <template v-else>
              <h3 class="project-name" @click="startEditName">
                {{ projectStore.currentProject?.name }}
              </h3>
              <span v-if="paperStore.papers.length > 0" class="paper-count">
                {{ paperStore.papers.length }}
              </span>
            </template>
            <button class="import-btn" @click="openImportDialog" :title="t('library.importPdfs')">
              <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
                <line x1="7" y1="2" x2="7" y2="12" stroke="currentColor" stroke-width="1.4" stroke-linecap="round"/>
                <line x1="2" y1="7" x2="12" y2="7" stroke="currentColor" stroke-width="1.4" stroke-linecap="round"/>
              </svg>
            </button>
          </div>
        </div>
        <PaperList />
      </div>
      <div class="library-main">
        <div v-if="!paperStore.selectedPaper" class="placeholder">
          <p>{{ t('library.selectPaper') }}</p>
        </div>
        <div v-else class="paper-workspace">
          <div class="paper-header">
            <h2 class="paper-title">{{ paperStore.selectedPaper.title }}</h2>
            <span class="paper-authors">
              {{ paperStore.selectedPaper.authors.join(', ') }}
              <template v-if="paperStore.selectedPaper.year">
                ({{ paperStore.selectedPaper.year }})
              </template>
            </span>
          </div>
          <div class="tab-bar">
            <button
              class="tab"
              :class="{ active: activeTab === 'pdf' }"
              @click="activeTab = 'pdf'"
            >
              {{ t('library.tabPdf') }}
            </button>
            <button
              class="tab"
              :class="{ active: activeTab === 'metadata' }"
              @click="activeTab = 'metadata'"
            >
              {{ t('library.tabMetadata') }}
            </button>
            <button
              class="tab"
              :class="{ active: activeTab === 'notes' }"
              @click="activeTab = 'notes'"
            >
              {{ t('library.tabNotes') }}
            </button>
            <button
              class="tab"
              :class="{ active: activeTab === 'relations' }"
              @click="activeTab = 'relations'"
            >
              {{ t('library.tabRelations') }}
            </button>
          </div>
          <div class="tab-content">
            <div v-if="activeTab === 'pdf'" class="pdf-view">
              <div v-if="pdfLoading" class="placeholder-inner">{{ t('pdf.loading') }}</div>
              <div v-else-if="pdfError" class="placeholder-inner pdf-error">{{ pdfError }}</div>
              <PdfReader
                v-else
                :data="pdfData"
              />
            </div>
            <div v-else-if="activeTab === 'metadata'" class="metadata-view">
              <MetadataEditor :paper="paperStore.selectedPaper" />
            </div>
            <div v-else-if="activeTab === 'notes'" class="notes-view">
              <textarea
                v-model="paperStore.selectedPaper.notes"
                class="notes-textarea"
                :placeholder="t('metadata.notes') + '...'"
                @input="handleNotesInput"
                @blur="paperStore.updatePaper(paperStore.selectedPaper)"
              />
            </div>
            <div v-else-if="activeTab === 'relations'" class="relations-view">
              <RelationList :paper="paperStore.selectedPaper" />
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style lang="scss" scoped>
.library-view {
  height: 100%;
}

.empty-state {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 100%;
  position: relative;

  &.drag-over {
    &::after {
      content: '';
      position: absolute;
      inset: 16px;
      border: 1px dashed $color-text-secondary;
      border-radius: $radius-md;
      pointer-events: none;
      z-index: 10;
    }
  }
}

.empty-content {
  text-align: center;
}

.empty-title {
  font-size: 28px;
  font-weight: 600;
  margin-bottom: $spacing-sm;
}

.empty-desc {
  color: $color-text-secondary;
  margin-bottom: $spacing-xl;
}

.empty-actions {
  display: flex;
  gap: $spacing-md;
  justify-content: center;
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

.library-layout {
  display: flex;
  height: 100%;
}

.library-sidebar {
  width: 280px;
  min-width: 280px;
  border-right: 1px solid $color-border;
  display: flex;
  flex-direction: column;
  height: 100%;
  position: relative;

  &.drag-over {
    &::after {
      content: '';
      position: absolute;
      inset: 0;
      background: rgba(0, 0, 0, 0.03);
      border: 1px dashed $color-text-secondary;
      border-right: none;
      pointer-events: none;
      z-index: 10;
    }
  }
}

.library-header {
  height: 48px;
  padding: 0 $spacing-lg;
  display: flex;
  align-items: center;
  border-bottom: 1px solid $color-border;
  flex-shrink: 0;
}

.header-top {
  display: flex;
  align-items: center;
  gap: $spacing-sm;
  width: 100%;
}

.project-name {
  font-size: 14px;
  font-weight: 500;
  cursor: pointer;
  margin: 0;
  border-radius: $radius-sm;
  transition: background $transition-fast;
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;

  &:hover {
    background: $color-panel;
  }
}

.import-btn {
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  border: 1px solid $color-border;
  border-radius: $radius-sm;
  background: $color-bg;
  color: $color-text-secondary;
  cursor: pointer;
  transition: all $transition-fast;

  &:hover {
    border-color: $color-text-secondary;
    color: $color-text-primary;
    background: $color-panel;
  }
}

.paper-count {
  flex-shrink: 0;
  font-size: 11px;
  color: $color-text-disabled;
  line-height: 1;
}

.name-edit-wrapper {
  flex: 1;
}

.name-edit-input {
  width: 100%;
  padding: 2px 4px;
  margin: -2px -4px;
  border: none;
  border-bottom: 1px solid $color-border;
  border-radius: 0;
  font-size: 14px;
  font-weight: 500;
  font-family: $font-family;
  background: transparent;
  color: $color-text-primary;

  &:focus {
    outline: none;
    border-bottom-color: $color-text-primary;
  }
}

.btn-text {
  border: none;
  background: none;
  color: $color-text-secondary;
  font-size: 12px;
  cursor: pointer;
  font-family: $font-family;
  padding: 2px 6px;
  border-radius: $radius-sm;

  &:hover {
    color: $color-text-primary;
    background: $color-panel;
  }
}

.library-main {
  flex: 1;
  overflow: hidden;
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

.paper-workspace {
  display: flex;
  flex-direction: column;
  height: 100%;
}

.paper-header {
  padding: $spacing-lg;
  border-bottom: 1px solid $color-border;
  flex-shrink: 0;
}

.paper-title {
  font-size: 16px;
  font-weight: 600;
  margin-bottom: 2px;
  line-height: 1.3;
}

.paper-authors {
  font-size: 12px;
  color: $color-text-secondary;
}

.tab-bar {
  display: flex;
  border-bottom: 1px solid $color-border;
  flex-shrink: 0;
}

.tab {
  padding: $spacing-sm $spacing-lg;
  border: none;
  background: none;
  color: $color-text-secondary;
  font-size: 13px;
  cursor: pointer;
  border-bottom: 2px solid transparent;
  font-family: $font-family;
  transition: all $transition-fast;

  &:hover {
    color: $color-text-primary;
  }

  &.active {
    color: $color-text-primary;
    border-bottom-color: $color-text-primary;
  }
}

.tab-content {
  flex: 1;
  overflow: hidden;
}

.pdf-view {
  height: 100%;
}

.metadata-view {
  height: 100%;
  overflow-y: auto;
  padding: $spacing-lg;
}

.notes-view {
  height: 100%;
  padding: $spacing-lg;
}

.relations-view {
  height: 100%;
  overflow-y: auto;
  padding: $spacing-lg;
}

.notes-textarea {
  width: 100%;
  height: 100%;
  border: 1px solid $color-border;
  border-radius: $radius-sm;
  padding: $spacing-md;
  font-size: 13px;
  font-family: $font-family;
  line-height: 1.7;
  resize: none;
  overflow-y: auto;
  background: $color-bg;
  color: $color-text-primary;
  transition: border-color $transition-fast, background $transition-fast, color $transition-fast;

  &:focus {
    outline: none;
    border-color: $color-node-border;
  }

  &::placeholder {
    color: $color-text-disabled;
  }
}

.placeholder-inner {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 100%;
  color: $color-text-disabled;
}
</style>
