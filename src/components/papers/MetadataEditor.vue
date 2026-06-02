<script setup lang="ts">
import { ref, computed, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import type { Paper } from '@/types';
import { usePaperStore } from '@/stores/paperStore';
import { useProjectStore } from '@/stores/projectStore';
import { aiParsePdf } from '@/api/aiApi';

const props = defineProps<{
  paper: Paper;
}>();

const paperStore = usePaperStore();
const projectStore = useProjectStore();
const { t } = useI18n();

interface FieldDef {
  key: string;
  label: string;
  multiline?: boolean;
}

const fields = computed<FieldDef[]>(() => [
  { key: 'researchQuestion', label: t('metadata.researchQuestion'), multiline: true },
  { key: 'coreClaim', label: t('metadata.coreClaim'), multiline: true },
  { key: 'assumptions', label: t('metadata.assumptions'), multiline: true },
  { key: 'theoreticalPerspective', label: t('metadata.theoreticalPerspective') },
  { key: 'methodology', label: t('metadata.methodology') },
  { key: 'findings', label: t('metadata.findings'), multiline: true },
  { key: 'limitations', label: t('metadata.limitations'), multiline: true },
  { key: 'selfPositioning', label: t('metadata.selfPositioning'), multiline: true },
]);

const formData = ref<Record<string, string>>({});
const hasChanges = ref(false);
const isParsing = ref(false);
const parseStatus = ref('');

const isAutoParsing = computed(() => paperStore.isPaperParsing(props.paper.id));
const isQueued = computed(() => paperStore.isPaperQueued(props.paper.id));

function loadForm() {
  const meta = props.paper.metadata as unknown as Record<string, string> | undefined;
  formData.value = {};
  for (const f of fields.value) {
    formData.value[f.key] = meta?.[f.key] ?? '';
  }
  hasChanges.value = false;
}

watch(() => props.paper.id, loadForm, { immediate: true });

// Auto-reload form when metadata changes (e.g. after import analysis completes)
watch(() => props.paper.metadata, () => {
  if (!hasChanges.value && !isParsing.value) {
    loadForm();
  }
}, { deep: true });

function handleFieldChange() {
  hasChanges.value = true;
}

async function handleSave() {
  const now = new Date().toISOString();
  const meta: Record<string, unknown> = {
    researchQuestion: formData.value.researchQuestion || '',
    coreClaim: formData.value.coreClaim || '',
    assumptions: formData.value.assumptions || '',
    theoreticalPerspective: formData.value.theoreticalPerspective || '',
    methodology: formData.value.methodology || '',
    findings: formData.value.findings || '',
    limitations: formData.value.limitations || '',
    selfPositioning: formData.value.selfPositioning || '',
    version: (props.paper.metadata?.version ?? 0) + 1,
    lastUpdated: now,
    source: props.paper.metadata?.source ?? 'manual',
    isAiGenerated: props.paper.metadata?.isAiGenerated,
  };

  const updatedPaper = { ...props.paper, metadata: meta as unknown as typeof props.paper.metadata };
  await paperStore.updatePaper(updatedPaper);
  hasChanges.value = false;
}

async function handleParse() {
  if (!projectStore.projectPath || isParsing.value) return;
  isParsing.value = true;
  parseStatus.value = t('metadata.discovering');
  try {
    const metadata = await aiParsePdf(projectStore.projectPath, props.paper.id);
    formData.value.researchQuestion = metadata.researchQuestion;
    formData.value.coreClaim = metadata.coreClaim;
    formData.value.assumptions = metadata.assumptions;
    formData.value.theoreticalPerspective = metadata.theoreticalPerspective;
    formData.value.methodology = metadata.methodology;
    formData.value.findings = metadata.findings;
    formData.value.limitations = metadata.limitations;
    formData.value.selfPositioning = metadata.selfPositioning;

    // Auto-save
    const now = new Date().toISOString();
    const meta: Record<string, unknown> = {
      ...formData.value,
      version: (props.paper.metadata?.version ?? 0) + 1,
      lastUpdated: now,
      source: 'ai',
      isAiGenerated: true,
    };
    const updatedPaper = { ...props.paper, metadata: meta as unknown as typeof props.paper.metadata };
    await paperStore.updatePaper(updatedPaper);

    parseStatus.value = t('common.saved');
    hasChanges.value = false;
  } catch (e) {
    parseStatus.value = e instanceof Error ? e.message : t('common.saveFailed');
  } finally {
    isParsing.value = false;
  }
}
</script>

<template>
  <div class="metadata-editor">
    <div class="editor-header">
      <div class="header-actions">
        <button
          class="action-btn"
          :disabled="isParsing || isAutoParsing || isQueued"
          @click="handleParse"
        >
          {{ (isParsing || isAutoParsing) ? t('metadata.discovering') : isQueued ? t('metadata.queued') : t('metadata.reDiscover') }}
        </button>
        <button
          v-if="hasChanges"
          class="save-btn"
          @click="handleSave"
        >
          {{ t('common.save') }}
        </button>
      </div>
    </div>

    <div v-if="parseStatus || isAutoParsing || isQueued" class="parse-status">
      <span v-if="isQueued" class="status-queued">{{ t('metadata.queued') }}</span>
      <span v-else-if="isAutoParsing" class="status-parsing">
        <span class="spinner" />
        {{ t('metadata.discovering') }}
      </span>
      <span v-else>{{ parseStatus }}</span>
    </div>

    <div class="fields">
      <div
        v-for="field in fields"
        :key="field.key"
        class="field"
      >
        <label class="field-label">{{ field.label }}</label>
        <textarea
          v-if="field.multiline"
          v-model="formData[field.key]"
          class="field-input multiline"
          :placeholder="field.label + '...'"
          @input="handleFieldChange"
        />
        <input
          v-else
          v-model="formData[field.key]"
          class="field-input"
          :placeholder="field.label + '...'"
          @input="handleFieldChange"
        />
      </div>
    </div>
  </div>
</template>

<style lang="scss" scoped>
.metadata-editor {
  display: flex;
  flex-direction: column;
  gap: $spacing-lg;
}

.editor-header {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  flex-wrap: wrap;
  gap: $spacing-sm;
}

.header-actions {
  display: flex;
  align-items: center;
  gap: $spacing-sm;
}

.action-btn {
  padding: 4px 10px;
  border: 1px solid $color-border;
  border-radius: $radius-sm;
  background: $color-bg;
  color: $color-text-primary;
  font-size: 12px;
  cursor: pointer;
  font-family: $font-family;
  transition: all $transition-fast;

  &:hover:not(:disabled) {
    background: $color-panel;
    border-color: $color-node-border;
  }

  &:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
}

.save-btn {
  padding: 4px 12px;
  background: $color-text-primary;
  color: $color-bg;
  border: none;
  border-radius: $radius-sm;
  font-size: 12px;
  cursor: pointer;
  font-family: $font-family;

  &:hover {
    opacity: 0.85;
  }
}

.parse-status {
  font-size: 12px;
  color: $color-text-secondary;
  padding: $spacing-sm;
  background: $color-panel;
  border-radius: $radius-sm;
}

.status-queued {
  color: $color-text-disabled;
}

.status-parsing {
  display: flex;
  align-items: center;
  gap: $spacing-sm;
}

.spinner {
  width: 12px;
  height: 12px;
  border: 1.5px solid $color-border;
  border-top-color: $color-text-secondary;
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
  flex-shrink: 0;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.fields {
  display: flex;
  flex-direction: column;
  gap: $spacing-lg;
}

.field {
  display: flex;
  flex-direction: column;
  gap: $spacing-xs;
}

.field-label {
  font-size: 12px;
  font-weight: 500;
  color: $color-text-secondary;
}

.field-input {
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

.multiline {
  min-height: 80px;
  resize: vertical;
  line-height: 1.6;
}
</style>
