use crate::errors::AppResult;
use lopdf::Document;
use std::path::{Path, PathBuf};

/// Minimum number of meaningful characters to consider text extraction sufficient.
const MIN_TEXT_LENGTH: usize = 100;

/// Model download URLs. Each model has multiple mirrors (ModelScope + HuggingFace).
/// Tries ModelScope first (faster in China), falls back to HuggingFace (global).
fn model_sources() -> Vec<(&'static str, Vec<&'static str>)> {
    vec![
        (
            "ch_PP-OCRv5_mobile_det.onnx",
            vec![
                "https://www.modelscope.cn/models/RapidAI/RapidOCR/resolve/v3.6.0/onnx/PP-OCRv5/det/ch_PP-OCRv5_mobile_det.onnx",
                "https://huggingface.co/RapidAI/RapidOCR/resolve/main/onnx/PP-OCRv5/det/ch_PP-OCRv5_mobile_det.onnx",
            ],
        ),
        (
            "ch_ppocr_mobile_v2.0_cls_infer.onnx",
            vec![
                "https://www.modelscope.cn/models/RapidAI/RapidOCR/resolve/v3.6.0/onnx/PP-OCRv4/cls/ch_ppocr_mobile_v2.0_cls_infer.onnx",
                "https://huggingface.co/RapidAI/RapidOCR/resolve/main/onnx/PP-OCRv4/cls/ch_ppocr_mobile_v2.0_cls_infer.onnx",
            ],
        ),
        (
            "ch_PP-OCRv5_rec_mobile_infer.onnx",
            vec![
                "https://www.modelscope.cn/models/RapidAI/RapidOCR/resolve/v3.6.0/onnx/PP-OCRv5/rec/ch_PP-OCRv5_rec_mobile_infer.onnx",
                "https://huggingface.co/RapidAI/RapidOCR/resolve/main/onnx/PP-OCRv5/rec/ch_PP-OCRv5_rec_mobile_infer.onnx",
            ],
        ),
        (
            "ppocrv5_dict.txt",
            vec![
                "https://www.modelscope.cn/models/RapidAI/RapidOCR/resolve/v3.6.0/paddle/PP-OCRv5/rec/ch_PP-OCRv5_rec_mobile_infer/ppocrv5_dict.txt",
                "https://huggingface.co/RapidAI/RapidOCR/resolve/main/paddle/PP-OCRv5/rec/ch_PP-OCRv5_rec_mobile_infer/ppocrv5_dict.txt",
            ],
        ),
    ]
}

/// Maximum retry attempts per download source.
const MAX_RETRIES: u32 = 3;

/// Extract text from a PDF file using lopdf (local, no external dependency).
pub fn extract_text(pdf_path: &Path) -> AppResult<String> {
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

/// Check if extracted text is sufficient for AI analysis.
pub fn has_sufficient_text(text: &str) -> bool {
    let trimmed = text.trim();
    if trimmed.len() < MIN_TEXT_LENGTH {
        return false;
    }
    let alpha_count = trimmed.chars().filter(|c| c.is_alphanumeric()).count();
    alpha_count >= MIN_TEXT_LENGTH / 2
}

/// Get the directory where OCR models are stored.
fn models_dir() -> AppResult<PathBuf> {
    let dir = dirs_next::data_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("guanfu")
        .join("ocr_models");
    Ok(dir)
}

/// Ensure OCR model files exist, downloading them if necessary.
/// Tries multiple mirrors (ModelScope → HuggingFace) with retry logic.
async fn ensure_models() -> AppResult<PathBuf> {
    let dir = models_dir()?;

    // Check if all files exist
    let all_exist = model_sources()
        .iter()
        .all(|(filename, _)| dir.join(filename).exists());
    if all_exist {
        return Ok(dir);
    }

    // Create directory
    std::fs::create_dir_all(&dir)
        .map_err(|e| crate::errors::AppError::IoError(format!("无法创建模型目录: {}", e)))?;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| crate::errors::AppError::Unknown(format!("创建 HTTP 客户端失败: {}", e)))?;

    // Download each missing file, trying all mirrors
    for (filename, sources) in model_sources() {
        let path = dir.join(filename);
        if path.exists() {
            continue;
        }

        let mut last_err = String::new();
        let mut downloaded = false;

        for source_url in &sources {
            for attempt in 1..=MAX_RETRIES {
                match download_file(&client, source_url, &path).await {
                    Ok(()) => {
                        downloaded = true;
                        break;
                    }
                    Err(e) => {
                        last_err = format!("{}", e);
                        if attempt < MAX_RETRIES {
                            // Brief delay before retry
                            tokio::time::sleep(std::time::Duration::from_millis(500 * attempt as u64)).await;
                        }
                    }
                }
            }
            if downloaded {
                break;
            }
        }

        if !downloaded {
            return Err(crate::errors::AppError::Unknown(format!(
                "OCR 模型下载失败 ({}). 已尝试所有下载源。请检查网络连接。最后错误: {}",
                filename, last_err
            )));
        }
    }

    Ok(dir)
}

