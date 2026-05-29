<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch, shallowRef, computed } from 'vue';
import cytoscape, { Core, NodeSingular, EdgeSingular, ElementDefinition } from 'cytoscape';
import { useI18n } from 'vue-i18n';
import { papersToElements, getCytoscapeStyles } from '@/utils/graphTransform';
import { usePaperStore } from '@/stores/paperStore';
import { useGraphStore } from '@/stores/graphStore';
import { useProjectStore } from '@/stores/projectStore';
import { useSettingsStore } from '@/stores/settingsStore';

const emit = defineEmits<{
  nodeClick: [id: string];
  edgeClick: [id: string];
}>();

const paperStore = usePaperStore();
const graphStore = useGraphStore();
const projectStore = useProjectStore();
const settingsStore = useSettingsStore();
const { t, locale } = useI18n();

const containerRef = ref<HTMLDivElement>();
const cy = shallowRef<Core>();
const currentZoom = ref(1);
const ZOOM_MIN = 0.2;
const ZOOM_MAX = 3;
const MIN_NODE_DISTANCE = 120;

const zoomPercent = computed(() => Math.round(currentZoom.value * 100));

// --- Layout state machine ---
let layoutRunning = false;
let needsFullLayout = false;

// --- Cytoscape init ---
function initCytoscape() {
  if (!containerRef.value) return;

  cy.value = cytoscape({
    container: containerRef.value,
    elements: [],
    style: getCytoscapeStyles(),
    layout: { name: 'preset' },
    minZoom: ZOOM_MIN,
    maxZoom: ZOOM_MAX,
    wheelSensitivity: 0.2,
  });

  // Restore saved zoom and pan
  const savedZoom = graphStore.graphLayout.zoom;
  if (savedZoom && savedZoom >= ZOOM_MIN && savedZoom <= ZOOM_MAX) {
    cy.value.zoom(savedZoom);
    currentZoom.value = savedZoom;
  }
  const savedPan = graphStore.graphLayout.pan;
  if (savedPan) {
    cy.value.pan(savedPan);
  }

  let zoomSaveTimer: ReturnType<typeof setTimeout> | null = null;

  cy.value.on('zoom', () => {
    if (cy.value) {
      currentZoom.value = cy.value.zoom();
      // Debounced save
      if (zoomSaveTimer) clearTimeout(zoomSaveTimer);
      zoomSaveTimer = setTimeout(() => {
        graphStore.graphLayout.zoom = currentZoom.value;
        graphStore.graphLayout.pan = { ...cy.value!.pan() };
        graphStore.saveLayout();
      }, 500);
    }
  });

  cy.value.on('tap', 'node', (e) => {
    emit('nodeClick', (e.target as NodeSingular).id());
  });

  cy.value.on('tap', 'edge', (e) => {
    emit('edgeClick', (e.target as EdgeSingular).id());
  });

  cy.value.on('tap', (e) => {
    if (e.target === cy.value) {
      emit('nodeClick', '');
      emit('edgeClick', '');
    }
  });

  cy.value.on('dragfree', 'node', (e) => {
    const node = e.target as NodeSingular;
    graphStore.graphLayout.positions[node.id()] = { ...node.position() };
    graphStore.saveLayout();
  });
}

// --- Core sync ---
function syncElements() {
  if (!cy.value) return;

  const elements = papersToElements(paperStore.papers, graphStore.relations);
  const positions = graphStore.graphLayout.positions;

  // Separate nodes and edges
  const desiredNodes: ElementDefinition[] = [];
  const desiredEdges: ElementDefinition[] = [];
  const nodeIds = new Set<string>();
  const edgeIds = new Set<string>();

  for (const el of elements) {
    if ('source' in el.data && 'target' in el.data) {
      desiredEdges.push(el);
      if (el.data.id) edgeIds.add(el.data.id);
    } else {
      desiredNodes.push(el);
      if (el.data.id) nodeIds.add(el.data.id);
    }
  }

  // Remove stale elements
  cy.value.elements().forEach((el) => {
    if (el.isNode() && !nodeIds.has(el.id())) el.remove();
    if (el.isEdge() && !edgeIds.has(el.id())) el.remove();
  });

  // Add nodes first, then edges
  cy.value.add(desiredNodes);
  cy.value.add(desiredEdges);

  // Restore saved positions
  cy.value.nodes().forEach((node) => {
    const pos = positions[node.id()];
    if (pos) node.position(pos);
  });

  // Check what needs layout
  const unpositioned = cy.value.nodes().filter((n) => !positions[n.id()]);

  // If all nodes have saved positions and no forced layout, skip
  if (!needsFullLayout && unpositioned.length === 0) return;
  // If locked and not forced, skip
  if (graphStore.graphLayout.locked && !needsFullLayout) return;

  // If a layout is already running, skip — it will re-sync on completion
  if (layoutRunning) return;

  startLayout(unpositioned);
}

