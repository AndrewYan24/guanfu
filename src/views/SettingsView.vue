<script setup lang="ts">
import { ref, reactive, watch, onMounted, nextTick } from 'vue';
import { useI18n } from 'vue-i18n';
import { useSettingsStore } from '@/stores/settingsStore';
import { useTheme } from '@/composables/useTheme';
import { useToast } from '@/composables/useToast';
import { open } from '@tauri-apps/plugin-dialog';
import { availableLocales, setLocale, getLocale } from '@/i18n';
import { toggleHttpServer, checkServerOcrModels, downloadServerOcrModels } from '@/api/aiApi';
import type { AiSettings, OcrMethod, OcrModelMode } from '@/types';
import type { ThemeMode } from '@/composables/useTheme';

const { t } = useI18n();
const settingsStore = useSettingsStore();
const { mode: themeMode, setMode } = useTheme();
const toast = useToast();

function setTheme(mode: ThemeMode) {
  setMode(mode);
  autoSave();
}

const loaded = ref(false);
const testResult = ref('');
const isTesting = ref(false);

const activeProvider = ref<'openaiCompatible' | 'anthropic' | null>(null);
const ocrMethod = ref<OcrMethod>('local');
const ocrModelMode = ref<OcrModelMode>('mobile');
const serverModelsDownloaded = ref(false);
const isDownloadingModels = ref(false);

const openai = reactive({
  apiKey: '',
  baseUrl: 'https://api.openai.com/v1',
  model: 'gpt-4o',
  maskedKey: '',
});

const anthropic = reactive({
  apiKey: '',
  baseUrl: '',
  model: 'claude-sonnet-4-20250514',
  maskedKey: '',
});

const mineru = reactive({
  apiKey: '',
  apiBase: 'https://mineru.net/api',
  maskedKey: '',
});

const embedding = reactive({
  model: '',
  baseUrl: '',
  apiKey: '',
  maskedKey: '',
});

const projectDir = ref('');

const httpApiEnabled = ref(false);
const httpApiPort = ref(17800);

const advancedConcurrency = ref(3);
const advancedAutoParse = ref(true);
const advancedRetryCount = ref(1);

onMounted(async () => {
  if (!settingsStore.maskedSettings) {
    await settingsStore.loadSettings();
  }
  loadForm();
  await nextTick();
  loaded.value = true;
});

function loadForm() {
  const s = settingsStore.maskedSettings;
  if (!s) return;

  activeProvider.value = s.activeProvider ?? null;
  ocrMethod.value = s.ocrMethod || 'local';
  ocrModelMode.value = s.ocrModelMode || 'mobile';

  // Check server model download status
  checkServerOcrModels().then(downloaded => {
    serverModelsDownloaded.value = downloaded;
  });

  if (s.openaiCompatible) {
    openai.baseUrl = s.openaiCompatible.baseUrl || 'https://api.openai.com/v1';
    openai.model = s.openaiCompatible.model || 'gpt-4o';
    openai.maskedKey = s.openaiCompatible.maskedApiKey || '';
    openai.apiKey = '';
  }

  if (s.anthropic) {
    anthropic.baseUrl = s.anthropic.baseUrl || '';
    anthropic.model = s.anthropic.model || 'claude-sonnet-4-20250514';
    anthropic.maskedKey = s.anthropic.maskedApiKey || '';
    anthropic.apiKey = '';
  }

  if (s.mineru) {
    mineru.apiBase = s.mineru.apiBase || 'https://mineru.net/api';
    mineru.maskedKey = s.mineru.maskedApiKey || '';
    mineru.apiKey = '';
  }

  embedding.model = s.embeddingModel || '';
  embedding.baseUrl = s.embeddingBaseUrl || '';
  embedding.maskedKey = s.maskedEmbeddingApiKey || '';
  embedding.apiKey = '';

  projectDir.value = s.defaultProjectDir || '';
  httpApiEnabled.value = s.httpApiEnabled || false;
  httpApiPort.value = s.httpApiPort || 17800;
  advancedConcurrency.value = s.advanced?.concurrency ?? 3;
  advancedAutoParse.value = s.advanced?.autoParse ?? true;
  advancedRetryCount.value = s.advanced?.retryCount ?? 1;
}

function selectProvider(p: 'openaiCompatible' | 'anthropic' | null) {
  activeProvider.value = p;
  autoSave();
}

