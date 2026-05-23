<script setup lang="ts">
defineProps<{
  title?: string;
  description?: string;
}>();
</script>

<template>
  <div class="empty-state">
    <p v-if="title" class="empty-title">{{ title }}</p>
    <p v-if="description" class="empty-desc">{{ description }}</p>
    <slot />
  </div>
</template>

<style lang="scss" scoped>
.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 60px $spacing-xl;
  text-align: center;
}

.empty-title {
  font-size: 14px;
  font-weight: 500;
  color: $color-text-primary;
  margin-bottom: $spacing-sm;
}

.empty-desc {
  font-size: 13px;
  color: $color-text-disabled;
}
</style>
