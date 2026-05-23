import { defineStore } from 'pinia';
import { ref } from 'vue';
import type { Insight } from '@/types';
import * as graphApi from '@/api/graphApi';

export const useInsightStore = defineStore('insight', () => {
  const insights = ref<Insight[]>([]);
  const isLoading = ref(false);

  let debounceTimer: ReturnType<typeof setTimeout> | null = null;

  async function loadSaved(projectPath: string) {
    if (!projectPath) return;
    try {
      const saved = await graphApi.loadSavedInsights(projectPath);
      if (saved.length > 0) {
        insights.value = saved;
      }
    } catch {
      // No saved insights, will generate fresh
    }
  }

  async function runAnalysis(projectPath: string) {
    if (!projectPath) return;
    isLoading.value = true;
    try {
      insights.value = await graphApi.runInsightAnalysis(projectPath);
      // Persist to disk
      try {
        await graphApi.saveInsights(projectPath, insights.value);
      } catch {
        // Silent fail on save
      }
    } finally {
      isLoading.value = false;
    }
  }

  function autoRun(projectPath: string) {
    if (!projectPath) return;
    if (debounceTimer) clearTimeout(debounceTimer);
    debounceTimer = setTimeout(() => {
      runAnalysis(projectPath);
    }, 1200);
  }

  function clearInsights() {
    insights.value = [];
  }

  return {
    insights,
    isLoading,
    loadSaved,
    runAnalysis,
    autoRun,
    clearInsights,
  };
});