function buildPayload(): AiSettings {
  const settings: AiSettings = {
    activeProvider: activeProvider.value,
    ocrMethod: ocrMethod.value,
    ocrModelMode: ocrModelMode.value,
    locale: getLocale(),
  };

  if (activeProvider.value === 'openaiCompatible' || openai.maskedKey) {
    settings.openaiCompatible = {
      enabled: activeProvider.value === 'openaiCompatible',
      apiKey: openai.apiKey,
      baseUrl: openai.baseUrl || undefined,
      model: openai.model,
    };
  }

  if (activeProvider.value === 'anthropic' || anthropic.maskedKey) {
    settings.anthropic = {
      enabled: activeProvider.value === 'anthropic',
      apiKey: anthropic.apiKey,
      baseUrl: anthropic.baseUrl || undefined,
      model: anthropic.model,
    };
  }

  if (ocrMethod.value === 'mineru') {
    settings.mineru = {
      apiKey: mineru.apiKey,
      apiBase: mineru.apiBase,
    };
  }

  if (embedding.model) {
    settings.embeddingModel = embedding.model;
  }
  if (embedding.baseUrl) {
    settings.embeddingBaseUrl = embedding.baseUrl;
  }
  if (embedding.apiKey) {
    settings.embeddingApiKey = embedding.apiKey;
  }

  if (projectDir.value) {
    settings.defaultProjectDir = projectDir.value;
  }

  settings.httpApiEnabled = httpApiEnabled.value;
  settings.httpApiPort = httpApiPort.value;

  settings.advanced = {
    concurrency: advancedConcurrency.value,
    autoParse: advancedAutoParse.value,
    retryCount: advancedRetryCount.value,
  };

  return settings;
}

let saveTimer: ReturnType<typeof setTimeout> | null = null;

function autoSave() {
  if (!loaded.value) return;
  if (saveTimer) clearTimeout(saveTimer);
  saveTimer = setTimeout(async () => {
    try {
      await settingsStore.saveSettings(buildPayload());
      toast.show(t('common.saved'), 'success');
    } catch {
      toast.show(t('common.saveFailed'), 'error');
    }
  }, 600);
}

// Watch all form fields for changes
watch(
  () => [
    activeProvider.value,
    ocrMethod.value,
    ocrModelMode.value,
    openai.apiKey, openai.baseUrl, openai.model,
    anthropic.apiKey, anthropic.baseUrl, anthropic.model,
    mineru.apiKey, mineru.apiBase,
    embedding.model, embedding.baseUrl, embedding.apiKey,
    projectDir.value,
    httpApiPort.value,
    advancedConcurrency.value,
    advancedAutoParse.value,
    advancedRetryCount.value,
  ],
  () => autoSave(),
  { deep: false }
);

async function browseProjectDir() {
  const dir = await open({
    directory: true,
    multiple: false,
    title: t('settings.projectDirBrowse'),
  });
  if (dir) {
    projectDir.value = dir;
  }
}

async function handleHttpToggle() {
  try {
    const running = await toggleHttpServer(httpApiEnabled.value);
    httpApiEnabled.value = running;
    toast.show(
      running ? t('settings.httpApiStarted') : t('settings.httpApiStopped'),
      'success'
    );
  } catch (e) {
    httpApiEnabled.value = false;
    toast.show(String(e), 'error');
  }
}

async function handleDownloadServerModels() {
  isDownloadingModels.value = true;
  try {
    await downloadServerOcrModels();
    serverModelsDownloaded.value = true;
    toast.show(t('settings.ocrModelDownloaded'), 'success');
  } catch (e) {
    toast.show(String(e), 'error');
  } finally {
    isDownloadingModels.value = false;
  }
}

async function handleTest() {
  isTesting.value = true;
  testResult.value = '';
  try {
    const ok = await settingsStore.testConnection();
    testResult.value = ok ? t('settings.connectionSuccess') : t('settings.connectionFailed');
  } catch {
    testResult.value = t('settings.connectionFailed');
  } finally {
    isTesting.value = false;
  }
}

interface NavItem {
  id: string;
  label: string;
}

const navItems: NavItem[] = [
  { id: 'provider', label: 'settings.modelProvider' },
  { id: 'embedding', label: 'settings.embedding' },
  { id: 'ocr', label: 'settings.ocr' },
  { id: 'httpApi', label: 'settings.httpApi' },
  { id: 'appearance', label: 'settings.appearance' },
  { id: 'language', label: 'settings.language' },
  { id: 'project', label: 'settings.project' },
  { id: 'export', label: 'settings.export' },
  { id: 'about', label: 'settings.about' },
];

interface OssProject {
  name: string;
  url: string;
  category: string;
}

