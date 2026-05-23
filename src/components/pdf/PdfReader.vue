<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch, shallowRef } from 'vue';
import { useI18n } from 'vue-i18n';
import { pdfjsLib } from '@/utils/pdfSetup';
import { usePaperStore } from '@/stores/paperStore';
import PdfPage from './PdfPage.vue';

const { t } = useI18n();

const props = defineProps<{
  data: Uint8Array | null;
}>();

const paperStore = usePaperStore();

// eslint-disable-next-line @typescript-eslint/no-explicit-any
const pages = ref<any[]>([]);
const scale = ref(1.2);
const isLoading = ref(false);
const loadError = ref('');
const numPages = ref(0);
// eslint-disable-next-line @typescript-eslint/no-explicit-any
const pdfDoc = shallowRef<any>();

const scrollEl = ref<HTMLElement | null>(null);
let currentPaperId: string | null = null;
let enableScrollSave = false;

function saveCurrentScroll() {
  if (scrollEl.value && currentPaperId) {
    paperStore.savePdfScrollPosition(currentPaperId, scrollEl.value.scrollTop);
  }
}

function restoreScrollPosition() {
  if (!scrollEl.value || !currentPaperId) return;
  const pos = paperStore.getPdfScrollPosition(currentPaperId);
  if (pos > 0) {
    scrollEl.value.scrollTop = pos;
  }
  enableScrollSave = true;
}

async function loadPdf() {
  if (!props.data || props.data.length === 0) {
    pages.value = [];
    numPages.value = 0;
    pdfDoc.value?.destroy();
    pdfDoc.value = null;
    return;
  }

  isLoading.value = true;
  loadError.value = '';
  pages.value = [];
  pdfDoc.value?.destroy();

  try {
    const loadingTask = pdfjsLib.getDocument({ data: props.data });
    const doc = await loadingTask.promise;
    pdfDoc.value = doc;
    numPages.value = doc.numPages;

    const loadedPages: unknown[] = [];
    for (let i = 1; i <= doc.numPages; i++) {
      const page = await doc.getPage(i);
      loadedPages.push(page);
    }
    pages.value = loadedPages;

    enableScrollSave = false;
    tryRestore();
  } catch (e) {
    loadError.value = e instanceof Error ? e.message : '加载 PDF 失败';
  } finally {
    isLoading.value = false;
  }
}

function tryRestore(attempt = 0) {
  if (attempt > 10) {
    enableScrollSave = true;
    return;
  }
  const delay = attempt === 0 ? 0 : 100;
  setTimeout(() => {
    if (scrollEl.value && scrollEl.value.scrollHeight > scrollEl.value.clientHeight) {
      restoreScrollPosition();
    } else {
      tryRestore(attempt + 1);
    }
  }, delay);
}

function onScroll() {
  if (enableScrollSave) {
    saveCurrentScroll();
  }
}

function handleZoomIn() {
  scale.value = Math.min(scale.value + 0.2, 3);
}

function handleZoomOut() {
  scale.value = Math.max(scale.value - 0.2, 0.5);
}

onMounted(() => {
  currentPaperId = paperStore.selectedPaperId;
  if (props.data) loadPdf();
});

watch(() => props.data, loadPdf);

onUnmounted(() => {
  saveCurrentScroll();
  pdfDoc.value?.destroy();
  scrollEl.value?.removeEventListener('scroll', onScroll);
});

watch(() => paperStore.selectedPaperId, (newId, oldId) => {
  if (oldId && oldId !== newId) {
    saveCurrentScroll();
  }
  currentPaperId = newId;
}, { flush: 'pre' });
</script>

<template>
  <div class="pdf-reader">
    <div class="pdf-container" ref="scrollEl" @scroll="onScroll">
      <div v-if="isLoading" class="loading">{{ t('common.loading') }}</div>
      <div v-else-if="loadError" class="error">{{ loadError }}</div>
      <div v-else-if="!data" class="loading">{{ t('pdf.selectToRead') }}</div>
      <div v-else class="pages-wrapper">
        <PdfPage
          v-for="(page, index) in pages"
          :key="index"
          :page="page"
          :scale="scale"
        />
      </div>
    </div>

    <div v-if="numPages || data" class="pdf-controls">
      <button class="ctrl-btn" @click="handleZoomIn" title="放大">
        <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
          <line x1="7" y1="3" x2="7" y2="11" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>
          <line x1="3" y1="7" x2="11" y2="7" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>
        </svg>
      </button>
      <span class="zoom-label">{{ Math.round(scale * 100) }}%</span>
      <button class="ctrl-btn" @click="handleZoomOut" title="缩小">
        <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
          <line x1="3" y1="7" x2="11" y2="7" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>
        </svg>
      </button>
      <div class="ctrl-divider" />
      <span v-if="numPages" class="page-info">{{ numPages }} 页</span>
    </div>
  </div>
</template>

<style lang="scss" scoped>
.pdf-reader {
  display: flex;
  height: 100%;
  position: relative;
}

.pdf-container {
  flex: 1;
  overflow-y: auto;
  padding: $spacing-lg;
  background: $color-panel;
}

.loading,
.error {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 200px;
  color: $color-text-disabled;
  font-size: 13px;
}

.error {
  color: $color-text-secondary;
}

.pages-wrapper {
  display: flex;
  flex-direction: column;
  align-items: center;
}

.pdf-controls {
  position: absolute;
  top: 50%;
  right: $spacing-lg;
  transform: translateY(-50%);
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 2px;
  background: $color-bg;
  border: 1px solid $color-border;
  border-radius: $radius-sm;
  padding: 4px;
  box-shadow: $shadow-sm;
  z-index: 10;
}

.ctrl-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
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

.zoom-label {
  font-size: 10px;
  color: $color-text-disabled;
  min-width: 32px;
  text-align: center;
  padding: 2px 0;
  user-select: none;
}

.ctrl-divider {
  width: 16px;
  height: 1px;
  background: $color-border;
  margin: 2px 0;
}

.page-info {
  font-size: 10px;
  color: $color-text-disabled;
  text-align: center;
  user-select: none;
}
</style>
