import { defineStore } from 'pinia';
import { ref } from 'vue';
import type { ChatMessage } from '@/types';
import * as chatApi from '@/api/chatApi';

export const useChatStore = defineStore('chat', () => {
  const messages = ref<ChatMessage[]>([]);
  const isLoading = ref(false);
  const isBuilding = ref(false);
  const buildError = ref('');
  const embeddingStatus = ref<'unknown' | 'none' | 'built'>('unknown');
  const isStreaming = ref(false);
  const streamingContent = ref('');

  async function ask(projectPath: string, question: string) {
    if (!projectPath || !question.trim()) return;
    isLoading.value = true;
    try {
      const answer = await chatApi.chatAsk(projectPath, question);
      // The backend saves both user and assistant messages.
      // Reload history to get the full list with proper timestamps.
      messages.value = await chatApi.getChatHistory(projectPath);
      embeddingStatus.value = 'built';
      return answer;
    } finally {
      isLoading.value = false;
    }
  }

  async function askStream(projectPath: string, question: string): Promise<void> {
    if (!projectPath || !question.trim()) return;
    isLoading.value = true;
    isStreaming.value = true;
    streamingContent.value = '';
    try {
      await chatApi.chatAskStream(projectPath, question);
      embeddingStatus.value = 'built';
      // Reload to get the final message with sources
      messages.value = await chatApi.getChatHistory(projectPath);
    } finally {
      isLoading.value = false;
      isStreaming.value = false;
      streamingContent.value = '';
    }
  }

  function appendStreamContent(chunk: string) {
    streamingContent.value += chunk;
  }

  async function buildEmbeddings(projectPath: string) {
    if (!projectPath) return;
    isBuilding.value = true;
    buildError.value = '';
    try {
      const count = await chatApi.chatBuildEmbeddings(projectPath);
      embeddingStatus.value = 'built';
      return count;
    } catch (e: unknown) {
      buildError.value = e instanceof Error ? e.message : String(e);
      throw e;
    } finally {
      isBuilding.value = false;
    }
  }

  async function autoSyncEmbeddings(projectPath: string) {
    if (!projectPath || embeddingStatus.value !== 'built') return;
    try {
      await chatApi.chatBuildEmbeddings(projectPath);
    } catch {
      // 静默失败，用户在对话页面可以看到错误
    }
  }

  async function loadHistory(projectPath: string) {
    if (!projectPath) return;
    try {
      messages.value = await chatApi.getChatHistory(projectPath);
    } catch {
      messages.value = [];
    }
  }

  function clearHistory() {
    messages.value = [];
  }

  function reset() {
    messages.value = [];
    embeddingStatus.value = 'unknown';
    buildError.value = '';
    isStreaming.value = false;
    streamingContent.value = '';
  }

  return {
    messages,
    isLoading,
    isBuilding,
    buildError,
    embeddingStatus,
    isStreaming,
    streamingContent,
    ask,
    askStream,
    appendStreamContent,
    buildEmbeddings,
    autoSyncEmbeddings,
    loadHistory,
    clearHistory,
    reset,
  };
});
