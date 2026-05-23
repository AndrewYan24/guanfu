<script setup lang="ts">
import { ref, nextTick, onMounted, onUnmounted, watch, computed } from 'vue';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { useProjectStore } from '@/stores/projectStore';
import { useChatStore } from '@/stores/chatStore';
import { useI18n } from 'vue-i18n';
import { marked } from 'marked';

const { t } = useI18n();
const projectStore = useProjectStore();
const chatStore = useChatStore();

const inputText = ref('');
const messagesContainer = ref<HTMLElement | null>(null);
const showSources = ref<Record<number, boolean>>({});
let streamUnlisten: UnlistenFn | null = null;

marked.setOptions({
  gfm: true,
  breaks: true,
});

onMounted(() => {
  if (projectStore.projectPath) {
    chatStore.loadHistory(projectStore.projectPath);
  }
});

onUnmounted(() => {
  streamUnlisten?.();
});

watch(() => projectStore.projectPath, (path) => {
  if (path) {
    chatStore.loadHistory(path);
  } else {
    chatStore.reset();
  }
});

watch(() => chatStore.messages.length + (chatStore.isStreaming ? 1 : 0), () => {
  nextTick(() => {
    scrollToBottom();
  });
});

watch(() => chatStore.streamingContent, () => {
  nextTick(() => {
    scrollToBottom();
  });
});

function scrollToBottom() {
  if (messagesContainer.value) {
    messagesContainer.value.scrollTop = messagesContainer.value.scrollHeight;
  }
}

async function handleSend() {
  const question = inputText.value.trim();
  if (!question || chatStore.isLoading || !projectStore.projectPath) return;
  inputText.value = '';

  // Show user message immediately
  chatStore.messages.push({
    role: 'user',
    content: question,
    sources: [],
    createdAt: new Date().toISOString(),
  });

  await nextTick();
  scrollToBottom();

  // Set up streaming listener
  streamUnlisten = await listen<{ content: string }>('chat-stream', (event) => {
    chatStore.appendStreamContent(event.payload.content);
  });

  try {
    await chatStore.askStream(projectStore.projectPath, question);
  } finally {
    streamUnlisten?.();
    streamUnlisten = null;
  }
}

function handleKeydown(e: KeyboardEvent) {
  if (e.key === 'Enter' && !e.shiftKey) {
    e.preventDefault();
    handleSend();
  }
}

async function handleBuild() {
  if (!projectStore.projectPath) return;
  await chatStore.buildEmbeddings(projectStore.projectPath);
}

function toggleSources(index: number) {
  showSources.value[index] = !showSources.value[index];
}

function handleClear() {
  chatStore.clearHistory();
}

function formatTime(iso: string): string {
  try {
    const d = new Date(iso);
    return d.toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' });
  } catch {
    return '';
  }
}

function renderMarkdown(text: string): string {
  try {
    return marked.parse(text) as string;
  } catch {
    return text;
  }
}

const renderedStreamingContent = computed(() => {
  if (!chatStore.streamingContent) return '';
  return renderMarkdown(chatStore.streamingContent);
});
</script>

