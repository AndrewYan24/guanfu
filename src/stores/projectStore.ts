import { defineStore } from 'pinia';
import { ref, computed } from 'vue';
import type { Project } from '@/types';
import * as projectApi from '@/api/projectApi';

export const useProjectStore = defineStore('project', () => {
  const currentProject = ref<Project | null>(null);
  const projectPath = ref<string>('');
  const isLoading = ref(false);
  const lastError = ref<string | null>(null);
  const saveStatus = ref<'saved' | 'saving' | 'error'>('saved');

  const hasProject = computed(() => currentProject.value !== null);
  const showCreateDialog = ref(false);

  let saveTimer: ReturnType<typeof setTimeout> | null = null;

  async function createProject(name: string, baseDir: string) {
    isLoading.value = true;
    lastError.value = null;
    try {
      const project = await projectApi.createProject(name, baseDir);
      currentProject.value = project;
      projectPath.value = project.path;
      await projectApi.setRecentProject(project.path);
      return project;
    } catch (e) {
      lastError.value = e instanceof Error ? e.message : String(e);
      throw e;
    } finally {
      isLoading.value = false;
    }
  }

  async function openProject(path: string) {
    isLoading.value = true;
    lastError.value = null;
    try {
      const project = await projectApi.openProject(path);
      currentProject.value = project;
      projectPath.value = project.path;
      await projectApi.setRecentProject(project.path);
      return project;
    } catch (e) {
      lastError.value = e instanceof Error ? e.message : String(e);
      throw e;
    } finally {
      isLoading.value = false;
    }
  }

  async function restoreRecentProject() {
    try {
      const recentPath = await projectApi.getRecentProject();
      if (recentPath) {
        return await openProject(recentPath);
      }
    } catch {
      // No recent project or failed to open — that's fine
    }
    return null;
  }

  async function saveProject() {
    if (!currentProject.value) return;
    saveStatus.value = 'saving';
    try {
      currentProject.value.updatedAt = new Date().toISOString();
      await projectApi.saveProject(currentProject.value);
      saveStatus.value = 'saved';
    } catch {
      saveStatus.value = 'error';
    }
  }

  function scheduleAutoSave() {
    if (saveTimer) clearTimeout(saveTimer);
    saveTimer = setTimeout(() => {
      saveProject();
    }, 800);
  }

  function closeProject() {
    currentProject.value = null;
    projectPath.value = '';
    if (saveTimer) {
      clearTimeout(saveTimer);
      saveTimer = null;
    }
  }

  async function renameProject(newName: string) {
    if (!currentProject.value || !newName.trim()) return;
    currentProject.value.name = newName.trim();
    await saveProject();
  }

  return {
    currentProject,
    projectPath,
    isLoading,
    lastError,
    saveStatus,
    hasProject,
    showCreateDialog,
    createProject,
    openProject,
    restoreRecentProject,
    saveProject,
    scheduleAutoSave,
    closeProject,
    renameProject,
  };
});
