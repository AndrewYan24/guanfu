<script setup lang="ts">
import { ref, computed, watch, onUnmounted } from 'vue';
import { useI18n } from 'vue-i18n';

const { t } = useI18n();

const props = defineProps<{
  minYear: number;
  maxYear: number;
}>();

const emit = defineEmits<{
  'update:range': [range: [number, number]];
  play: [];
  pause: [];
}>();

const rangeStart = ref(props.minYear);
const rangeEnd = ref(props.maxYear);
const isPlaying = ref(false);
let playTimer: ReturnType<typeof setInterval> | null = null;

// Reset range when bounds change
watch(() => [props.minYear, props.maxYear], () => {
  rangeStart.value = props.minYear;
  rangeEnd.value = props.maxYear;
});

const yearSpan = computed(() => props.maxYear - props.minYear + 1);

function onRangeStartInput(e: Event) {
  const val = parseInt((e.target as HTMLInputElement).value);
  rangeStart.value = Math.min(val, rangeEnd.value);
  emit('update:range', [rangeStart.value, rangeEnd.value]);
}

function onRangeEndInput(e: Event) {
  const val = parseInt((e.target as HTMLInputElement).value);
  rangeEnd.value = Math.max(val, rangeStart.value);
  emit('update:range', [rangeStart.value, rangeEnd.value]);
}

function togglePlay() {
  if (isPlaying.value) {
    stopPlay();
  } else {
    startPlay();
  }
}

function startPlay() {
  if (yearSpan.value <= 1) return;
  isPlaying.value = true;
  emit('play');

  // Start from min year, end at max year
  rangeStart.value = props.minYear;
  rangeEnd.value = props.minYear;
  emit('update:range', [rangeStart.value, rangeEnd.value]);

  const stepMs = Math.max(400, 2000 / yearSpan.value); // faster for larger spans
  playTimer = setInterval(() => {
    if (rangeEnd.value >= props.maxYear) {
      stopPlay();
      return;
    }
    rangeEnd.value++;
    emit('update:range', [rangeStart.value, rangeEnd.value]);
  }, stepMs);
}

function stopPlay() {
  isPlaying.value = false;
  if (playTimer) {
    clearInterval(playTimer);
    playTimer = null;
  }
  emit('pause');
}

onUnmounted(() => {
  if (playTimer) clearInterval(playTimer);
});
</script>

<template>
  <div class="time-slider" v-if="minYear < maxYear">
    <button class="play-btn" @click="togglePlay" :title="isPlaying ? t('graph.pause') : t('graph.play')">
      <svg v-if="!isPlaying" width="12" height="12" viewBox="0 0 12 12" fill="currentColor">
        <polygon points="2,0 12,6 2,12" />
      </svg>
      <svg v-else width="12" height="12" viewBox="0 0 12 12" fill="currentColor">
        <rect x="1" y="0" width="4" height="12" />
        <rect x="7" y="0" width="4" height="12" />
      </svg>
    </button>
    <span class="year-label">{{ rangeStart }}</span>
    <div class="slider-track">
      <input
        type="range"
        class="range-input range-start"
        :min="minYear"
        :max="maxYear"
        :step="1"
        :value="rangeStart"
        @input="onRangeStartInput"
      />
      <input
        type="range"
        class="range-input range-end"
        :min="minYear"
        :max="maxYear"
        :step="1"
        :value="rangeEnd"
        @input="onRangeEndInput"
      />
      <div
        class="slider-fill"
        :style="{
          left: ((rangeStart - minYear) / (maxYear - minYear)) * 100 + '%',
          right: (1 - (rangeEnd - minYear) / (maxYear - minYear)) * 100 + '%',
        }"
      />
    </div>
    <span class="year-label">{{ rangeEnd }}</span>
  </div>
</template>

<style lang="scss" scoped>
.time-slider {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 12px;
  background: $color-bg;
  border-top: 1px solid $color-border;
  flex-shrink: 0;
}

.play-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 24px;
  height: 24px;
  border: 1px solid $color-border;
  border-radius: $radius-sm;
  background: $color-bg;
  color: $color-text-secondary;
  cursor: pointer;
  flex-shrink: 0;

  &:hover {
    border-color: $color-text-secondary;
    color: $color-text-primary;
    background: $color-panel;
  }
}

.year-label {
  font-size: 11px;
  color: $color-text-secondary;
  min-width: 32px;
  text-align: center;
  font-variant-numeric: tabular-nums;
  user-select: none;
}

.slider-track {
  flex: 1;
  position: relative;
  height: 20px;
  display: flex;
  align-items: center;
}

.range-input {
  position: absolute;
  width: 100%;
  height: 4px;
  -webkit-appearance: none;
  appearance: none;
  background: transparent;
  pointer-events: none;
  outline: none;
  margin: 0;

  &::-webkit-slider-thumb {
    -webkit-appearance: none;
    appearance: none;
    width: 12px;
    height: 12px;
    border-radius: 50%;
    background: $color-text-primary;
    cursor: pointer;
    pointer-events: auto;
    position: relative;
    z-index: 2;
  }

  &::-moz-range-thumb {
    width: 12px;
    height: 12px;
    border-radius: 50%;
    background: $color-text-primary;
    border: none;
    cursor: pointer;
    pointer-events: auto;
  }
}

.slider-fill {
  position: absolute;
  height: 4px;
  background: $color-text-primary;
  border-radius: 2px;
  pointer-events: none;
  z-index: 1;
}
</style>
