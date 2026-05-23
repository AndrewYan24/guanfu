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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MineruConfig {
    pub api_key: String,
    pub api_base: String,
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
    pub mineru: Option<MaskedMineruConfig>,
    pub embedding_model: Option<String>,
    pub embedding_base_url: Option<String>,
    pub masked_embedding_api_key: Option<String>,
    pub default_project_dir: Option<String>,
}

impl AiProviderConfig {
    pub fn mask_key(&self) -> String {
        let key = &self.api_key;
        if key.len() <= 8 {
            return "****".to_string();
        }
        format!("{}****{}", &key[..4], &key[key.len() - 4..])
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
        if key.len() <= 8 {
            return "****".to_string();
        }
        format!("{}****{}", &key[..4], &key[key.len() - 4..])
    }

    pub fn to_masked(&self) -> MaskedMineruConfig {
        MaskedMineruConfig {
            masked_api_key: self.mask_key(),
            api_base: self.api_base.clone(),
        }
    }
}
