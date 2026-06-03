export interface AiProviderConfig {
  enabled: boolean;
  apiKey: string;
  baseUrl?: string;
  model: string;
}

export type OcrMethod = 'local' | 'mineru' | 'agent';

export interface MineruConfig {
  apiKey: string;
  apiBase: string;
}

export interface AdvancedSettings {
  concurrency: number;   // 1-5, default 3
  autoParse: boolean;    // default true
  retryCount: number;    // 0-3, default 1
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
  httpApiEnabled?: boolean;
  httpApiPort?: number;
  advanced?: AdvancedSettings;
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
  httpApiEnabled: boolean;
  httpApiPort: number;
  advanced?: AdvancedSettings;
}
