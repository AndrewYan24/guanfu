export interface AiProviderConfig {
  enabled: boolean;
  apiKey: string;
  baseUrl?: string;
  model: string;
}

export type OcrMethod = 'local' | 'mineru';

export interface MineruConfig {
  apiKey: string;
  apiBase: string;
}

export interface AiSettings {
  openaiCompatible?: AiProviderConfig;
  anthropic?: AiProviderConfig;
  activeProvider?: 'openaiCompatible' | 'anthropic' | null;
  ocrMethod?: OcrMethod;
  mineru?: MineruConfig;
  embeddingModel?: string;
  embeddingBaseUrl?: string;
  embeddingApiKey?: string;
  locale?: string;
  defaultProjectDir?: string;
}

export interface MaskedAiProviderConfig {
  enabled: boolean;
  maskedApiKey: string;
  baseUrl?: string;
  model: string;
}

export interface MaskedMineruConfig {
  maskedApiKey: string;
  apiBase: string;
}

export interface MaskedAiSettings {
  openaiCompatible?: MaskedAiProviderConfig;
  anthropic?: MaskedAiProviderConfig;
  activeProvider?: 'openaiCompatible' | 'anthropic' | null;
  ocrMethod: OcrMethod;
  mineru?: MaskedMineruConfig;
  embeddingModel?: string;
  embeddingBaseUrl?: string;
  maskedEmbeddingApiKey?: string;
  defaultProjectDir?: string;
}