const ossProjects: OssProject[] = [
  { name: 'Tauri', url: 'https://tauri.app', category: 'Desktop' },
  { name: 'Vue.js', url: 'https://vuejs.org', category: 'Frontend' },
  { name: 'Vite', url: 'https://vitejs.dev', category: 'Frontend' },
  { name: 'Pinia', url: 'https://pinia.vuejs.org', category: 'Frontend' },
  { name: 'Cytoscape.js', url: 'https://js.cytoscape.org', category: 'Frontend' },
  { name: 'PDF.js', url: 'https://mozilla.github.io/pdf.js', category: 'Frontend' },
  { name: 'Lucide', url: 'https://lucide.dev', category: 'Frontend' },
  { name: 'vue-i18n', url: 'https://vue-i18n.intlify.dev', category: 'Frontend' },
  { name: 'axum', url: 'https://github.com/tokio-rs/axum', category: 'Backend' },
  { name: 'lopdf', url: 'https://github.com/J-F-Liu/lopdf', category: 'Backend' },
  { name: 'reqwest', url: 'https://github.com/seanmonstar/reqwest', category: 'Backend' },
  { name: 'serde', url: 'https://github.com/serde-rs/serde', category: 'Backend' },
  { name: 'PaddleOCR', url: 'https://github.com/PaddlePaddle/PaddleOCR', category: 'OCR' },
  { name: 'RapidOCR', url: 'https://github.com/RapidAI/RapidOCR', category: 'OCR' },
  { name: 'ort (ONNX Runtime)', url: 'https://github.com/pykeio/ort', category: 'OCR' },
  { name: 'pdf-render', url: 'https://github.com/laurentzziu/pdf-render', category: 'OCR' },
];

function scrollTo(id: string) {
  const el = document.getElementById(id);
  if (el) el.scrollIntoView({ behavior: 'smooth', block: 'start' });
}
</script>

