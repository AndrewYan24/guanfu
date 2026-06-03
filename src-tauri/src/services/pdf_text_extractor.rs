use crate::errors::AppResult;
use crate::models::OcrModelMode;
use lopdf::Document;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// Bundled models directory, set at app startup. Checked before downloading.
static BUNDLED_MODELS_DIR: OnceLock<PathBuf> = OnceLock::new();

/// Set the bundled models directory (called once at app startup).
pub fn set_bundled_models_dir(dir: PathBuf) {
    let _ = BUNDLED_MODELS_DIR.set(dir);
}

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

/// Server det model download URL (higher precision text detection, ~80MB).
/// Only the det model is available in ONNX format for server version.
/// cls/rec/dict reuse the bundled mobile models.
fn server_det_source() -> (&'static str, Vec<&'static str>) {
    (
        "ch_PP-OCRv5_server_det.onnx",
        vec![
            "https://www.modelscope.cn/models/RapidAI/RapidOCR/resolve/v3.6.0/onnx/PP-OCRv5/det/ch_PP-OCRv5_server_det.onnx",
        ],
    )
}

/// Extract text from a PDF file using lopdf (local, no external dependency).
pub fn extract_text(pdf_path: &Path) -> AppResult<String> {
    let file_name = pdf_path.file_name().unwrap_or_default().to_string_lossy();
    let bytes = std::fs::read(pdf_path)
        .map_err(|e| crate::errors::AppError::IoError(format!("无法读取 PDF 文件: {}", e)))?;

    let load_bytes = preprocess_pdf(&bytes);
    let doc = match Document::load_mem(&load_bytes) {
        Ok(doc) => doc,
        Err(e) => {
            Document::load(pdf_path).map_err(|e2| {
                crate::errors::AppError::Unknown(format!("lopdf 加载失败: load_mem={}, load={}", e, e2))
            })?
        }
    };

    let pages = doc.get_pages();
    let page_count = pages.len();
    let mut text = String::new();

    for (page_num, _) in pages.iter() {
        if let Ok(page_text) = doc.extract_text(&[*page_num]) {
            text.push_str(&page_text);
            text.push('\n');
        }
    }

    eprintln!("[pdf] {} 提取 {} 页, {} 字符", file_name, page_count, text.len());
    Ok(text)
}

/// Find the last occurrence of `needle` in `haystack`.
fn find_last(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .rposition(|w| w == needle)
}


/// Preprocess PDF bytes to fix common CNKI/non-standard PDF formatting issues.
/// Fixes:
/// 1. `xref N M` on the same line (lopdf requires `xref` on its own line)
/// 2. Extra metadata injected after `%%EOF`
fn preprocess_pdf(bytes: &[u8]) -> Vec<u8> {
    let mut result = bytes.to_vec();

    // Fix 1: `xref` followed by space + numbers on the same line.
    // lopdf requires `xref\n` but CNKI puts `xref 0 259\n`.
    // Pattern: b"xref " followed by digit -> insert \n after "xref"
    let mut fixes = 0;
    let mut i = 0;
    while i + 5 < result.len() {
        if &result[i..i + 5] == b"xref "
            && i + 5 < result.len()
            && result[i + 5].is_ascii_digit()
        {
            // Insert \n after "xref" (replace the space with \n)
            result[i + 4] = b'\n';
            fixes += 1;
        }
        i += 1;
    }
    if fixes > 0 {
        eprintln!("[pdf_text_extractor] 修复了 {} 处 xref 格式", fixes);
    }

    // Fix 2: Trim content after last %%EOF
    if let Some(pos) = find_last(&result, b"%%EOF") {
        let end = pos + 5;
        let end = if end < result.len() && (result[end] == b'\n' || result[end] == b'\r') {
            end + 1
        } else {
            end
        };
        if end < result.len() {
            eprintln!(
                "[pdf_text_extractor] 裁剪 PDF 尾部: {} → {} 字节",
                result.len(),
                end
            );
            result.truncate(end);
        }
    }

    result
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

/// Get the directory where downloaded OCR models are stored.
fn models_dir() -> AppResult<PathBuf> {
    let dir = dirs_next::data_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("guanfu")
        .join("ocr_models");
    Ok(dir)
}

/// Get the directory where server OCR models are stored.
fn server_models_dir() -> AppResult<PathBuf> {
    let dir = dirs_next::data_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("guanfu")
        .join("ocr_models_server");
    Ok(dir)
}

/// Ensure OCR model files exist and return individual file paths: (det, cls, rec, dict).
/// Mobile mode: uses bundled resources (no download needed).
/// Server mode: uses server det model (downloaded on demand) + bundled mobile cls/rec/dict.
async fn ensure_models(mode: OcrModelMode) -> AppResult<(PathBuf, PathBuf, PathBuf, PathBuf)> {
    match mode {
        OcrModelMode::Mobile => ensure_mobile_models().await,
        OcrModelMode::Server => ensure_server_models().await,
    }
}

/// Ensure mobile models (all bundled).
async fn ensure_mobile_models() -> AppResult<(PathBuf, PathBuf, PathBuf, PathBuf)> {
    // Check bundled resources first
    if let Some(bundled) = BUNDLED_MODELS_DIR.get() {
        let all_exist = model_sources()
            .iter()
            .all(|(filename, _)| bundled.join(filename).exists());
        if all_exist {
            eprintln!("[ocr] 使用内置 Mobile 模型: {}", bundled.display());
            return Ok((
                bundled.join("ch_PP-OCRv5_mobile_det.onnx"),
                bundled.join("ch_ppocr_mobile_v2.0_cls_infer.onnx"),
                bundled.join("ch_PP-OCRv5_rec_mobile_infer.onnx"),
                bundled.join("ppocrv5_dict.txt"),
            ));
        }
    }

    // Fall back to download directory
    let dir = models_dir()?;
    let all_exist = model_sources()
        .iter()
        .all(|(filename, _)| dir.join(filename).exists());
    if all_exist {
        return Ok((
            dir.join("ch_PP-OCRv5_mobile_det.onnx"),
            dir.join("ch_ppocr_mobile_v2.0_cls_infer.onnx"),
            dir.join("ch_PP-OCRv5_rec_mobile_infer.onnx"),
            dir.join("ppocrv5_dict.txt"),
        ));
    }

    // Download missing files
    std::fs::create_dir_all(&dir)
        .map_err(|e| crate::errors::AppError::IoError(format!("无法创建模型目录: {}", e)))?;
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| crate::errors::AppError::Unknown(format!("创建 HTTP 客户端失败: {}", e)))?;

    for (filename, sources) in model_sources() {
        let path = dir.join(filename);
        if path.exists() { continue; }
        eprintln!("[ocr] 下载 Mobile 模型: {}", filename);
        download_with_retries(&client, sources, &path).await?;
    }

    Ok((
        dir.join("ch_PP-OCRv5_mobile_det.onnx"),
        dir.join("ch_ppocr_mobile_v2.0_cls_infer.onnx"),
        dir.join("ch_PP-OCRv5_rec_mobile_infer.onnx"),
        dir.join("ppocrv5_dict.txt"),
    ))
}