async fn download_file(client: &reqwest::Client, url: &str, path: &Path) -> AppResult<()> {
    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| crate::errors::AppError::Unknown(format!("网络请求失败: {}", e)))?;

    if !resp.status().is_success() {
        return Err(crate::errors::AppError::Unknown(format!(
            "HTTP {}",
            resp.status()
        )));
    }

    let bytes = resp
        .bytes()
        .await
        .map_err(|e| crate::errors::AppError::Unknown(format!("下载数据失败: {}", e)))?;

    std::fs::write(path, &bytes)
        .map_err(|e| crate::errors::AppError::IoError(format!("写入文件失败: {}", e)))?;

    Ok(())
}

/// Extract text from a PDF using integrated PaddleOCR (no system dependencies).
async fn extract_text_paddle_ocr(pdf_path: &Path) -> AppResult<String> {
    // Step 1: Ensure models are downloaded
    let models_dir = ensure_models().await?;
    let det_path = models_dir.join("ch_PP-OCRv5_mobile_det.onnx");
    let cls_path = models_dir.join("ch_ppocr_mobile_v2.0_cls_infer.onnx");
    let rec_path = models_dir.join("ch_PP-OCRv5_rec_mobile_infer.onnx");
    let dict_path = models_dir.join("ppocrv5_dict.txt");

    // Step 2: Render PDF pages to images using pdf-render (pure Rust)
    let pdf_bytes = std::fs::read(pdf_path)
        .map_err(|e| crate::errors::AppError::IoError(format!("无法读取 PDF: {}", e)))?;

    let pdf = pdf_render::pdf_syntax::Pdf::new(pdf_bytes)
        .map_err(|e| crate::errors::AppError::Unknown(format!("无法解析 PDF: {:?}", e)))?;

    let page_count = pdf.pages().len();
    // Limit to first 15 pages for performance
    let max_pages = page_count.min(15);
    let range = if max_pages > 0 {
        Some(0..=max_pages - 1)
    } else {
        None
    };

    let settings = pdf_render::pdf_interpret::InterpreterSettings::default();
    let scale: f32 = 2.0; // 2x scale for better OCR accuracy

    let pixmaps = pdf_render::render_pdf(&pdf, scale, settings, range)
        .ok_or_else(|| crate::errors::AppError::Unknown("PDF 渲染失败".to_string()))?;

    if pixmaps.is_empty() {
        return Err(crate::errors::AppError::Unknown("PDF 没有可渲染的页面".to_string()));
    }

    // Step 3: Initialize PaddleOCR
    let mut ocr = paddle_ocr_rs::ocr_lite::OcrLite::new();
    ocr.init_models_with_dict(
        det_path.to_str().unwrap(),
        cls_path.to_str().unwrap(),
        rec_path.to_str().unwrap(),
        dict_path.to_str().unwrap(),
        2, // num_threads
    )
    .map_err(|e| crate::errors::AppError::Unknown(format!("OCR 模型加载失败: {}", e)))?;

    // Step 4: Run OCR on each rendered page
    let mut all_text = String::new();

    for pixmap in &pixmaps {
        // Convert Pixmap (premultiplied RGBA) to RGB image
        let width = pixmap.width() as u32;
        let height = pixmap.height() as u32;
        let unpremul = pixmap.clone().take_unpremultiplied();
        let rgb_data: Vec<u8> = unpremul.iter().flat_map(|p| [p.r, p.g, p.b]).collect();

        let rgb_image = image::RgbImage::from_raw(width, height, rgb_data)
            .ok_or_else(|| crate::errors::AppError::Unknown("图像转换失败".to_string()))?;

        // Run OCR
        match ocr.detect(
            &rgb_image,
            50,    // padding
            1024,  // max_side_len
            0.5,   // box_score_thresh
            0.3,   // box_thresh
            1.6,   // un_clip_ratio
            false, // do_angle
            false, // most_angle
        ) {
            Ok(result) => {
                for block in &result.text_blocks {
                    if !block.text.trim().is_empty() {
                        all_text.push_str(block.text.trim());
                        all_text.push('\n');
                    }
                }
            }
            Err(e) => {
                // Log but continue with other pages
                eprintln!("OCR 页面识别失败: {}", e);
            }
        }
    }

    if all_text.trim().is_empty() {
        return Err(crate::errors::AppError::Unknown(
            "OCR 未识别到任何文字".to_string(),
        ));
    }

    Ok(all_text)
}

/// Extract text with automatic fallback to OCR.
/// First tries lopdf extraction, then falls back to PaddleOCR if text is insufficient.
pub async fn extract_text_with_ocr_fallback(pdf_path: &Path) -> AppResult<String> {
    // Try standard text extraction first
    match extract_text(pdf_path) {
        Ok(text) if has_sufficient_text(&text) => Ok(text),
        Ok(_) => {
            // Text is insufficient (likely scanned PDF), try OCR
            match extract_text_paddle_ocr(pdf_path).await {
                Ok(ocr_text) => Ok(ocr_text),
                Err(ocr_err) => {
                    eprintln!("警告: PDF 文本提取不足且 OCR 失败 ({}). 将使用有限的文本。", ocr_err);
                    // Return whatever lopdf extracted, even if insufficient
                    extract_text(pdf_path)
                }
            }
        }
        Err(e) => Err(e),
    }
}

/// Call MinerU API to extract text from a PDF file.
pub async fn extract_text_mineru(
    pdf_path: &Path,
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