<template>
  <div class="settings-view">
    <div class="settings-content">
      <div class="settings-header">
        <h3 class="settings-title">{{ t('settings.title') }}</h3>
      </div>

      <nav class="settings-nav">
        <button
          v-for="item in navItems"
          :key="item.id"
          class="nav-item"
          @click="scrollTo(item.id)"
        >
          {{ t(item.label) }}
        </button>
      </nav>

      <section id="provider" class="settings-section">
        <h4 class="section-title">{{ t('settings.modelProvider') }}</h4>
        <p class="section-desc">{{ t('settings.modelProviderDesc') }}</p>

        <!-- OpenAI Compatible -->
        <div class="provider-card" :class="{ active: activeProvider === 'openaiCompatible' }" @click="selectProvider('openaiCompatible')">
          <div class="provider-header">
            <div class="provider-radio">
              <input type="radio" :checked="activeProvider === 'openaiCompatible'" @change="selectProvider('openaiCompatible')" />
            </div>
            <div class="provider-info">
              <span class="provider-name">{{ t('settings.openaiCompatible') }}</span>
              <span class="provider-hint">{{ t('settings.openaiHint') }}</span>
            </div>
          </div>
          <div v-if="activeProvider === 'openaiCompatible'" class="provider-fields" @click.stop>
            <div class="field">
              <label>{{ t('settings.apiKey') }}</label>
              <input
                type="password"
                v-model="openai.apiKey"
                :placeholder="openai.maskedKey || 'sk-...'"
                class="input input-api-key"
              />
            </div>
            <div class="field">
              <label>{{ t('settings.baseUrl') }}</label>
              <input
                type="text"
                v-model="openai.baseUrl"
                placeholder="https://api.openai.com/v1"
                class="input"
              />
              <span class="field-hint">{{ t('settings.baseUrlHint') }}</span>
            </div>
            <div class="field">
              <label>{{ t('settings.model') }}</label>
              <input
                type="text"
                v-model="openai.model"
                placeholder="gpt-4o"
                class="input"
              />
            </div>
          </div>
        </div>

        <!-- Anthropic -->
        <div class="provider-card" :class="{ active: activeProvider === 'anthropic' }" @click="selectProvider('anthropic')">
          <div class="provider-header">
            <div class="provider-radio">
              <input type="radio" :checked="activeProvider === 'anthropic'" @change="selectProvider('anthropic')" />
            </div>
            <div class="provider-info">
              <span class="provider-name">{{ t('settings.anthropic') }}</span>
              <span class="provider-hint">{{ t('settings.anthropicHint') }}</span>
            </div>
          </div>
          <div v-if="activeProvider === 'anthropic'" class="provider-fields" @click.stop>
            <div class="field">
              <label>{{ t('settings.apiKey') }}</label>
              <input
                type="password"
                v-model="anthropic.apiKey"
                :placeholder="anthropic.maskedKey || 'sk-ant-...'"
                class="input input-api-key"
              />
            </div>
            <div class="field">
              <label>{{ t('settings.baseUrlOptional') }}</label>
              <input
                type="text"
                v-model="anthropic.baseUrl"
                :placeholder="t('settings.defaultEndpoint')"
                class="input"
              />
              <span class="field-hint">{{ t('settings.baseUrlAnthropicHint') }}</span>
            </div>
            <div class="field">
              <label>{{ t('settings.model') }}</label>
              <input
                type="text"
                v-model="anthropic.model"
                placeholder="claude-sonnet-4-20250514"
                class="input"
              />
            </div>
          </div>
        </div>

        <!-- No provider -->
        <div class="provider-card" :class="{ active: activeProvider === null }" @click="selectProvider(null)">
          <div class="provider-header">
            <div class="provider-radio">
              <input type="radio" :checked="activeProvider === null" @change="selectProvider(null)" />
            </div>
            <div class="provider-info">
              <span class="provider-name">{{ t('settings.notUse') }}</span>
              <span class="provider-hint">{{ t('settings.notUseHint') }}</span>
            </div>
          </div>
        </div>

        <div v-if="activeProvider" class="test-section">
          <button class="btn-secondary" :disabled="isTesting" @click="handleTest">
            {{ isTesting ? t('settings.testing') : t('settings.testConnection') }}
          </button>
          <span v-if="testResult" class="test-result" :class="{ success: testResult === t('settings.connectionSuccess') }">
            {{ testResult }}
          </span>
        </div>
      </section>

      <section v-if="activeProvider" id="embedding" class="settings-section">
        <h4 class="section-title">{{ t('settings.embedding') }}</h4>
        <p class="section-desc">{{ t('settings.embeddingDesc') }}</p>

        <div class="embedding-fields">
          <div class="field">
            <label>{{ t('settings.embeddingApiKey') }}</label>
            <input
              type="password"
              v-model="embedding.apiKey"
              :placeholder="embedding.maskedKey || t('settings.embeddingApiKeyHint')"
              class="input input-api-key"
            />
          </div>
          <div class="field">
            <label>{{ t('settings.embeddingModel') }}</label>
            <input
              type="text"
              v-model="embedding.model"
              :placeholder="t('settings.embeddingModelHint')"
              class="input"
            />
          </div>
          <div class="field">
            <label>{{ t('settings.embeddingBaseUrl') }}</label>
            <input
              type="text"
              v-model="embedding.baseUrl"
              placeholder="https://api.openai.com/v1"
              class="input"
            />
            <span class="field-hint">{{ t('settings.embeddingBaseUrlHint') }}</span>
          </div>
        </div>
      </section>

      <section id="ocr" class="settings-section">
        <h4 class="section-title">{{ t('settings.ocr') }}</h4>
        <p class="section-desc">{{ t('settings.ocrDesc') }}</p>

        <div class="ocr-options">
          <label class="ocr-option" :class="{ active: ocrMethod === 'local' }">
            <input type="radio" value="local" v-model="ocrMethod" />
            <div class="ocr-info">
              <span class="ocr-name">{{ t('settings.localOcr') }}</span>
              <span class="ocr-hint">{{ t('settings.localOcrHint') }}</span>
            </div>
          </label>
          <label class="ocr-option" :class="{ active: ocrMethod === 'mineru' }">
            <input type="radio" value="mineru" v-model="ocrMethod" />
            <div class="ocr-info">
              <span class="ocr-name">{{ t('settings.mineru') }}</span>
              <span class="ocr-hint">{{ t('settings.mineruHint') }}</span>
            </div>
          </label>
        </div>

        <div v-if="ocrMethod === 'local'" class="model-options">
          <label class="model-option" :class="{ active: ocrModelMode === 'mobile' }">
            <input type="radio" value="mobile" v-model="ocrModelMode" />
            <div class="model-info">
              <span class="model-name">{{ t('settings.ocrModelMobile') }}</span>
              <span class="model-hint">{{ t('settings.ocrModelMobileHint') }}</span>
            </div>
          </label>
          <label class="model-option" :class="{ active: ocrModelMode === 'server' }">
            <input type="radio" value="server" v-model="ocrModelMode" />
            <div class="model-info">
              <span class="model-name">
                {{ t('settings.ocrModelServer') }}
                <span v-if="serverModelsDownloaded" class="model-badge downloaded">{{ t('settings.ocrModelDownloaded') }}</span>
                <span v-else-if="isDownloadingModels" class="model-badge downloading">{{ t('settings.ocrModelDownloading') }}</span>
                <span v-else class="model-badge not-downloaded">{{ t('settings.ocrModelNotDownloaded') }}</span>
              </span>
              <span class="model-hint">{{ t('settings.ocrModelServerHint') }}</span>
            </div>
          </label>
          <div v-if="ocrModelMode === 'server' && !serverModelsDownloaded && !isDownloadingModels" class="model-download-row">
            <button class="btn-secondary btn-sm" @click="handleDownloadServerModels">
              {{ t('settings.ocrModelDownload') }}
            </button>
          </div>
        </div>

        <div v-if="ocrMethod === 'mineru'" class="mineru-fields">
          <div class="field">
            <label>{{ t('settings.apiKey') }}</label>
            <input
              type="password"
              v-model="mineru.apiKey"
              :placeholder="mineru.maskedKey || 'MinerU API Key'"
              class="input input-api-key"
            />
          </div>
          <div class="field">
            <label>{{ t('settings.baseUrl') }}</label>
            <input
              type="text"
              v-model="mineru.apiBase"
              placeholder="https://mineru.net/api"
              class="input"
            />
          </div>
        </div>
      </section>

      <details id="advanced" class="settings-section advanced-section">
        <summary class="section-title summary-title">{{ t('settings.advanced') }}</summary>
        <p class="section-desc">{{ t('settings.advancedDesc') }}</p>

        <div class="advanced-fields">
          <div class="field">
            <label>{{ t('settings.concurrency') }}</label>
            <input
              type="number"
              v-model.number="advancedConcurrency"
              min="1"
              max="5"
              class="input port-input"
            />
            <span class="field-hint">{{ t('settings.concurrencyHint') }}</span>
          </div>

          <label class="toggle-row">
            <span class="toggle-info">
              <span class="toggle-label">{{ t('settings.autoParse') }}</span>
              <span class="toggle-hint">{{ t('settings.autoParseHint') }}</span>
            </span>
            <input
              type="checkbox"
              class="toggle-switch"
              :checked="advancedAutoParse"
              @change="advancedAutoParse = ($event.target as HTMLInputElement).checked"
            />
          </label>

          <div class="field">
            <label>{{ t('settings.retryCount') }}</label>
            <input
              type="number"
              v-model.number="advancedRetryCount"
              min="0"
              max="3"
              class="input port-input"
            />
            <span class="field-hint">{{ t('settings.retryCountHint') }}</span>
          </div>
        </div>
      </details>

      <section id="httpApi" class="settings-section">
        <h4 class="section-title">{{ t('settings.httpApi') }}</h4>
        <p class="section-desc">{{ t('settings.httpApiDesc') }}</p>

        <label class="toggle-row">
          <span class="toggle-info">
            <span class="toggle-label">{{ t('settings.httpApiEnable') }}</span>
            <span class="toggle-hint">{{ t('settings.httpApiEnableHint') }}</span>
          </span>
          <input
            type="checkbox"
            class="toggle-switch"
            :checked="httpApiEnabled"
            @change="httpApiEnabled = ($event.target as HTMLInputElement).checked; handleHttpToggle()"
          />
        </label>

        <div v-if="httpApiEnabled" class="http-api-info">
          <div class="field">
            <label>{{ t('settings.httpApiPort') }}</label>
            <input
              type="number"
              v-model.number="httpApiPort"
              min="1024"
              max="65535"
              class="input port-input"
            />
            <span class="field-hint">{{ t('settings.httpApiPortHint') }}</span>
          </div>
          <div class="http-api-links">
            <a :href="`http://localhost:${httpApiPort}`" target="_blank" class="http-link">
              {{ t('settings.httpApiDocs') }}
            </a>
            <a :href="`http://localhost:${httpApiPort}/openapi.json`" target="_blank" class="http-link">
              OpenAPI
            </a>
          </div>
        </div>
      </section>

      <section id="appearance" class="settings-section">
        <h4 class="section-title">{{ t('settings.appearance') }}</h4>
        <div class="theme-options">
          <label class="theme-option" :class="{ active: themeMode === 'system' }">
            <input type="radio" value="system" :checked="themeMode === 'system'" @change="setTheme('system')" />
            <div class="theme-info">
              <span class="theme-name">{{ t('settings.followSystem') }}</span>
              <span class="theme-hint">{{ t('settings.followSystemHint') }}</span>
            </div>
          </label>
          <label class="theme-option" :class="{ active: themeMode === 'light' }">
            <input type="radio" value="light" :checked="themeMode === 'light'" @change="setTheme('light')" />
            <div class="theme-info">
              <span class="theme-name">{{ t('settings.light') }}</span>
            </div>
          </label>
          <label class="theme-option" :class="{ active: themeMode === 'dark' }">
            <input type="radio" value="dark" :checked="themeMode === 'dark'" @change="setTheme('dark')" />
            <div class="theme-info">
              <span class="theme-name">{{ t('settings.dark') }}</span>
            </div>
          </label>
        </div>
      </section>

      <section id="language" class="settings-section">
        <h4 class="section-title">{{ t('settings.language') }}</h4>
        <p class="section-desc">{{ t('settings.languageHint') }}</p>
        <div class="language-options">
          <label
            v-for="loc in availableLocales"
            :key="loc.code"
            class="lang-option"
            :class="{ active: getLocale() === loc.code }"
          >
            <input
              type="radio"
              :value="loc.code"
              :checked="getLocale() === loc.code"
              @change="setLocale(loc.code); autoSave()"
            />
            <span class="lang-name">{{ loc.name }}</span>
          </label>
        </div>
      </section>

      <section id="project" class="settings-section">
        <h4 class="section-title">{{ t('settings.project') }}</h4>
        <p class="section-desc">{{ t('settings.projectDirHint') }}</p>
        <div class="dir-row">
          <button class="btn-secondary btn-sm" @click="browseProjectDir">
            {{ t('settings.projectDirBrowse') }}
          </button>
          <span v-if="projectDir" class="dir-path" :title="projectDir">
            {{ projectDir }}
          </span>
          <span v-else class="dir-placeholder">{{ t('settings.projectDirDefault') }}</span>
        </div>
      </section>

      <section id="export" class="settings-section">
        <h4 class="section-title">{{ t('settings.export') }}</h4>
        <label class="toggle-row">
          <span class="toggle-info">
            <span class="toggle-label">{{ t('settings.watermark') }}</span>
            <span class="toggle-hint">{{ t('settings.watermarkHint') }}</span>
          </span>
          <input
            type="checkbox"
            class="toggle-switch"
            :checked="settingsStore.watermarkEnabled"
            @change="settingsStore.setWatermarkEnabled(($event.target as HTMLInputElement).checked)"
          />
        </label>
      </section>

      <section id="about" class="settings-section">
        <h4 class="section-title">{{ t('settings.about') }}</h4>
        <p class="about-text">
          {{ t('settings.aboutText') }}
        </p>

        <details class="oss-section">
          <summary class="oss-toggle">{{ t('settings.openSource') }}</summary>
          <p class="section-desc">{{ t('settings.openSourceDesc') }}</p>
          <div class="oss-list">
            <a
              v-for="item in ossProjects"
              :key="item.url"
              :href="item.url"
              target="_blank"
              class="oss-item"
            >
              <span class="oss-name">{{ item.name }}</span>
              <span class="oss-category">{{ item.category }}</span>
            </a>
          </div>
        </details>
      </section>
    </div>
  </div>