<template>
  <div class="chat-view">
    <div v-if="!projectStore.hasProject" class="placeholder">
      <p>{{ t('chat.noProject') }}</p>
    </div>
    <div v-else class="chat-container">
      <div class="chat-header">
        <h3>{{ t('chat.title') }}</h3>
        <button
          v-if="chatStore.messages.length > 0"
          class="btn-clear"
          @click="handleClear"
        >
          {{ t('chat.clearHistory') }}
        </button>
      </div>

      <div ref="messagesContainer" class="messages-area">
        <!-- Empty state -->
        <div v-if="chatStore.messages.length === 0 && !chatStore.isLoading" class="empty-state">
          <div v-if="chatStore.embeddingStatus !== 'built'" class="build-prompt">
            <p>{{ t('chat.firstUseHint') }}</p>
            <p class="build-hint">{{ t('chat.buildHint') }}</p>
            <button
              class="btn-build"
              :disabled="chatStore.isBuilding"
              @click="handleBuild"
            >
              {{ chatStore.isBuilding ? t('chat.building') : t('chat.buildIndex') }}
            </button>
            <p v-if="chatStore.buildError" class="build-error">{{ chatStore.buildError }}</p>
          </div>
          <div v-else class="welcome-text">
            <p>{{ t('chat.indexReady') }}</p>
            <p class="build-hint">{{ t('chat.indexHint') }}</p>
          </div>
        </div>

        <!-- Messages -->
        <div
          v-for="(msg, index) in chatStore.messages"
          :key="index"
          class="message"
          :class="[msg.role]"
        >
          <div class="message-bubble">
            <div
              v-if="msg.role === 'user'"
              class="message-content"
            >{{ msg.content }}</div>
            <div
              v-else
              class="message-content markdown-body"
              v-html="renderMarkdown(msg.content)"
            />

            <!-- Sources for assistant messages -->
            <div v-if="msg.role === 'assistant' && msg.sources.length > 0" class="sources-section">
              <button class="sources-toggle" @click="toggleSources(index)">
                {{ showSources[index] ? t('chat.hideSources') : t('chat.sourceCount', { count: msg.sources.length }) }}
              </button>
              <div v-if="showSources[index]" class="sources-list">
                <div
                  v-for="(source, si) in msg.sources"
                  :key="si"
                  class="source-item"
                >
                  <span class="source-title">{{ source.paperTitle }}</span>
                  <span class="source-similarity">{{ (source.similarity * 100).toFixed(0) }}%</span>
                  <p class="source-text">{{ source.chunkText }}</p>
                </div>
              </div>
            </div>
          </div>
          <span class="message-time">{{ formatTime(msg.createdAt) }}</span>
        </div>

        <!-- Streaming message -->
        <div v-if="chatStore.isStreaming" class="message assistant">
          <div class="message-bubble">
            <!-- Typing indicator while waiting for first chunk -->
            <div v-if="!chatStore.streamingContent" class="typing-indicator">
              <span></span>
              <span></span>
              <span></span>
            </div>
            <!-- Streaming content with markdown -->
            <div
              v-else
              class="message-content markdown-body"
              v-html="renderedStreamingContent"
            />
          </div>
        </div>
      </div>

      <!-- Input area -->
      <div class="input-area">
        <div v-if="chatStore.embeddingStatus === 'none'" class="input-disabled-hint">
          {{ t('chat.needIndex') }}
        </div>
        <div class="input-row" :class="{ disabled: chatStore.embeddingStatus !== 'built' && chatStore.messages.length === 0 }">
          <textarea
            v-model="inputText"
            class="chat-input"
            :placeholder="t('chat.inputPlaceholder')"
            rows="1"
            :disabled="chatStore.isLoading || (chatStore.embeddingStatus !== 'built' && chatStore.messages.length === 0)"
            @keydown="handleKeydown"
          />
          <button
            class="btn-send"
            :disabled="!inputText.trim() || chatStore.isLoading || (chatStore.embeddingStatus !== 'built' && chatStore.messages.length === 0)"
            @click="handleSend"
          >
            <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
              <path d="M2 8l12-6-4 6 4 6L2 8z" fill="currentColor"/>
            </svg>
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<style lang="scss" scoped>
.chat-view {
  height: 100%;
  display: flex;
  flex-direction: column;
}

.placeholder {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 100%;
  color: $color-text-disabled;
}

.chat-container {
  display: flex;
  flex-direction: column;
  height: 100%;
  max-width: 720px;
  margin: 0 auto;
  width: 100%;
}

.chat-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: $spacing-lg $spacing-xl;
  border-bottom: 1px solid $color-border;
  flex-shrink: 0;

  h3 {
    font-size: 16px;
    font-weight: 500;
  }
}

.btn-clear {
  background: none;
  border: 1px solid $color-border;
  border-radius: $radius-sm;
  padding: $spacing-xs $spacing-md;
  font-size: 12px;
  color: $color-text-secondary;
  cursor: pointer;
  font-family: $font-family;

  &:hover {
    background: $color-panel;
    border-color: $color-node-border;
  }
}

.messages-area {
  flex: 1;
  overflow-y: auto;
  padding: $spacing-xl;
  display: flex;
  flex-direction: column;
  gap: $spacing-lg;
}

.empty-state {
  display: flex;
  align-items: center;
  justify-content: center;
  flex: 1;
  text-align: center;
}

.build-prompt {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: $spacing-sm;

  p {
    font-size: 14px;
    color: $color-text-primary;
  }
}

.welcome-text {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: $spacing-sm;

  p {
    font-size: 14px;
    color: $color-text-secondary;
  }
}

.build-hint {
  font-size: 12px !important;
  color: $color-text-disabled !important;
  max-width: 320px;
}

.build-error {
  font-size: 12px !important;
  color: var(--color-error) !important;
  max-width: 360px;
  word-break: break-word;
}

.btn-build {
  margin-top: $spacing-sm;
  padding: $spacing-sm $spacing-xl;
  background: $color-text-primary;
  color: $color-bg;
  border: none;
  border-radius: $radius-sm;
  font-size: 13px;
  cursor: pointer;
  font-family: $font-family;

  &:hover:not(:disabled) {
    opacity: 0.85;
  }

  &:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
}

.message {
  display: flex;
  flex-direction: column;
  gap: 4px;

  &.user {
    align-items: flex-end;

    .message-bubble {
      background: $color-text-primary;
      color: $color-bg;
      border-radius: $radius-md $radius-md 0 $radius-md;
    }
  }

  &.assistant {
    align-items: flex-start;

    .message-bubble {
      background: $color-panel;
      border: 1px solid $color-border;
      border-radius: $radius-md $radius-md $radius-md 0;
    }
  }
}

.message-bubble {
  max-width: 85%;
  padding: $spacing-md $spacing-lg;
}