function startLayout(unpositioned: cytoscape.NodeCollection) {
  if (!cy.value || cy.value.nodes().length === 0) return;

  // Spread unpositioned nodes in a circle around center
  if (unpositioned.length > 0) {
    const existing = cy.value.nodes().filter((n) =>
      !!graphStore.graphLayout.positions[n.id()]
    );

    let cx = 300, cy_ = 300;
    if (existing.length > 0) {
      const bb = existing.boundingBox({});
      cx = (bb.x1 + bb.x2) / 2;
      cy_ = (bb.y1 + bb.y2) / 2;
    } else {
      const ext = cy.value.extent();
      cx = (ext.x1 + ext.x2) / 2 || 300;
      cy_ = (ext.y1 + ext.y2) / 2 || 300;
    }

    const radius = Math.max(180, unpositioned.length * 60);
    unpositioned.forEach((node, i) => {
      const angle = (2 * Math.PI * i) / unpositioned.length - Math.PI / 2;
      node.position({
        x: cx + radius * Math.cos(angle),
        y: cy_ + radius * Math.sin(angle),
      });
    });
  }

  layoutRunning = true;
  const isFirst = needsFullLayout;
  needsFullLayout = false;

  // Listen for completion BEFORE running layout
  cy.value.one('layoutstop', () => {
    // Enforce minimum distance between nodes
    enforceMinDistance();

    // Fit all nodes into view, centered with padding
    if (cy.value!.nodes().length > 0) {
      cy.value!.fit(undefined, 40);
      // Cap zoom at 150%
      if (cy.value!.zoom() > 1.5) {
        cy.value!.zoom(1.5);
      }
      currentZoom.value = cy.value!.zoom();
    }

    // Save all positions + viewport
    cy.value!.nodes().forEach((node) => {
      graphStore.graphLayout.positions[node.id()] = { ...node.position() };
    });
    graphStore.graphLayout.zoom = currentZoom.value;
    graphStore.graphLayout.pan = { ...cy.value!.pan() };
    graphStore.saveLayout();
    layoutRunning = false;

    // Re-sync in case data changed during animation
    syncElements();
  });

  cy.value.layout({
    name: 'cose',
    animate: true,
    animationDuration: isFirst ? 400 : 300,
    randomize: false,
    fit: isFirst,
    padding: isFirst ? 40 : 30,
    componentSpacing: 100,
    nodeRepulsion: () => 16000,
    idealEdgeLength: (edge: any) => {
      if (edge.hasClass('relation-opposes')) return 260;
      if (edge.hasClass('relation-supports')) return 180;
      return 200;
    },
    edgeElasticity: (edge: any) => {
      if (edge.hasClass('relation-opposes')) return 80;
      return 120;
    },
    nestingFactor: 0.5,
  }).run();
}

// --- Minimum distance enforcement ---
function enforceMinDistance() {
  if (!cy.value) return;

  const nodes = cy.value.nodes();
  const iterations = 60;

  for (let iter = 0; iter < iterations; iter++) {
    let moved = false;

    for (let i = 0; i < nodes.length; i++) {
      for (let j = i + 1; j < nodes.length; j++) {
        const a = nodes[i];
        const b = nodes[j];
        const pa = a.position();
        const pb = b.position();

        const dx = pb.x - pa.x;
        const dy = pb.y - pa.y;
        const dist = Math.sqrt(dx * dx + dy * dy) || 0.01;

        // Minimum distance based on node sizes
        const sizeA = (a.width() || 40) / 2;
        const sizeB = (b.width() || 40) / 2;
        const minDist = sizeA + sizeB + MIN_NODE_DISTANCE;

        if (dist < minDist) {
          const overlap = (minDist - dist) / 2;
          const nx = dx / dist;
          const ny = dy / dist;

          a.position({
            x: pa.x - nx * overlap,
            y: pa.y - ny * overlap,
          });
          b.position({
            x: pb.x + nx * overlap,
            y: pb.y + ny * overlap,
          });
          moved = true;
        }
      }
    }

    if (!moved) break;
  }
}

// --- Debounced schedule ---
let syncTimer: ReturnType<typeof setTimeout> | null = null;

function scheduleSync() {
  if (syncTimer) clearTimeout(syncTimer);
  syncTimer = setTimeout(() => {
    syncTimer = null;
    syncElements();
  }, 100);
}

watch(
  () => [paperStore.papers.length, graphStore.relations.length],
  () => scheduleSync()
);

// --- Lifecycle ---
let themeObserver: MutationObserver | null = null;

onMounted(() => {
  initCytoscape();
  requestAnimationFrame(() => syncElements());

  themeObserver = new MutationObserver((mutations) => {
    for (const m of mutations) {
      if (m.attributeName === 'data-theme') {
        if (cy.value) cy.value.style(getCytoscapeStyles() as any);
        break;
      }
    }
  });
  themeObserver.observe(document.documentElement, { attributes: true });
});