</template>

<style lang="scss" scoped>
.settings-view {
  height: 100%;
  overflow-y: auto;
}

.settings-content {
  max-width: 620px;
  margin: 0 auto;
  padding: $spacing-xl;
}

.settings-header {
  margin-bottom: $spacing-lg;
}

.settings-title {
  font-size: 18px;
  font-weight: 500;
}

.settings-nav {
  display: flex;
  flex-wrap: wrap;
  gap: $spacing-xs;
  margin-bottom: $spacing-xl;
  padding-bottom: $spacing-lg;
  border-bottom: 1px solid $color-border;
  position: sticky;
  top: 0;
  background: $color-bg;
  z-index: 10;
  padding-top: $spacing-sm;
}

.nav-item {
  padding: $spacing-xs $spacing-md;
  background: none;
  border: 1px solid $color-border;
  border-radius: $radius-sm;
  font-size: 12px;
  color: $color-text-secondary;
  cursor: pointer;
  font-family: $font-family;
  transition: all $transition-fast;

  &:hover {
    border-color: $color-node-border;
    color: $color-text-primary;
    background: $color-panel;
  }
}

.settings-section {
  margin-bottom: $spacing-xl;
  scroll-margin-top: 60px;
}

.section-title {
  font-size: 14px;
  font-weight: 500;
  margin-bottom: $spacing-xs;
  color: $color-text-primary;
}

