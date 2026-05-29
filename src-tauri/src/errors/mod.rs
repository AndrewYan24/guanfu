use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AppError {
    #[error("项目不存在: {0}")]
    ProjectNotFound(String),

    #[error("文件不存在: {0}")]
    FileNotFound(String),

    #[error("IO 错误: {0}")]
    IoError(String),

    #[error("JSON 解析错误: {0}")]
    JsonError(String),

    #[error("AI 调用失败: {0}")]
    AiError(String),

    #[error("网络请求失败: {0}")]
    NetworkError(String),

    #[error("元数据抓取失败: {0}")]
    MetadataError(String),

    #[error("未知错误: {0}")]
    Unknown(String),
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError::IoError(e.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        AppError::JsonError(e.to_string())
    }
}

impl From<anyhow::Error> for AppError {
    fn from(e: anyhow::Error) -> Self {
        AppError::Unknown(e.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;

/// Helper: convert a poisoned mutex lock into an AppError.
pub fn lock_mutex<'a, T>(m: &'a std::sync::Mutex<T>) -> AppResult<std::sync::MutexGuard<'a, T>> {
    m.lock().map_err(|_| AppError::Unknown("内部状态锁异常".to_string()))
}
