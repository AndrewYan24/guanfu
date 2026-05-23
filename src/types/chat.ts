export interface ChatMessage {
  role: 'user' | 'assistant';
  content: string;
  sources: ChatSource[];
  createdAt: string;
}

export interface ChatSource {
  paperId: string;
  paperTitle: string;
  chunkText: string;
  similarity: number;
}
