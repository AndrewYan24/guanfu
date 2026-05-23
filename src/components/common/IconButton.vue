<script setup lang="ts">
defineProps<{
  title?: string;
  active?: boolean;
}>();

defineEmits<{
  click: [];
}>();
</script>

<template>
  <button
    class="icon-button"
    :class="{ active }"
    :title="title"
    @click="$emit('click')"
  >
    <slot />
  </button>
</template>

<style lang="scss" scoped>
.icon-button {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  border: none;
  background: none;
  color: $color-text-secondary;
  cursor: pointer;
  border-radius: $radius-sm;
  transition: all $transition-fast;

  &:hover {
    background: $color-panel;
    color: $color-text-primary;
  }

  &.active {
    background: $color-panel;
    color: $color-text-primary;
  }
}
</style>