onUnmounted(() => {
  if (syncTimer) clearTimeout(syncTimer);
  themeObserver?.disconnect();
  cy.value?.destroy();
});

// --- Toolbar ---
function runLayout() {
  if (!cy.value || cy.value.nodes().length === 0) return;
  if (layoutRunning) return;

  needsFullLayout = true;
  syncElements();
}

function zoomIn() {
  if (!cy.value) return;
  const next = Math.min(currentZoom.value * 1.25, ZOOM_MAX);
  cy.value.zoom(next);
  currentZoom.value = next;
}

function zoomOut() {
  if (!cy.value) return;
  const next = Math.max(currentZoom.value * 0.8, ZOOM_MIN);
  cy.value.zoom(next);
  currentZoom.value = next;
}

function onZoomSlider(e: Event) {
  if (!cy.value) return;
  const val = parseFloat((e.target as HTMLInputElement).value);
  cy.value.zoom(val);
  currentZoom.value = val;
}

function fitToScreen() {
  if (!cy.value) return;
  cy.value.fit(undefined, 40);
  if (cy.value.zoom() > 1.5) {
    cy.value.zoom(1.5);
  }
  currentZoom.value = cy.value.zoom();
  graphStore.graphLayout.zoom = currentZoom.value;
  graphStore.graphLayout.pan = { ...cy.value.pan() };
  graphStore.saveLayout();
}

// --- Export ---
// Standard export sizes (width × height) ranked by preference
const EXPORT_SIZES = [
  { w: 1920, h: 1080 }, // 16:9
  { w: 1600, h: 1200 }, // 4:3
  { w: 1800, h: 1200 }, // 3:2
  { w: 1400, h: 1400 }, // square
];

function pickExportSize(bbW: number, bbH: number) {
  if (bbW === 0 || bbH === 0) return EXPORT_SIZES[0];
  const graphAspect = bbW / bbH;
  let best = EXPORT_SIZES[0];
  let bestDiff = Infinity;
  for (const s of EXPORT_SIZES) {
    const diff = Math.abs(s.w / s.h - graphAspect);
    if (diff < bestDiff) { bestDiff = diff; best = s; }
  }
  return best;
}

function exportPng() {
  if (!cy.value || cy.value.nodes().length === 0) return;

  const bb = cy.value.elements().boundingBox({});
  const padding = 60;
  const contentW = bb.w + padding * 2;
  const contentH = bb.h + padding * 2;

  const target = pickExportSize(contentW, contentH);
  const scale = Math.min(target.w / contentW, target.h / contentH, 2);

  // Raw graph image from Cytoscape
  const pngDataUrl = cy.value.png({ full: true, scale, bg: '#FFFFFF', padding } as any);

  const graphImg = new Image();
  graphImg.onload = () => {
    const WATERMARK_H = settingsStore.watermarkEnabled ? 36 : 0;

    const canvas = document.createElement('canvas');
    canvas.width = target.w;
    canvas.height = target.h + WATERMARK_H;
    const ctx = canvas.getContext('2d');
    if (!ctx) { triggerDownload(pngDataUrl); return; }

    // White background
    ctx.fillStyle = '#FFFFFF';
    ctx.fillRect(0, 0, canvas.width, canvas.height);

    // Center graph on canvas
    const ox = (target.w - graphImg.width) / 2;
    const oy = (target.h - graphImg.height) / 2;
    ctx.drawImage(graphImg, ox, oy);

    // Watermark strip
    if (settingsStore.watermarkEnabled) {
      const sepY = target.h;
      ctx.strokeStyle = '#E8E8E8';
      ctx.lineWidth = 1;
      ctx.beginPath();
      ctx.moveTo(0, sepY + 0.5);
      ctx.lineTo(canvas.width, sepY + 0.5);
      ctx.stroke();

      const logoH = Math.round(WATERMARK_H * 0.55);
      const fontSize = Math.round(WATERMARK_H * 0.32);
      const textColor = 'rgba(0, 0, 0, 0.32)';
      const text = locale.value === 'tra'
        ? '由觀復產生'
        : locale.value === 'sim'
          ? '由观复生成'
          : 'Generated by Guanfu';

      ctx.font = `500 ${fontSize}px "HarmonyOS Sans", -apple-system, sans-serif`;
      ctx.fillStyle = textColor;
      const textW = ctx.measureText(text).width;
      const gap = 6;
      const blockW = logoH + gap + textW;
      const bx = (canvas.width - blockW) / 2;
      const by = sepY + (WATERMARK_H - logoH) / 2;

      const logoImg = new Image();
      logoImg.onload = () => {
        ctx.save();
        ctx.beginPath();
        ctx.arc(bx + logoH / 2, by + logoH / 2, logoH / 2, 0, Math.PI * 2);
        ctx.clip();
        ctx.drawImage(logoImg, bx, by, logoH, logoH);
        ctx.restore();
        ctx.fillStyle = textColor;
        ctx.fillText(text, bx + logoH + gap, sepY + WATERMARK_H / 2 + fontSize * 0.35);
        triggerDownload(canvas.toDataURL('image/png'));
      };
      logoImg.onerror = () => {
        ctx.fillStyle = textColor;
        ctx.fillText(text, (canvas.width - textW) / 2, sepY + WATERMARK_H / 2 + fontSize * 0.35);
        triggerDownload(canvas.toDataURL('image/png'));
      };
      logoImg.src = '/logo.png';
    } else {
      triggerDownload(canvas.toDataURL('image/png'));
    }
  };
  graphImg.onerror = () => triggerDownload(pngDataUrl);
  graphImg.src = pngDataUrl;
}

