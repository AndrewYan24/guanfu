<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue';
import { useI18n } from 'vue-i18n';
import { usePaperStore } from '@/stores/paperStore';

const { t } = useI18n();
const paperStore = usePaperStore();

const papers = computed(() => paperStore.papers);

const contextMenuPaperId = ref<string | null>(null);
const contextMenuX = ref(0);
const contextMenuY = ref(0);

function handleContextMenu(e: MouseEvent, paperId: string) {
  e.preventDefault();
  contextMenuPaperId.value = paperId;
  contextMenuX.value = e.clientX;
  contextMenuY.value = e.clientY;
}

function closeContextMenu() {
  contextMenuPaperId.value = null;
}

async function handleDelete() {
  const id = contextMenuPaperId.value;
  closeContextMenu();
  if (!id) return;
  await paperStore.deletePaper(id);
}

function handleGlobalClick() {
  closeContextMenu();
}

onMounted(() => {
  document.addEventListener('click', handleGlobalClick);
});

onUnmounted(() => {
  document.removeEventListener('click', handleGlobalClick);
});
</script>

<template>
  <div class="paper-list">
    <div v-if="papers.length === 0" class="empty-list">
      <p>{{ t('library.dropPdfs') }}</p>
    </div>
    <div
      v-for="paper in papers"
      :key="paper.id"
      class="paper-item"
      :class="{ selected: paperStore.selectedPaperId === paper.id, parsing: paperStore.isPaperParsing(paper.id) }"
      @click="paperStore.selectPaper(paper.id)"
      @contextmenu="handleContextMenu($event, paper.id)"
    >
      <div class="paper-info">
        <span class="paper-title">
          <span class="paper-title-text">{{ paper.title || t('common.empty') }}</span>
          <span v-if="paperStore.isPaperParsing(paper.id)" class="parse-spinner" />
        </span>
        <span class="paper-meta">
          <template v-if="paperStore.isPaperParsing(paper.id)">
            <span class="parsing-text">{{ t('metadata.discovering') }}</span>
          </template>
          <template v-else-if="paperStore.isPaperQueued(paper.id)">
            <span class="parsing-text">{{ t('metadata.queued') }}</span>
          </template>
          <template v-else>
            {{ paper.authors[0] || '—' }}
            <template v-if="paper.year"> · {{ paper.year }}</template>
          </template>
        </span>
      </div>
      <div class="paper-indicators">
        <span v-if="paper.tags.length" class="indicator tags">
          {{ paper.tags.length }}
        </span>
      </div>
    </div>

    <Teleport to="body">
      <div
        v-if="contextMenuPaperId"
        class="context-menu"
        :style="{ left: contextMenuX + 'px', top: contextMenuY + 'px' }"
      >
        <button class="context-menu-item danger" @click="handleDelete">{{ t('library.deletePaper') }}</button>
      </div>
    </Teleport>
  </div>
</template>

<style lang="scss" scoped>
.paper-list {
  flex: 1;
  overflow-y: auto;
}

.empty-list {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 200px;
  color: $color-text-disabled;
  font-size: 13px;
}

.paper-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: $spacing-md $spacing-lg;
  border-bottom: 1px solid $color-border;
  cursor: pointer;
  transition: background $transition-fast;

  &:hover {
    background: $color-panel;
  }

  &.selected {
    background: $color-panel;
    border-left: 2px solid $color-text-primary;
  }
}

.paper-info {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
  flex: 1;
}

.paper-title {
  font-size: 13px;
  font-weight: 450;
  display: flex;
  align-items: center;
  gap: 6px;
  min-width: 0;
}

.paper-title-text {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.paper-meta {
  font-size: 12px;
  color: $color-text-secondary;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.paper-indicators {
  display: flex;
  gap: $spacing-xs;
  flex-shrink: 0;
}

.indicator {
  font-size: 10px;
  padding: 1px 5px;
  border-radius: 2px;
  background: $color-panel;
  color: $color-text-secondary;
  border: 1px solid $color-border;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.parse-spinner {
  flex-shrink: 0;
  width: 14px;
  height: 14px;
  border: 2px solid $color-border;
  border-top-color: $color-text-primary;
  border-radius: 50%;
  animation: spin 0.6s linear infinite;
}

.parsing {
  background: var(--hover-bg) !important;
}

.parsing-text {
  color: $color-text-primary;
  font-size: 12px;
}
</style>

<style lang="scss">
.context-menu {
  position: fixed;
  z-index: 200;
  background: $color-bg;
  border: 1px solid $color-border;
  border-radius: $radius-sm;
  box-shadow: $shadow-md;
  padding: 4px 0;
  min-width: 100px;
}

.context-menu-item {
  display: block;
  width: 100%;
  padding: 6px 14px;
  border: none;
  background: none;
  font-size: 12px;
  font-family: $font-family;
  text-align: left;
  cursor: pointer;
  color: $color-text-primary;

  &:hover {
    background: $color-panel;
  }

  &.danger {
    color: var(--color-error);
  }
}
</style>
