import { safeInvoke } from './tauriClient';
import type { ChatMessage } from '@/types';

export function chatAsk(projectPath: string, question: string) {
  return safeInvoke<ChatMessage>('chat_ask', { projectPath, question });
}

export function chatAskStream(projectPath: string, question: string) {
  return safeInvoke<ChatMessage>('chat_ask_stream', { projectPath, question });
}

export function chatBuildEmbeddings(projectPath: string) {
  return safeInvoke<number>('chat_build_embeddings', { projectPath });
}

export function getChatHistory(projectPath: string) {
  return safeInvoke<ChatMessage[]>('get_chat_history', { projectPath });
}
