import { defineStore } from 'pinia';
import { ref } from 'vue';
import type { AiSettings, MaskedAiSettings } from '@/types';
import * as aiApi from '@/api/aiApi';

export const useSettingsStore = defineStore('settings', () => {
  const maskedSettings = ref<MaskedAiSettings | null>(null);
  const isLoading = ref(false);
  const watermarkEnabled = ref(
    localStorage.getItem('guanfu_watermark') !== 'false'
  );

  async function loadSettings() {
    isLoading.value = true;
    try {
      maskedSettings.value = await aiApi.getAiSettingsMasked();
    } finally {
      isLoading.value = false;
    }
  }

  async function saveSettings(settings: AiSettings) {
    await aiApi.saveAiSettings(settings);
    await loadSettings();
  }

  async function testConnection(settings?: AiSettings) {
    if (settings) {
      return aiApi.testAiConnection(settings);
    }
    return aiApi.testAiConnectionStored();
  }

  function setWatermarkEnabled(enabled: boolean) {
    watermarkEnabled.value = enabled;
    localStorage.setItem('guanfu_watermark', String(enabled));
  }

  return {
    maskedSettings,
    isLoading,
    watermarkEnabled,
    loadSettings,
    saveSettings,
    testConnection,
    setWatermarkEnabled,
  };
});
