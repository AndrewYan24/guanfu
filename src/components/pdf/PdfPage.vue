<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch, toRaw } from 'vue';

// eslint-disable-next-line @typescript-eslint/no-explicit-any
type PDFPageProxy = any;

const props = defineProps<{
  page: PDFPageProxy;
  scale: number;
}>();

const canvasRef = ref<HTMLCanvasElement>();
const textLayerRef = ref<HTMLDivElement>();
// eslint-disable-next-line @typescript-eslint/no-explicit-any
let renderTask: any = null;

async function renderPage() {
  if (!canvasRef.value) return;

  if (renderTask) {
    renderTask.cancel();
    renderTask = null;
  }

  // Unwrap Vue Proxy — pdfjs 4.x uses private fields that break through Proxy
  const page = toRaw(props.page);

  try {
    const vp = page.getViewport({ scale: props.scale });

    const canvas = canvasRef.value;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    canvas.width = vp.width;
    canvas.height = vp.height;

    renderTask = page.render({ canvasContext: ctx, viewport: vp });
    await renderTask.promise;
    renderTask = null;

    await renderTextLayer(vp);
  } catch (e) {
    if ((e as Error)?.name === 'RenderingCancelledException') return;
    console.error('[PdfPage] render error:', e);
  }
}

async function renderTextLayer(vp: unknown) {
  if (!textLayerRef.value) return;

  const page = toRaw(props.page);
  const viewport = vp as { width: number; height: number };
  textLayerRef.value.innerHTML = '';
  textLayerRef.value.style.width = `${viewport.width}px`;
  textLayerRef.value.style.height = `${viewport.height}px`;

  try {
    const textContent = await page.getTextContent();
    const pdfjs = await import('pdfjs-dist');

    // pdfjs 4.x TextLayer
    const textLayer = new pdfjs.TextLayer({
      textContentSource: textContent,
      container: textLayerRef.value,
      viewport: vp,
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
    } as any);
    await textLayer.render();
  } catch {
    // Text layer rendering failed - canvas still works
  }
}

watch(() => props.scale, renderPage);

onMounted(renderPage);

onUnmounted(() => {
  if (renderTask) {
    renderTask.cancel();
  }
});
</script>

<template>
  <div class="pdf-page">
    <canvas ref="canvasRef" class="pdf-canvas" />
    <div ref="textLayerRef" class="text-layer" />
  </div>
</template>

<style lang="scss" scoped>
.pdf-page {
  position: relative;
  margin-bottom: 8px;
  box-shadow: $shadow-sm;
  background: white;
}

.pdf-canvas {
  display: block;
}

.text-layer {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  overflow: hidden;
  opacity: 0.2;
  line-height: 1;
  pointer-events: auto;

  :deep(span) {
    color: transparent;
    position: absolute;
    white-space: pre;
    transform-origin: 0% 0%;
  }

  :deep(::selection) {
    background: rgba(0, 0, 0, 0.2);
  }
}
</style>
