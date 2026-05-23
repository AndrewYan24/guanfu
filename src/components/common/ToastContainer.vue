<script setup lang="ts">
import { useToast } from '@/composables/useToast';

const { toasts } = useToast();
</script>

<template>
  <Teleport to="body">
    <div class="toast-container">
      <TransitionGroup name="toast">
        <div
          v-for="toast in toasts"
          :key="toast.id"
          class="toast"
          :class="toast.type"
        >
          {{ toast.message }}
        </div>
      </TransitionGroup>
    </div>
  </Teleport>
</template>

<style lang="scss" scoped>
.toast-container {
  position: fixed;
  bottom: $spacing-xl;
  right: $spacing-xl;
  z-index: 9999;
  display: flex;
  flex-direction: column;
  gap: $spacing-sm;
  pointer-events: none;
}

.toast {
  padding: $spacing-sm $spacing-lg;
  border-radius: $radius-sm;
  font-size: 12px;
  font-family: $font-family;
  pointer-events: auto;
  box-shadow: $shadow-md;

  &.success {
    background: $color-text-primary;
    color: $color-bg;
  }

  &.error {
    background: var(--color-error);
    color: #fff;
  }
}

.toast-enter-active {
  transition: all 0.2s ease;
}

.toast-leave-active {
  transition: all 0.2s ease;
}

.toast-enter-from {
  opacity: 0;
  transform: translateY(8px);
}

.toast-leave-to {
  opacity: 0;
  transform: translateY(-8px);
}
</style>