function triggerDownload(dataUrl: string) {
  const link = document.createElement('a');
  link.href = dataUrl;
  const appName = locale.value === 'tra' ? '觀復' : 'Guanfu';
  const projectName = projectStore.currentProject?.name || 'graph';
  link.download = `${appName}-${projectName}.png`;
  link.click();
}

function exportSvg() {
  exportPng();
}

defineExpose({ runLayout, exportPng, exportSvg });
</script>

<template>
  <div class="graph-canvas-wrapper">
    <div ref="containerRef" class="graph-canvas" />
    <div class="zoom-controls">
      <button class="zoom-btn" @click="zoomOut" :title="t('graph.zoomOut')">−</button>
      <input
        type="range"
        class="zoom-slider"
        :min="ZOOM_MIN"
        :max="ZOOM_MAX"
        :step="0.05"
        :value="currentZoom"
        @input="onZoomSlider"
      />
      <button class="zoom-btn" @click="zoomIn" :title="t('graph.zoomIn')">+</button>
      <span class="zoom-label">{{ zoomPercent }}%</span>
      <button class="zoom-btn fit-btn" @click="fitToScreen" :title="t('graph.fitCanvas')">
        <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
          <path d="M2 5V2h3M9 2h3v3M12 9v3H9M5 12H2V9" stroke="currentColor" stroke-width="1.2"/>
        </svg>
      </button>
      <button class="zoom-btn layout-btn" @click="runLayout" :title="t('graph.autoArrange')">
        <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
          <circle cx="4" cy="4" r="1.8" stroke="currentColor" stroke-width="1.2" fill="none"/>
          <circle cx="10" cy="4" r="1.8" stroke="currentColor" stroke-width="1.2" fill="none"/>
          <circle cx="7" cy="10" r="1.8" stroke="currentColor" stroke-width="1.2" fill="none"/>
          <line x1="5.2" y1="5" x2="5.8" y2="8.5" stroke="currentColor" stroke-width="1"/>
          <line x1="8.8" y1="5" x2="8.2" y2="8.5" stroke="currentColor" stroke-width="1"/>
        </svg>
      </button>
    </div>
  </div>
</template>

<style lang="scss" scoped>
.graph-canvas-wrapper {
  flex: 1;
  position: relative;
  overflow: hidden;
}

.graph-canvas {
  width: 100%;
  height: 100%;
}

.zoom-controls {
  position: absolute;
  bottom: $spacing-lg;
  right: $spacing-lg;
  display: flex;
  align-items: center;
  gap: 4px;
  background: $color-bg;
  border: 1px solid $color-border;
  border-radius: $radius-sm;
  padding: 4px 8px;
  box-shadow: $shadow-sm;
  z-index: 10;
}

.zoom-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 24px;
  height: 24px;
  border: none;
  background: none;
  color: $color-text-secondary;
  cursor: pointer;
  font-size: 14px;
  border-radius: $radius-sm;

  &:hover {
    background: $color-panel;
    color: $color-text-primary;
  }
}

.zoom-slider {
  width: 100px;
  height: 4px;
  -webkit-appearance: none;
  appearance: none;
  background: $color-border;
  border-radius: 2px;
  outline: none;
  cursor: pointer;

  &::-webkit-slider-thumb {
    -webkit-appearance: none;
    appearance: none;
    width: 12px;
    height: 12px;
    border-radius: 50%;
    background: $color-text-primary;
    cursor: pointer;
  }

  &::-moz-range-thumb {
    width: 12px;
    height: 12px;
    border-radius: 50%;
    background: $color-text-primary;
    border: none;
    cursor: pointer;
  }
}

.zoom-label {
  font-size: 11px;
  color: $color-text-disabled;
  min-width: 32px;
  text-align: center;
  user-select: none;
}

.fit-btn svg {
  color: inherit;
}
</style>