/// Ensure server models: server det (downloaded) + mobile cls/rec/dict (bundled).
async fn ensure_server_models() -> AppResult<(PathBuf, PathBuf, PathBuf, PathBuf)> {
    // cls/rec/dict always come from bundled mobile models
    let (cls_path, rec_path, dict_path) = if let Some(bundled) = BUNDLED_MODELS_DIR.get() {
        let cls = bundled.join("ch_ppocr_mobile_v2.0_cls_infer.onnx");
        let rec = bundled.join("ch_PP-OCRv5_rec_mobile_infer.onnx");
        let dict = bundled.join("ppocrv5_dict.txt");
        if cls.exists() && rec.exists() && dict.exists() {
            (cls, rec, dict)
        } else {
            let dir = models_dir()?;
            (dir.join("ch_ppocr_mobile_v2.0_cls_infer.onnx"),
             dir.join("ch_PP-OCRv5_rec_mobile_infer.onnx"),
             dir.join("ppocrv5_dict.txt"))
        }
    } else {
        let dir = models_dir()?;
        (dir.join("ch_ppocr_mobile_v2.0_cls_infer.onnx"),
         dir.join("ch_PP-OCRv5_rec_mobile_infer.onnx"),
         dir.join("ppocrv5_dict.txt"))
    };

    // Server det model: check server models directory
    let server_dir = server_models_dir()?;
    let (det_filename, det_sources) = server_det_source();
    let det_path = server_dir.join(det_filename);

    if det_path.exists() {
        eprintln!("[ocr] 使用 Server det 模型: {}", det_path.display());
        return Ok((det_path, cls_path, rec_path, dict_path));
    }

    // Download server det model
    std::fs::create_dir_all(&server_dir)
        .map_err(|e| crate::errors::AppError::IoError(format!("无法创建 Server 模型目录: {}", e)))?;
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(300)) // 80MB needs more time
        .build()
        .map_err(|e| crate::errors::AppError::Unknown(format!("创建 HTTP 客户端失败: {}", e)))?;

    eprintln!("[ocr] 下载 Server det 模型 (~80MB)...");
    download_with_retries(&client, det_sources, &det_path).await?;

    Ok((det_path, cls_path, rec_path, dict_path))
}

