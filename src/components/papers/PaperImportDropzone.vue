<script setup lang="ts">
import { ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { open } from '@tauri-apps/plugin-dialog';

const { t } = useI18n();

const emit = defineEmits<{
  import: [paths: string[]];
}>();

const isDragging = ref(false);
const isImporting = ref(false);

async function openDialog() {
  if (isImporting.value) return;
  isImporting.value = true;
  try {
    const selected = await open({
      multiple: true,
      filters: [{ name: 'PDF', extensions: ['pdf'] }],
    });
    if (selected) {
      const paths = Array.isArray(selected) ? selected : [selected];
      if (paths.length) {
        emit('import', paths);
      }
    }
  } finally {
    isImporting.value = false;
  }
}

function handleDragOver(e: DragEvent) {
  e.preventDefault();
  isDragging.value = true;
}

function handleDragLeave() {
  isDragging.value = false;
}

function handleDrop(e: DragEvent) {
  e.preventDefault();
  isDragging.value = false;
  openDialog();
}
</script>

<template>
  <div
    class="dropzone"
    :class="{ dragging: isDragging }"
    @dragover="handleDragOver"
    @dragleave="handleDragLeave"
    @drop="handleDrop"
    @click="openDialog"
  >
    <span class="dropzone-text">
      {{ isDragging ? t('library.dropPdfs') : isImporting ? t('common.loading') : t('library.importPdfs') }}
    </span>
  </div>
</template>

<style lang="scss" scoped>
.dropzone {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: $spacing-sm $spacing-md;
  border: 1px dashed $color-border;
  border-radius: $radius-sm;
  cursor: pointer;
  transition: all $transition-fast;

  &:hover {
    border-color: $color-node-border;
    background: $color-panel;
  }

  &.dragging {
    border-color: $color-text-primary;
    background: $color-panel;
  }
}

.dropzone-text {
  font-size: 12px;
  color: $color-text-secondary;
}
</style>
