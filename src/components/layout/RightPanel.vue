<script setup lang="ts">
defineProps<{
  open: boolean;
}>();

defineEmits<{
  close: [];
}>();
</script>

<template>
  <aside v-if="open" class="right-panel">
    <div class="panel-header">
      <span class="panel-title">属性</span>
      <button class="close-btn" @click="$emit('close')">
        <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
          <line x1="2" y1="2" x2="12" y2="12" stroke="currentColor" stroke-width="1.5" />
          <line x1="12" y1="2" x2="2" y2="12" stroke="currentColor" stroke-width="1.5" />
        </svg>
      </button>
    </div>
    <div class="panel-content">
      <slot>
        <p class="empty-text">选择节点或文献查看详情</p>
      </slot>
    </div>
  </aside>
</template>

<style lang="scss" scoped>
.right-panel {
  width: $right-panel;
  min-width: $right-panel;
  height: 100%;
  background: $color-bg;
  border-left: 1px solid $color-border;
  display: flex;
  flex-direction: column;
}

.panel-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: $spacing-md;
  height: 48px;
  border-bottom: 1px solid $color-border;
}

.panel-title {
  font-size: 13px;
  font-weight: 500;
  color: $color-text-primary;
}

.close-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 24px;
  height: 24px;
  border: none;
  background: none;
  color: $color-text-secondary;
  cursor: pointer;
  border-radius: $radius-sm;

  &:hover {
    background: $color-panel;
    color: $color-text-primary;
  }
}

.panel-content {
  flex: 1;
  overflow-y: auto;
  padding: $spacing-lg;
}

.empty-text {
  color: $color-text-disabled;
  font-size: 13px;
  text-align: center;
  margin-top: 40px;
}
</style>
