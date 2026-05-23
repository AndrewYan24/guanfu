use std::sync::Mutex;
use crate::models::{AiSettings, OcrMethod};

pub struct AppState {
    pub ai_settings: Mutex<AiSettings>,
}

impl Default for AppState {
    fn default() -> Self {
        let settings = load_persisted_settings().unwrap_or(AiSettings {
            openai_compatible: None,
            anthropic: None,
            active_provider: None,
            ocr_method: OcrMethod::Local,
            mineru: None,
            embedding_model: None,
            embedding_base_url: None,
            embedding_api_key: None,
            locale: None,
            default_project_dir: None,
        });

        Self {
            ai_settings: Mutex::new(settings),
        }
    }
}

pub fn settings_file_path() -> std::path::PathBuf {
    let dir = dirs_next::data_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    let app_dir = dir.join("com.guanfu.app");
    std::fs::create_dir_all(&app_dir).ok();
    app_dir.join("ai_settings.json")
}

pub fn load_persisted_settings() -> Option<AiSettings> {
    let path = settings_file_path();
    let content = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

pub fn persist_settings(settings: &AiSettings) {
    let path = settings_file_path();
    if let Ok(json) = serde_json::to_string_pretty(settings) {
        std::fs::write(path, json).ok();
    }
}
