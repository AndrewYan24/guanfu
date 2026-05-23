use crate::errors::AppResult;
use lopdf::Document;

/// Extract text from a PDF file using lopdf (local, no external dependency).
/// Returns extracted text or empty string if extraction fails.
pub fn extract_text(pdf_path: &std::path::Path) -> AppResult<String> {
    let doc = Document::load(pdf_path)
        .map_err(|e| crate::errors::AppError::Unknown(format!("无法加载 PDF: {}", e)))?;

    let pages = doc.get_pages();
    let mut text = String::new();

    for (page_num, _) in pages.iter() {
        if let Ok(page_text) = doc.extract_text(&[*page_num]) {
            text.push_str(&page_text);
            text.push('\n');
        }
    }

    Ok(text)
}

/// Call MinerU API to extract text from a PDF file.
/// MinerU is an external OCR/document parsing service.
pub async fn extract_text_mineru(
    pdf_path: &std::path::Path,
    api_key: &str,
    api_base: &str,
) -> AppResult<String> {
    let client = reqwest::Client::new();
    let url = format!("{}/v1/extract", api_base.trim_end_matches('/'));

    let pdf_bytes = std::fs::read(pdf_path)
        .map_err(|e| crate::errors::AppError::IoError(format!("无法读取 PDF: {}", e)))?;

    let part = reqwest::multipart::Part::bytes(pdf_bytes)
        .file_name("paper.pdf")
        .mime_str("application/pdf")
        .map_err(|e| crate::errors::AppError::Unknown(format!("MIME 错误: {}", e)))?;

    let form = reqwest::multipart::Form::new()
        .part("file", part)
        .text("ocr", "true");

    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .multipart(form)
        .send()
        .await
        .map_err(|e| crate::errors::AppError::Unknown(format!("MinerU 请求失败: {}", e)))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let err_text = resp.text().await.unwrap_or_default();
        return Err(crate::errors::AppError::Unknown(format!(
            "MinerU API 错误 {}: {}",
            status, err_text
        )));
    }

    let resp_json: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| crate::errors::AppError::Unknown(format!("解析 MinerU 响应失败: {}", e)))?;

    let text = resp_json["text"]
        .as_str()
        .or_else(|| resp_json["content"].as_str())
        .unwrap_or("")
        .to_string();

    Ok(text)
}