/// Download a file with retries across multiple mirrors.
async fn download_with_retries(client: &reqwest::Client, sources: Vec<&str>, path: &Path) -> AppResult<()> {
    let mut last_err = String::new();
    for source_url in &sources {
        for attempt in 1..=MAX_RETRIES {
            match download_file(client, source_url, path).await {
                Ok(()) => return Ok(()),
                Err(e) => {
                    last_err = format!("{}", e);
                    if attempt < MAX_RETRIES {
                        tokio::time::sleep(std::time::Duration::from_millis(500 * attempt as u64)).await;
                    }
                }
            }
        }
    }
    Err(crate::errors::AppError::Unknown(format!(
        "模型下载失败. 已尝试所有下载源。请检查网络连接。最后错误: {}",
        last_err
    )))
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
async fn extract_text_paddle_ocr(pdf_path: &Path, mode: OcrModelMode) -> AppResult<String> {
    let file_name = pdf_path.file_name().unwrap_or_default().to_string_lossy();

    // Step 1: Ensure models are available
    eprintln!("[ocr] {} 检查模型 ({:?})...", file_name, mode);
    let (det_path, cls_path, rec_path, dict_path) = ensure_models(mode).await?;
    eprintln!("[ocr] {} 模型就绪", file_name);

    // Step 2: Render PDF pages to images using pdf-render (pure Rust)
    eprintln!("[ocr] {} 渲染 PDF 页面...", file_name);
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
    eprintln!("[ocr] {} 渲染完成, {} 页", file_name, pixmaps.len());

    // Step 3: Initialize PaddleOCR
    eprintln!("[ocr] {} 加载 OCR 模型...", file_name);
    let mut ocr = paddle_ocr_rs::ocr_lite::OcrLite::new();
    ocr.init_models_with_dict(
        det_path.to_str().unwrap(),
        cls_path.to_str().unwrap(),
        rec_path.to_str().unwrap(),
        dict_path.to_str().unwrap(),
        2, // num_threads
    )
    .map_err(|e| crate::errors::AppError::Unknown(format!("OCR 模型加载失败: {}", e)))?;
    eprintln!("[ocr] {} OCR 模型已加载", file_name);

    // Step 4: Run OCR on each rendered page
    let mut all_text = String::new();

    for (i, pixmap) in pixmaps.iter().enumerate() {
        eprintln!("[ocr] {} 识别第 {}/{} 页...", file_name, i + 1, pixmaps.len());
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
                let page_chars: usize = result.text_blocks.iter()
                    .map(|b| b.text.trim().len())
                    .sum();
                eprintln!("[ocr] {} 第 {} 页识别 {} 字符", file_name, i + 1, page_chars);
                for block in &result.text_blocks {
                    if !block.text.trim().is_empty() {
                        all_text.push_str(block.text.trim());
                        all_text.push('\n');
                    }
                }
            }
            Err(e) => {
                eprintln!("[ocr] {} 第 {} 页识别失败: {}", file_name, i + 1, e);
            }
        }
    }

    if all_text.trim().is_empty() {
        return Err(crate::errors::AppError::Unknown(
            "OCR 未识别到任何文字".to_string(),
        ));
    }

    eprintln!("[pdf] OCR 处理 {} 页, 识别 {} 字符", max_pages, all_text.len());
    Ok(all_text)
}

/// Extract text with automatic fallback to OCR.
/// First tries lopdf extraction, then falls back to PaddleOCR if text is insufficient or lopdf fails.
pub async fn extract_text_with_ocr_fallback(pdf_path: &Path, mode: OcrModelMode) -> AppResult<String> {
    let file_name = pdf_path.file_name().unwrap_or_default().to_string_lossy();
    match extract_text(pdf_path) {
        Ok(text) if has_sufficient_text(&text) => Ok(text),
        Ok(text) => {
            eprintln!("[pdf] {} 文本不足 ({} 字符), 尝试 OCR...", file_name, text.len());
            match extract_text_paddle_ocr(pdf_path, mode).await {
                Ok(ocr_text) => {
                    eprintln!("[pdf] {} OCR 成功, {} 字符", file_name, ocr_text.len());
                    Ok(ocr_text)
                }
                Err(_) => Ok(text),
            }
        }
        Err(e) => {
            eprintln!("[pdf] {} lopdf 失败, 尝试 OCR...", file_name);
            match extract_text_paddle_ocr(pdf_path, mode).await {
                Ok(ocr_text) => {
                    eprintln!("[pdf] {} OCR 成功, {} 字符", file_name, ocr_text.len());
                    Ok(ocr_text)
                }
                Err(_) => Err(e),
            }
        }
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

/// Check if server OCR models are downloaded.
pub fn has_server_ocr_models() -> bool {
    let Ok(dir) = server_models_dir() else { return false; };
    let (filename, _) = server_det_source();
    dir.join(filename).exists()
}

/// Download server OCR models to the user data directory.
pub async fn download_server_ocr_models() -> AppResult<()> {
    let (det_filename, det_sources) = server_det_source();
    let dir = server_models_dir()?;
    std::fs::create_dir_all(&dir)
        .map_err(|e| crate::errors::AppError::IoError(format!("无法创建 Server 模型目录: {}", e)))?;

    let path = dir.join(det_filename);
    if path.exists() {
        eprintln!("[ocr] Server det 模型已存在");
        return Ok(());
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .build()
        .map_err(|e| crate::errors::AppError::Unknown(format!("创建 HTTP 客户端失败: {}", e)))?;

    eprintln!("[ocr] 下载 Server det 模型 (~80MB)...");
    download_with_retries(&client, det_sources, &path).await?;
    eprintln!("[ocr] Server det 模型下载完成");
    Ok(())
}
