use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiProviderConfig {
    pub enabled: bool,
    pub api_key: String,
    pub base_url: Option<String>,
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum OcrMethod {
    Local,
    Mineru,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum OcrModelMode {
    Mobile,
    Server,
}

impl Default for OcrModelMode {
    fn default() -> Self {
        Self::Mobile
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MineruConfig {
    pub api_key: String,
    pub api_base: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdvancedSettings {
    /// Number of concurrent paper parses (1-5, default 3)
    #[serde(default = "default_concurrency")]
    pub concurrency: u8,
    /// Whether to auto-parse papers after import (default true)
    #[serde(default = "default_true")]
    pub auto_parse: bool,
    /// Number of retries on parse failure (0-3, default 1)
    #[serde(default = "default_retry_count")]
    pub retry_count: u8,
}

fn default_concurrency() -> u8 { 3 }
fn default_true() -> bool { true }
fn default_retry_count() -> u8 { 1 }

impl Default for AdvancedSettings {
    fn default() -> Self {
        Self {
            concurrency: 3,
            auto_parse: true,
            retry_count: 1,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiSettings {
    /// Which AI provider is active (None = no AI)
    pub openai_compatible: Option<AiProviderConfig>,
    pub anthropic: Option<AiProviderConfig>,
    /// Which provider to use: "openaiCompatible" | "anthropic" | null
    pub active_provider: Option<String>,
    /// OCR method for text extraction
    pub ocr_method: OcrMethod,
    /// OCR model mode: Mobile (built-in, fast) or Server (downloaded, high-precision)
    #[serde(default)]
    pub ocr_model_mode: OcrModelMode,
    /// MinerU API config
    pub mineru: Option<MineruConfig>,
    /// Embedding model name (optional, defaults to provider's default)
    #[serde(default)]
    pub embedding_model: Option<String>,
    /// Embedding base URL (optional, defaults to chat provider's base URL)
    #[serde(default)]
    pub embedding_base_url: Option<String>,
    /// Embedding API key (optional, defaults to chat provider's key)
    #[serde(default)]
    pub embedding_api_key: Option<String>,
    /// User's UI locale (e.g. "zh-CN", "en", "eo"), affects AI response language
    #[serde(default)]
    pub locale: Option<String>,
    /// Default directory for new projects
    #[serde(default)]
    pub default_project_dir: Option<String>,
    /// Whether the HTTP API server is enabled
    #[serde(default)]
    pub http_api_enabled: bool,
    /// Port for the HTTP API server
    #[serde(default = "default_http_port")]
    pub http_api_port: u16,
    /// Advanced processing settings
    #[serde(default)]
    pub advanced: Option<AdvancedSettings>,
}

fn default_http_port() -> u16 {
    17800
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MaskedAiProviderConfig {
    pub enabled: bool,
    pub masked_api_key: String,
    pub base_url: Option<String>,
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MaskedMineruConfig {
    pub masked_api_key: String,
    pub api_base: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MaskedAiSettings {
    pub openai_compatible: Option<MaskedAiProviderConfig>,
    pub anthropic: Option<MaskedAiProviderConfig>,
    pub active_provider: Option<String>,
    pub ocr_method: OcrMethod,
    pub ocr_model_mode: OcrModelMode,
    pub mineru: Option<MaskedMineruConfig>,
    pub embedding_model: Option<String>,
    pub embedding_base_url: Option<String>,
    pub masked_embedding_api_key: Option<String>,
    pub default_project_dir: Option<String>,
    pub http_api_enabled: bool,
    pub http_api_port: u16,
    pub advanced: Option<AdvancedSettings>,
}

impl AiProviderConfig {
    pub fn mask_key(&self) -> String {
        let key = &self.api_key;
        if key.len() <= 16 {
            return "****".to_string();
        }
        let mask_len = 10.min(key.len() / 3);
        let prefix_len = (key.len() - mask_len) / 2;
        let suffix_len = key.len() - mask_len - prefix_len;
        let stars: String = "*".repeat(mask_len);
        format!("{}{}{}", &key[..prefix_len], stars, &key[key.len() - suffix_len..])
    }

    pub fn to_masked(&self) -> MaskedAiProviderConfig {
        MaskedAiProviderConfig {
            enabled: self.enabled,
            masked_api_key: self.mask_key(),
            base_url: self.base_url.clone(),
            model: self.model.clone(),
        }
    }
}

impl MineruConfig {
    pub fn mask_key(&self) -> String {
        let key = &self.api_key;
        if key.len() <= 16 {
            return "****".to_string();
        }
        let mask_len = 10.min(key.len() / 3);
        let prefix_len = (key.len() - mask_len) / 2;
        let suffix_len = key.len() - mask_len - prefix_len;
        let stars: String = "*".repeat(mask_len);
        format!("{}{}{}", &key[..prefix_len], stars, &key[key.len() - suffix_len..])
    }

    pub fn to_masked(&self) -> MaskedMineruConfig {
        MaskedMineruConfig {
            masked_api_key: self.mask_key(),
            api_base: self.api_base.clone(),
        }
    }
}
