<script setup lang="ts">
import { ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { open } from '@tauri-apps/plugin-dialog';
import Sidebar from './Sidebar.vue';
import RightPanel from './RightPanel.vue';
import { useProjectStore } from '@/stores/projectStore';
import { usePaperStore } from '@/stores/paperStore';
import { useGraphStore } from '@/stores/graphStore';
import { useSettingsStore } from '@/stores/settingsStore';
import * as projectApi from '@/api/projectApi';

const { t } = useI18n();
const projectStore = useProjectStore();
const paperStore = usePaperStore();
const graphStore = useGraphStore();
const settingsStore = useSettingsStore();
const sidebarExpanded = ref(false);
const rightPanelOpen = ref(false);

const newProjectName = ref('');
const selectedDir = ref('');
const isCreating = ref(false);

watch(() => projectStore.showCreateDialog, async (show) => {
  if (show) {
    newProjectName.value = '';
    isCreating.value = false;
    const customDir = settingsStore.maskedSettings?.defaultProjectDir;
    if (customDir) {
      selectedDir.value = customDir;
    } else {
      try {
        selectedDir.value = await projectApi.getDefaultProjectDir();
      } catch {
        selectedDir.value = '';
      }
    }
  }
});

async function selectDirectory() {
  const dir = await open({
    directory: true,
    multiple: false,
    title: t('library.projectLocation'),
  });
  if (dir) {
    selectedDir.value = dir;
  }
}

async function handleCreateProject() {
  if (!newProjectName.value.trim() || !selectedDir.value || isCreating.value) return;
  isCreating.value = true;
  try {
    const project = await projectStore.createProject(newProjectName.value.trim(), selectedDir.value);
    if (project) {
      paperStore.loadFromProject(project.papers);
      graphStore.loadFromProject(project.relations, project.graphLayout);
    }
    projectStore.showCreateDialog = false;
  } finally {
    isCreating.value = false;
  }
}

function closeDialog() {
  projectStore.showCreateDialog = false;
}
</script>

<template>
  <div class="app-shell">
    <Sidebar
      :expanded="sidebarExpanded"
      @toggle="sidebarExpanded = !sidebarExpanded"
    />
    <main class="main-workspace">
      <div v-if="projectStore.isLoading" class="loading-overlay">
        <span class="loading-text">{{ t('common.loading') }}</span>
      </div>
      <router-view v-else />
    </main>
    <RightPanel :open="rightPanelOpen" @close="rightPanelOpen = false" />

    <!-- Global create project dialog -->
    <div v-if="projectStore.showCreateDialog" class="dialog-overlay" @click.self="closeDialog">
      <div class="dialog">
        <h3 class="dialog-title">{{ t('library.createProject') }}</h3>
        <input
          v-model="newProjectName"
          class="dialog-input"
          :placeholder="t('library.projectName')"
          @keyup.enter="handleCreateProject"
          autofocus
        />
        <div class="dir-picker">
          <button class="btn-secondary btn-sm" @click="selectDirectory">
            {{ t('library.projectLocation') }}
          </button>
          <span v-if="selectedDir" class="dir-path" :title="selectedDir">
            {{ selectedDir }}
          </span>
          <span v-else class="dir-placeholder">{{ t('settings.projectDirDefault') }}</span>
        </div>
        <div class="dialog-actions">
          <button class="btn-secondary" @click="closeDialog">{{ t('common.cancel') }}</button>
          <button
            class="btn-primary"
            :disabled="!newProjectName.trim() || !selectedDir || isCreating"
            @click="handleCreateProject"
          >
            {{ isCreating ? t('common.saving') : t('common.confirm') }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<style lang="scss" scoped>
.app-shell {
  display: flex;
  height: 100vh;
  overflow: hidden;
}

.main-workspace {
  flex: 1;
  overflow: hidden;
  position: relative;
}

.loading-overlay {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 100%;
}

.loading-text {
  color: $color-text-disabled;
  font-size: 14px;
}

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
  width: 360px;
  box-shadow: $shadow-md;
}

.dialog-title {
  font-size: 16px;
  font-weight: 500;
  margin-bottom: $spacing-lg;
}

.dialog-input {
  width: 100%;
  padding: $spacing-sm $spacing-md;
  border: 1px solid $color-border;
  border-radius: $radius-sm;
  font-size: 13px;
  font-family: $font-family;
  color: $color-text-primary;
  background: $color-bg;
  margin-bottom: $spacing-lg;

  &:focus {
    outline: none;
    border-color: $color-node-border;
  }

  &::placeholder {
    color: $color-text-disabled;
  }
}

.dir-picker {
  display: flex;
  align-items: center;
  gap: $spacing-sm;
  margin-bottom: $spacing-lg;
}

.dir-path {
  font-size: 12px;
  color: $color-text-secondary;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.dir-placeholder {
  font-size: 12px;
  color: $color-text-disabled;
}

.dialog-actions {
  display: flex;
  gap: $spacing-sm;
  justify-content: flex-end;
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

.btn-sm {
  padding: 4px 10px;
  font-size: 12px;
  flex-shrink: 0;
}
</style>