.message-content {
  font-size: 13px;
  line-height: 1.7;
  white-space: pre-wrap;
  word-break: break-word;
}

.message-time {
  font-size: 11px;
  color: $color-text-disabled;
  padding: 0 $spacing-xs;
}

.sources-section {
  margin-top: $spacing-sm;
  border-top: 1px solid $color-border;
  padding-top: $spacing-sm;
}

.sources-toggle {
  background: none;
  border: none;
  font-size: 11px;
  color: $color-text-secondary;
  cursor: pointer;
  padding: 2px 0;
  font-family: $font-family;

  &:hover {
    color: $color-text-primary;
  }
}

.sources-list {
  display: flex;
  flex-direction: column;
  gap: $spacing-sm;
  margin-top: $spacing-sm;
}

.source-item {
  padding: $spacing-sm;
  background: $color-bg;
  border: 1px solid $color-border;
  border-radius: $radius-sm;
}

.source-title {
  font-size: 12px;
  font-weight: 500;
  color: $color-text-primary;
}

.source-similarity {
  font-size: 11px;
  color: $color-text-disabled;
  margin-left: $spacing-sm;
}

.source-text {
  font-size: 11px;
  color: $color-text-secondary;
  margin-top: 4px;
  line-height: 1.5;
  display: -webkit-box;
  -webkit-line-clamp: 3;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

.typing-indicator {
  display: flex;
  gap: 4px;
  padding: 4px 0;

  span {
    width: 6px;
    height: 6px;
    background: $color-text-disabled;
    border-radius: 50%;
    animation: typing 1.2s infinite;

    &:nth-child(2) { animation-delay: 0.2s; }
    &:nth-child(3) { animation-delay: 0.4s; }
  }
}

@keyframes typing {
  0%, 60%, 100% { opacity: 0.3; transform: scale(0.8); }
  30% { opacity: 1; transform: scale(1); }
}

.input-area {
  flex-shrink: 0;
  padding: $spacing-md $spacing-xl $spacing-xl;
  border-top: 1px solid $color-border;
}

.input-disabled-hint {
  text-align: center;
  font-size: 12px;
  color: $color-text-disabled;
  margin-bottom: $spacing-sm;
}

.input-row {
  display: flex;
  gap: $spacing-sm;
  align-items: flex-end;

  &.disabled {
    opacity: 0.5;
  }
}

.chat-input {
  flex: 1;
  resize: none;
  padding: $spacing-sm $spacing-md;
  border: 1px solid $color-border;
  border-radius: $radius-sm;
  font-size: 13px;
  font-family: $font-family;
  color: $color-text-primary;
  background: $color-bg;
  line-height: 1.5;
  max-height: 120px;

  &:focus {
    outline: none;
    border-color: $color-node-border;
  }

  &::placeholder {
    color: $color-text-disabled;
  }
}

.btn-send {
  width: 36px;
  height: 36px;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  background: $color-text-primary;
  color: $color-bg;
  border: none;
  border-radius: $radius-sm;
  cursor: pointer;

  &:hover:not(:disabled) {
    opacity: 0.85;
  }

  &:disabled {
    opacity: 0.3;
    cursor: not-allowed;
  }
}

// Markdown body styles
.markdown-body {
  white-space: normal;

  :deep(h1), :deep(h2), :deep(h3), :deep(h4) {
    font-size: 14px;
    font-weight: 600;
    margin: $spacing-sm 0 $spacing-xs;
  }

  :deep(p) {
    margin: 0 0 $spacing-sm;

    &:last-child {
      margin-bottom: 0;
    }
  }

  :deep(ul), :deep(ol) {
    margin: 0 0 $spacing-sm;
    padding-left: $spacing-lg;
  }

  :deep(li) {
    margin-bottom: 2px;
  }

  :deep(code) {
    font-family: 'SF Mono', Menlo, Consolas, monospace;
    font-size: 12px;
    background: rgba(0, 0, 0, 0.06);
    padding: 1px 4px;
    border-radius: 2px;
  }

  :deep(pre) {
    background: rgba(0, 0, 0, 0.04);
    padding: $spacing-sm;
    border-radius: $radius-sm;
    overflow-x: auto;
    margin: $spacing-sm 0;

    code {
      background: none;
      padding: 0;
    }
  }

  :deep(blockquote) {
    border-left: 2px solid $color-border;
    padding-left: $spacing-md;
    margin: $spacing-sm 0;
    color: $color-text-secondary;
  }

  :deep(table) {
    border-collapse: collapse;
    margin: $spacing-sm 0;
    font-size: 12px;
  }

  :deep(th), :deep(td) {
    border: 1px solid $color-border;
    padding: $spacing-xs $spacing-sm;
    text-align: left;
  }

  :deep(th) {
    background: rgba(0, 0, 0, 0.04);
    font-weight: 500;
  }

  :deep(strong) {
    font-weight: 600;
  }

  :deep(hr) {
    border: none;
    border-top: 1px solid $color-border;
    margin: $spacing-md 0;
  }
}
</style>
