<script setup lang="ts">
import { watch, onMounted } from 'vue';
import { useI18n } from 'vue-i18n';
import { useProjectStore } from '@/stores/projectStore';
import { useInsightStore } from '@/stores/insightStore';
import { usePaperStore } from '@/stores/paperStore';
import { useGraphStore } from '@/stores/graphStore';

const { t } = useI18n();
const projectStore = useProjectStore();
const insightStore = useInsightStore();
const paperStore = usePaperStore();
const graphStore = useGraphStore();

function triggerAutoRun() {
  if (projectStore.projectPath) {
    insightStore.autoRun(projectStore.projectPath);
  }
}

// Run on mount if project is open
onMounted(async () => {
  if (projectStore.hasProject && insightStore.insights.length === 0) {
    // Try loading saved insights first
    await insightStore.loadSaved(projectStore.projectPath!);
    // If none saved, generate fresh
    if (insightStore.insights.length === 0) {
      triggerAutoRun();
    }
  }
});

// Watch papers count
watch(() => paperStore.papers.length, triggerAutoRun);

// Watch relations count
watch(() => graphStore.relations.length, triggerAutoRun);

// Watch paper metadata changes (title, updatedAt as proxy)
watch(
  () => paperStore.papers.map(p => `${p.id}:${p.metadata ? 'm' : ''}:${p.updatedAt}`).join(','),
  triggerAutoRun
);

function typeLabel(type: string): string {
  const labels: Record<string, string> = {
    'potential-fault-line': t('insights.potentialFaultLine'),
    'lack-pluralistic-testing': t('insights.lackPluralisticTesting'),
    'method-homogeneity': t('insights.methodHomogeneity'),
    'theoretical-gap': t('insights.theoreticalGap'),
    'creative-opportunity': t('insights.creativeOpportunity'),
  };
  return labels[type] ?? type;
}
</script>

<template>
  <div class="insights-view">
    <div v-if="!projectStore.hasProject" class="placeholder">
      <p>{{ t('insights.noProject') }}</p>
    </div>
    <div v-else class="insights-content">
      <div class="insights-header">
        <h3>{{ t('insights.title') }}</h3>
      </div>

      <div class="insights-list">
        <div v-if="insightStore.isLoading && insightStore.insights.length === 0" class="empty-insights">
          <div class="loading-big">
            <div class="spinner-lg" />
            <p>{{ t('insights.analyzingDetail') }}</p>
          </div>
        </div>
        <div v-else-if="!insightStore.isLoading && insightStore.insights.length === 0" class="empty-insights">
          <p>{{ t('insights.emptyHint') }}</p>
        </div>
        <div
          v-for="insight in insightStore.insights"
          :key="insight.id"
          class="insight-card"
        >
          <span class="insight-type">{{ typeLabel(insight.type) }}</span>
          <h4 class="insight-title">{{ insight.title }}</h4>
          <p class="insight-desc">{{ insight.description }}</p>
        </div>
      </div>
    </div>
  </div>
</template>

<style lang="scss" scoped>
.insights-view {
  height: 100%;
  overflow-y: auto;
}

.placeholder {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 100%;
  color: $color-text-disabled;
}

.insights-content {
  padding: $spacing-xl;
  max-width: 720px;
  margin: 0 auto;
}

.insights-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: $spacing-xl;

  h3 {
    font-size: 16px;
    font-weight: 500;
  }
}

.insights-list {
  display: flex;
  flex-direction: column;
  gap: $spacing-md;
}

.empty-insights {
  text-align: center;
  padding: 60px 0;
  color: $color-text-disabled;
}

.loading-big {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: $spacing-md;

  p {
    font-size: 13px;
    color: $color-text-disabled;
  }
}

.spinner-lg {
  width: 28px;
  height: 28px;
  border: 3px solid $color-border;
  border-top-color: $color-text-primary;
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.insight-card {
  padding: $spacing-lg;
  border: 1px solid $color-border;
  border-radius: $radius-sm;

  &:hover {
    border-color: $color-node-border;
  }
}

.insight-type {
  font-size: 11px;
  color: $color-text-disabled;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.insight-title {
  font-size: 14px;
  font-weight: 500;
  margin: $spacing-sm 0;
}

.insight-desc {
  font-size: 13px;
  color: $color-text-secondary;
  line-height: 1.6;
}
</style>
