<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import { useGraphStore } from '@/stores/graphStore';

const { t } = useI18n();
const graphStore = useGraphStore();

const emit = defineEmits<{
  layout: [];
  exportPng: [];
  exportSvg: [];
}>();
</script>

<template>
  <div class="graph-toolbar">
    <div class="toolbar-left">
      <span class="toolbar-title">{{ t('graph.title') }}</span>
      <span class="toolbar-info">
        {{ t('graph.relationCount', { count: graphStore.relations.length }) }}
      </span>
    </div>
    <div class="toolbar-right">
      <button class="toolbar-btn" @click="emit('layout')" :title="t('graph.relayout')">
        <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
          <circle cx="4" cy="4" r="2" stroke="currentColor" stroke-width="1.2" fill="none"/>
          <circle cx="10" cy="4" r="2" stroke="currentColor" stroke-width="1.2" fill="none"/>
          <circle cx="7" cy="10" r="2" stroke="currentColor" stroke-width="1.2" fill="none"/>
          <line x1="5.5" y1="5" x2="5.5" y2="8.5" stroke="currentColor" stroke-width="1"/>
          <line x1="8.5" y1="5" x2="8.5" y2="8.5" stroke="currentColor" stroke-width="1"/>
        </svg>
      </button>
      <button
        class="toolbar-btn"
        :class="{ active: graphStore.graphLayout.locked }"
        @click="graphStore.graphLayout.locked = !graphStore.graphLayout.locked; graphStore.saveLayout()"
        :title="graphStore.graphLayout.locked ? t('graph.unlockLayout') : t('graph.lockLayout')"
      >
        <svg v-if="graphStore.graphLayout.locked" width="14" height="14" viewBox="0 0 14 14" fill="none">
          <rect x="3" y="6" width="8" height="6" rx="1" stroke="currentColor" stroke-width="1.2" fill="none"/>
          <path d="M5 6V4a2 2 0 014 0v2" stroke="currentColor" stroke-width="1.2" fill="none"/>
        </svg>
        <svg v-else width="14" height="14" viewBox="0 0 14 14" fill="none">
          <rect x="3" y="6" width="8" height="6" rx="1" stroke="currentColor" stroke-width="1.2" fill="none"/>
          <path d="M5 6V4a2 2 0 014 0" stroke="currentColor" stroke-width="1.2" fill="none"/>
        </svg>
      </button>
      <button class="toolbar-btn" @click="emit('exportPng')" :title="t('graph.exportPng')">
        <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
          <path d="M2 10v2h10v-2M7 2v7M4 6l3 3 3-3" stroke="currentColor" stroke-width="1.2"/>
        </svg>
      </button>
      <button class="toolbar-btn" @click="emit('exportSvg')" title="SVG">
        <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
          <text x="3" y="11" font-size="9" font-weight="600" fill="currentColor" font-family="sans-serif">S</text>
        </svg>
      </button>
    </div>
  </div>
</template>

<style lang="scss" scoped>
.graph-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 $spacing-md;
  height: 48px;
  border-bottom: 1px solid $color-border;
  flex-shrink: 0;
}

.toolbar-left {
  display: flex;
  align-items: center;
  gap: $spacing-md;
}

.toolbar-title {
  font-size: 14px;
  font-weight: 500;
}

.toolbar-info {
  color: $color-text-disabled;
  font-size: 12px;
}

.toolbar-right {
  display: flex;
  align-items: center;
  gap: $spacing-xs;
}

.toolbar-btn {
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
    background: $color-panel;
    color: $color-text-primary;
    border-color: $color-node-border;
  }

  &.active {
    background: $color-panel;
    color: $color-text-primary;
    border-color: $color-node-border;
  }
}
</style>
