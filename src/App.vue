<script setup lang="ts">
import { onMounted } from 'vue';
import AppShell from '@/components/layout/AppShell.vue';
import ToastContainer from '@/components/common/ToastContainer.vue';
import { useProjectStore } from '@/stores/projectStore';
import { usePaperStore } from '@/stores/paperStore';
import { useGraphStore } from '@/stores/graphStore';
import { useSettingsStore } from '@/stores/settingsStore';
import { useTheme } from '@/composables/useTheme';

const projectStore = useProjectStore();
const paperStore = usePaperStore();
const graphStore = useGraphStore();
const settingsStore = useSettingsStore();

// Ensure theme is applied on startup, not just when settings page loads
useTheme();

onMounted(async () => {
  // Pre-load settings so SettingsView doesn't re-fetch
  settingsStore.loadSettings();

  const project = await projectStore.restoreRecentProject();
  if (project) {
    paperStore.loadFromProject(project.papers);
    graphStore.loadFromProject(project.relations, project.graphLayout);
  }
});
</script>

<template>
  <AppShell />
  <ToastContainer />
</template>

<style lang="scss">
html,
body,
#app {
  height: 100%;
  margin: 0;
  padding: 0;
}
</style>
