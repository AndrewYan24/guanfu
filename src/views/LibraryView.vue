<script setup lang="ts">
import { ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { open } from '@tauri-apps/plugin-dialog';
import { useProjectStore } from '@/stores/projectStore';
import { usePaperStore } from '@/stores/paperStore';
import { useGraphStore } from '@/stores/graphStore';
import { readPdfFile } from '@/api/paperApi';
import PaperList from '@/components/papers/PaperList.vue';
import PaperImportDropzone from '@/components/papers/PaperImportDropzone.vue';
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
    const bytes = await readPdfFile(projectPath, paper.id);
    pdfData.value = new Uint8Array(bytes);
  } catch (e) {
    pdfData.value = null;
    pdfError.value = e instanceof Error ? e.message : '无法读取 PDF 文件';
  } finally {
    pdfLoading.value = false;
  }
}

// Load PDF when paper changes (also fires on mount if paper already selected)
watch(
  () => paperStore.selectedPaper?.id,
  (newId, oldId) => {
    if (oldId && oldId !== newId) {
      // Save scroll before PdfReader is destroyed
      const el = document.querySelector('.pdf-container') as HTMLElement | null;
      if (el) {
        paperStore.savePdfScrollPosition(oldId, el.scrollTop);
      }
    }
    loadPdfData();
  },
  { immediate: true }
);

// Save scroll when leaving PDF tab, reload when entering
watch(activeTab, (tab, oldTab) => {
  if (oldTab === 'pdf') {
    const el = document.querySelector('.pdf-container') as HTMLElement | null;
    if (el && paperStore.selectedPaperId) {
      paperStore.savePdfScrollPosition(paperStore.selectedPaperId, el.scrollTop);
    }
  }
  if (tab === 'pdf') {
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
</script>

<template>
  <div class="library-view">
    <!-- No project state -->
    <div v-if="!projectStore.hasProject" class="empty-state">
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
      <div class="library-sidebar">
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
            <h3 v-else class="project-name" @click="startEditName" title="点击修改项目名">
              {{ projectStore.currentProject?.name }}
            </h3>
          </div>
          <PaperImportDropzone @import="handleImportFiles" />
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
              PDF 阅读
            </button>
            <button
              class="tab"
              :class="{ active: activeTab === 'metadata' }"
              @click="activeTab = 'metadata'"
            >
              结构化数据
            </button>
            <button
              class="tab"
              :class="{ active: activeTab === 'notes' }"
              @click="activeTab = 'notes'"
            >
              笔记
            </button>
            <button
              class="tab"
              :class="{ active: activeTab === 'relations' }"
              @click="activeTab = 'relations'"
            >
              关系
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
}

.library-header {
  padding: $spacing-lg;
  border-bottom: 1px solid $color-border;
}

.header-top {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: $spacing-md;
}

.project-name {
  font-size: 14px;
  font-weight: 500;
  cursor: pointer;
  padding: 2px 4px;
  margin: -2px -4px;
  border-radius: $radius-sm;
  transition: background $transition-fast;

  &:hover {
    background: $color-panel;
  }
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
