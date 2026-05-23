import { ref, watch } from 'vue';

export type ThemeMode = 'system' | 'light' | 'dark';

const THEME_KEY = 'guanfu_theme';

const mode = ref<ThemeMode>(
  (localStorage.getItem(THEME_KEY) as ThemeMode) || 'system'
);

let initialized = false;

function getSystemTheme(): 'light' | 'dark' {
  return window.matchMedia('(prefers-color-scheme: dark)').matches
    ? 'dark'
    : 'light';
}

function applyTheme() {
  const effective = mode.value === 'system' ? getSystemTheme() : mode.value;
  document.documentElement.setAttribute(
    'data-theme',
    effective === 'dark' ? 'dark' : ''
  );
}

function initThemeListeners() {
  if (initialized) return;
  initialized = true;

  // System theme change
  window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', () => {
    if (mode.value === 'system') applyTheme();
  });

  // Re-apply when window regains visibility (catches changes while minimized)
  document.addEventListener('visibilitychange', () => {
    if (document.visibilityState === 'visible' && mode.value === 'system') {
      applyTheme();
    }
  });

  // React to manual mode changes
  watch(mode, applyTheme);
}

// Apply immediately to prevent FOUC
applyTheme();

export function useTheme() {
  initThemeListeners();

  function setMode(newMode: ThemeMode) {
    mode.value = newMode;
    localStorage.setItem(THEME_KEY, newMode);
  }

  return { mode, setMode };
}