.section-desc {
  font-size: 12px;
  color: $color-text-disabled;
  margin-bottom: $spacing-lg;
}

.provider-card {
  border: 1px solid $color-border;
  border-radius: $radius-sm;
  padding: $spacing-md $spacing-lg;
  margin-bottom: $spacing-sm;
  cursor: pointer;
  transition: border-color $transition-fast;

  &.active {
    border-color: $color-node-border;
  }
}

.provider-header {
  display: flex;
  align-items: center;
  gap: $spacing-md;
}

.provider-radio {
  flex-shrink: 0;

  input[type="radio"] {
    accent-color: $color-text-primary;
    width: 14px;
    height: 14px;
    margin: 0;
    cursor: pointer;
  }
}

.provider-info {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.provider-name {
  font-size: 13px;
  font-weight: 500;
}

.provider-hint {
  font-size: 11px;
  color: $color-text-disabled;
}

.provider-fields {
  display: flex;
  flex-direction: column;
  gap: $spacing-md;
  margin-top: $spacing-md;
  padding-top: $spacing-md;
  border-top: 1px solid $color-border;
}

.field {
  display: flex;
  flex-direction: column;
  gap: $spacing-xs;

  label {
    font-size: 12px;
    color: $color-text-secondary;
  }
}

.field-hint {
  font-size: 11px;
  color: $color-text-disabled;
}

.input {
  padding: $spacing-sm $spacing-md;
  border: 1px solid $color-border;
  border-radius: $radius-sm;
  font-size: 13px;
  font-family: $font-family;
  color: $color-text-primary;
  background: $color-bg;

  &:focus {
    outline: none;
    border-color: $color-node-border;
  }

  &::placeholder {
    color: $color-text-disabled;
  }
}

.input-api-key {
  &::placeholder {
    color: $color-text-secondary;
  }

  &:not(:focus)::placeholder {
    color: $color-text-primary;
  }
}

.test-section {
  display: flex;
  align-items: center;
  gap: $spacing-md;
  margin-top: $spacing-lg;
}

.btn-secondary {
  padding: $spacing-sm $spacing-lg;
  background: $color-bg;
  color: $color-text-primary;
  border: 1px solid $color-border;
  border-radius: $radius-sm;
  cursor: pointer;
  font-size: 13px;
  font-family: $font-family;

  &:hover:not(:disabled) {
    background: $color-panel;
    border-color: $color-node-border;
  }

  &:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
}

.test-result {
  font-size: 12px;
  color: var(--color-error);

  &.success {
    color: var(--color-success);
  }
}

.ocr-options {
  display: flex;
  flex-direction: column;
  gap: $spacing-sm;
}

.ocr-option {
  display: flex;
  align-items: center;
  gap: $spacing-md;
  border: 1px solid $color-border;
  border-radius: $radius-sm;
  padding: $spacing-md $spacing-lg;
  cursor: pointer;
  transition: border-color $transition-fast;

  &.active {
    border-color: $color-node-border;
  }

  input[type="radio"] {
    accent-color: $color-text-primary;
    width: 14px;
    height: 14px;
    flex-shrink: 0;
    margin: 0;
    cursor: pointer;
  }
}

.ocr-info {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.ocr-name {
  font-size: 13px;
  font-weight: 500;
}

.ocr-hint {
  font-size: 11px;
  color: $color-text-disabled;
}

.model-options {
  display: flex;
  flex-direction: column;
  gap: $spacing-sm;
  margin-top: $spacing-md;
  padding: $spacing-lg;
  border: 1px solid $color-border;
  border-radius: $radius-sm;
}

.model-option {
  display: flex;
  align-items: center;
  gap: $spacing-md;
  padding: $spacing-sm 0;
  cursor: pointer;

  &.active .model-name {
    color: $color-text-primary;
  }

  input[type="radio"] {
    accent-color: $color-text-primary;
    width: 14px;
    height: 14px;
    flex-shrink: 0;
    margin: 0;
    cursor: pointer;
  }
}

.model-info {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.model-name {
  font-size: 13px;
  font-weight: 500;
  color: $color-text-secondary;
  display: flex;
  align-items: center;
  gap: $spacing-sm;
}

.model-hint {
  font-size: 11px;
  color: $color-text-disabled;
}

.model-badge {
  font-size: 10px;
  padding: 1px 6px;
  border-radius: 3px;
  font-weight: 400;

  &.downloaded {
    background: rgba(39, 174, 96, 0.1);
    color: var(--color-success);
  }

  &.not-downloaded {
    background: rgba(0, 0, 0, 0.05);
    color: $color-text-disabled;
  }

  &.downloading {
    background: rgba(0, 0, 0, 0.05);
    color: $color-text-secondary;
  }
}

.model-download-row {
  padding-top: $spacing-sm;
}

.theme-options {
  display: flex;
  flex-direction: column;
  gap: $spacing-sm;
}

.theme-option {
  display: flex;
  align-items: center;
  gap: $spacing-md;
  border: 1px solid $color-border;
  border-radius: $radius-sm;
  padding: $spacing-md $spacing-lg;
  cursor: pointer;
  transition: border-color $transition-fast;

  &.active {
    border-color: $color-node-border;
  }

  input[type="radio"] {
    accent-color: $color-text-primary;
    width: 14px;
    height: 14px;
    flex-shrink: 0;
    margin: 0;
    cursor: pointer;
  }
}

.theme-info {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.theme-name {
  font-size: 13px;
  font-weight: 500;
}

.theme-hint {
  font-size: 11px;
  color: $color-text-disabled;
}

.language-options {
  display: flex;
  flex-direction: column;
  gap: $spacing-sm;
}

.lang-option {
  display: flex;
  align-items: center;
  gap: $spacing-md;
  border: 1px solid $color-border;
  border-radius: $radius-sm;
  padding: $spacing-md $spacing-lg;
  cursor: pointer;
  transition: border-color $transition-fast;

  &.active {
    border-color: $color-node-border;
  }

  input[type="radio"] {
    accent-color: $color-text-primary;
    width: 14px;
    height: 14px;
    flex-shrink: 0;
    margin: 0;
    cursor: pointer;
  }
}

.lang-name {
  font-size: 13px;
  font-weight: 500;
}

.mineru-fields {
  display: flex;
  flex-direction: column;
  gap: $spacing-md;
  margin-top: $spacing-md;
  padding: $spacing-lg;
  border: 1px solid $color-border;
  border-radius: $radius-sm;
}

.embedding-fields {
  display: flex;
  flex-direction: column;
  gap: $spacing-md;
}

.dir-row {
  display: flex;
  align-items: center;
  gap: $spacing-sm;
}

.btn-sm {
  padding: 4px 10px;
  font-size: 12px;
  flex-shrink: 0;
}

.dir-path {
  font-size: 12px;
  color: $color-text-secondary;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.dir-placeholder {
  font-size: 12px;
  color: $color-text-disabled;
}

.toggle-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: $spacing-md $spacing-lg;
  border: 1px solid $color-border;
  border-radius: $radius-sm;
  cursor: pointer;

  &:hover {
    background: $color-panel;
  }
}

.toggle-info {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.toggle-label {
  font-size: 13px;
  color: $color-text-primary;
}

.toggle-hint {
  font-size: 11px;
  color: $color-text-disabled;
}

.toggle-switch {
  flex-shrink: 0;
  width: 36px;
  height: 20px;
  -webkit-appearance: none;
  appearance: none;
  background: $color-border;
  border-radius: 10px;
  position: relative;
  cursor: pointer;
  outline: none;
  transition: background 0.2s;

  &:checked {
    background: $color-text-primary;
  }

  &::before {
    content: '';
    position: absolute;
    top: 2px;
    left: 2px;
    width: 16px;
    height: 16px;
    background: $color-bg;
    border-radius: 50%;
    transition: transform 0.2s;
  }

  &:checked::before {
    transform: translateX(16px);
  }
}

.about-text {
  color: $color-text-secondary;
  font-size: 13px;
}

.http-api-info {
  margin-top: $spacing-md;
  display: flex;
  flex-direction: column;
  gap: $spacing-md;
}

.port-input {
  width: 120px;
}

.http-api-links {
  display: flex;
  gap: $spacing-md;
}

.http-link {
  font-size: 12px;
  color: $color-text-secondary;
  text-decoration: none;
  padding: $spacing-xs $spacing-sm;
  border: 1px solid $color-border;
  border-radius: $radius-sm;

  &:hover {
    color: $color-text-primary;
    border-color: $color-node-border;
    background: $color-panel;
  }
}

.advanced-section {
  summary {
    cursor: pointer;
    list-style: none;

    &::-webkit-details-marker {
      display: none;
    }

    &::before {
      content: '\25B8';
      display: inline-block;
      margin-right: $spacing-xs;
      font-size: 11px;
      transition: transform 0.2s;
    }
  }

  &[open] summary::before {
    transform: rotate(90deg);
  }
}

.advanced-fields {
  display: flex;
  flex-direction: column;
  gap: $spacing-md;
  margin-top: $spacing-md;
}

.oss-section {
  margin-top: $spacing-lg;

  summary {
    cursor: pointer;
    list-style: none;
    font-size: 13px;
    color: $color-text-secondary;
    padding: $spacing-sm 0;

    &::-webkit-details-marker {
      display: none;
    }

    &::before {
      content: '\25B8';
      display: inline-block;
      margin-right: $spacing-xs;
      font-size: 11px;
      transition: transform 0.2s;
    }
  }

  &[open] summary::before {
    transform: rotate(90deg);
  }
}

.oss-toggle {
  user-select: none;

  &:hover {
    color: $color-text-primary;
  }
}

.oss-list {
  display: flex;
  flex-direction: column;
  gap: 1px;
  margin-top: $spacing-sm;
  border: 1px solid $color-border;
  border-radius: $radius-sm;
  overflow: hidden;
}

.oss-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: $spacing-sm $spacing-md;
  text-decoration: none;
  color: $color-text-primary;
  background: $color-panel;
  font-size: 12px;

  &:hover {
    background: $color-border;
  }
}

.oss-name {
  font-weight: 500;
}

.oss-category {
  font-size: 11px;
  color: $color-text-disabled;
}
</style>
