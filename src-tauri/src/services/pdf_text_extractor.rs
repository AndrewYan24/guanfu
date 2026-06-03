use crate::errors::AppResult;
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

/// Get the directory where OCR models are stored.
fn models_dir() -> AppResult<PathBuf> {
    let dir = dirs_next::data_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("guanfu")
        .join("ocr_models");
    Ok(dir)
}

/// Ensure OCR model files exist.
/// First checks bundled resources (packaged with the app), then falls back to download.
async fn ensure_models() -> AppResult<PathBuf> {
    // 1. Check bundled resources first
    if let Some(bundled) = BUNDLED_MODELS_DIR.get() {
        let all_exist = model_sources()
            .iter()
            .all(|(filename, _)| bundled.join(filename).exists());
        if all_exist {
            eprintln!("[ocr] 使用内置模型: {}", bundled.display());
            return Ok(bundled.clone());
        }
    }

    // 2. Fall back to download directory
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
        eprintln!("[ocr] 下载模型: {}", filename);

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
    use tokio::io::AsyncWriteExt;
    use futures_util::StreamExt;

    let resp = client
        .get(url)
        .header("User-Agent", "guanfu/0.5.0")
        .send()
        .await
        .map_err(|e| crate::errors::AppError::Unknown(format!("网络请求失败: {}", e)))?;

    if !resp.status().is_success() {
        return Err(crate::errors::AppError::Unknown(format!(
            "HTTP {}",
            resp.status()
        )));
    }

    // Stream to file instead of loading entire response into memory
    let tmp_path = path.with_extension("downloading");
    let mut file = tokio::fs::File::create(&tmp_path)
        .await
        .map_err(|e| crate::errors::AppError::IoError(format!("创建临时文件失败: {}", e)))?;

    let mut stream = resp.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| crate::errors::AppError::Unknown(format!("下载数据失败: {}", e)))?;
        file.write_all(&chunk)
            .await
            .map_err(|e| crate::errors::AppError::IoError(format!("写入文件失败: {}", e)))?;
    }
    file.flush().await
        .map_err(|e| crate::errors::AppError::IoError(format!("刷新文件失败: {}", e)))?;
    drop(file);

    // Atomic rename
    std::fs::rename(&tmp_path, path)
        .map_err(|e| crate::errors::AppError::IoError(format!("重命名文件失败: {}", e)))?;

    Ok(())
}

/// Extract text from a PDF using integrated PaddleOCR (no system dependencies).
async fn extract_text_paddle_ocr(pdf_path: &Path) -> AppResult<String> {
    let file_name = pdf_path.file_name().unwrap_or_default().to_string_lossy();

    // Step 1: Ensure models are downloaded
    eprintln!("[ocr] {} 检查模型...", file_name);
    let models_dir = ensure_models().await?;
    let det_path = models_dir.join("ch_PP-OCRv5_mobile_det.onnx");
    let cls_path = models_dir.join("ch_ppocr_mobile_v2.0_cls_infer.onnx");
    let rec_path = models_dir.join("ch_PP-OCRv5_rec_mobile_infer.onnx");
    let dict_path = models_dir.join("ppocrv5_dict.txt");
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
pub async fn extract_text_with_ocr_fallback(pdf_path: &Path) -> AppResult<String> {
    let file_name = pdf_path.file_name().unwrap_or_default().to_string_lossy();
    match extract_text(pdf_path) {
        Ok(text) if has_sufficient_text(&text) => Ok(text),
        Ok(text) => {
            eprintln!("[pdf] {} 文本不足 ({} 字符), 尝试 OCR...", file_name, text.len());
            match extract_text_paddle_ocr(pdf_path).await {
                Ok(ocr_text) => {
                    eprintln!("[pdf] {} OCR 成功, {} 字符", file_name, ocr_text.len());
                    Ok(ocr_text)
                }
                Err(_) => Ok(text),
            }
        }
        Err(e) => {
            eprintln!("[pdf] {} lopdf 失败, 尝试 OCR...", file_name);
            match extract_text_paddle_ocr(pdf_path).await {
                Ok(ocr_text) => {
                    eprintln!("[pdf] {} OCR 成功, {} 字符", file_name, ocr_text.len());
                    Ok(ocr_text)
                }
                Err(_) => Err(e),
            }
        }
    }
}

/// MinerU precision API — uses batch file upload endpoint.
/// POST /api/v4/file-urls/batch → PUT upload → poll /api/v4/extract-results/batch/{batch_id}
pub async fn extract_text_mineru(
    pdf_path: &Path,
    api_key: &str,
    api_base: &str,
) -> AppResult<String> {
    let file_name = pdf_path.file_name().unwrap_or_default().to_string_lossy();
    let client = reqwest::Client::new();
    let base = api_base.trim_end_matches('/');

    // Step 1: Request upload URLs
    let batch_url = format!("{}/v4/file-urls/batch", base);
    let pdf_bytes = std::fs::read(pdf_path)
        .map_err(|e| crate::errors::AppError::IoError(format!("无法读取 PDF: {}", e)))?;

    let body = serde_json::json!({
        "files": [{ "name": format!("{}.pdf", uuid::Uuid::new_v4()) }],
        "is_ocr": true,
        "enable_formula": false,
        "enable_table": true,
        "language": "ch"
    });

    eprintln!("[mineru] {} 请求上传 URL...", file_name);
    let resp = client.post(&batch_url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .send().await
        .map_err(|e| crate::errors::AppError::Unknown(format!("MinerU 请求失败: {}", e)))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let err_text = resp.text().await.unwrap_or_default();
        return Err(crate::errors::AppError::Unknown(format!("MinerU API 错误 {}: {}", status, err_text)));
    }

    let resp_json: serde_json::Value = resp.json().await
        .map_err(|e| crate::errors::AppError::Unknown(format!("解析 MinerU 响应失败: {}", e)))?;

    let batch_id = resp_json["data"]["batch_id"].as_str()
        .ok_or_else(|| crate::errors::AppError::Unknown("MinerU 响应缺少 batch_id".to_string()))?;
    let upload_url = resp_json["data"]["file_urls"][0].as_str()
        .ok_or_else(|| crate::errors::AppError::Unknown("MinerU 响应缺少上传 URL".to_string()))?;

    // Step 2: Upload PDF
    eprintln!("[mineru] {} 上传文件...", file_name);
    client.put(upload_url)
        .body(pdf_bytes)
        .send().await
        .map_err(|e| crate::errors::AppError::Unknown(format!("MinerU 上传失败: {}", e)))?
        .error_for_status()
        .map_err(|e| crate::errors::AppError::Unknown(format!("MinerU 上传错误: {}", e)))?;

    // Step 3: Poll batch results
    let poll_url = format!("{}/v4/extract-results/batch/{}", base, batch_id);
    eprintln!("[mineru] {} 等待解析...", file_name);
    poll_mineru_batch(&client, api_key, &poll_url, &file_name).await
}

async fn poll_mineru_batch(
    client: &reqwest::Client,
    api_key: &str,
    poll_url: &str,
    file_name: &str,
) -> AppResult<String> {
    let poll_interval = std::time::Duration::from_secs(3);
    let timeout = std::time::Duration::from_secs(300);
    let start = std::time::Instant::now();

    loop {
        if start.elapsed() > timeout {
            return Err(crate::errors::AppError::Unknown("MinerU 解析超时 (300s)".to_string()));
        }

        tokio::time::sleep(poll_interval).await;

        let resp = client.get(poll_url)
            .header("Authorization", format!("Bearer {}", api_key))
            .send().await
            .map_err(|e| crate::errors::AppError::Unknown(format!("MinerU 轮询失败: {}", e)))?;

        if !resp.status().is_success() {
            continue;
        }

        let json: serde_json::Value = resp.json().await
            .map_err(|e| crate::errors::AppError::Unknown(format!("MinerU 响应解析失败: {}", e)))?;

        let results = match json["data"]["extract_result"].as_array() {
            Some(r) => r,
            None => continue,
        };

        if let Some(first) = results.first() {
            let state = first["state"].as_str().unwrap_or("");
            match state {
                "done" => {
                    let zip_url = first["full_zip_url"].as_str()
                        .ok_or_else(|| crate::errors::AppError::Unknown("MinerU 缺少 zip URL".to_string()))?;
                    eprintln!("[mineru] {} 解析完成，下载结果...", file_name);
                    return download_and_extract_zip(client, zip_url).await;
                }
                "failed" => {
                    let err = first["err_msg"].as_str().unwrap_or("未知错误");
                    return Err(crate::errors::AppError::Unknown(format!("MinerU 解析失败: {}", err)));
                }
                _ => {
                    eprintln!("[mineru] {} 状态: {}", file_name, state);
                }
            }
        }
    }
}

async fn download_and_extract_zip(client: &reqwest::Client, zip_url: &str) -> AppResult<String> {
    let resp = client.get(zip_url).send().await
        .map_err(|e| crate::errors::AppError::Unknown(format!("下载 MinerU 结果失败: {}", e)))?;

    if !resp.status().is_success() {
        return Err(crate::errors::AppError::Unknown(format!("下载 MinerU 结果 HTTP {}", resp.status())));
    }

    let zip_bytes = resp.bytes().await
        .map_err(|e| crate::errors::AppError::Unknown(format!("读取 MinerU 结果失败: {}", e)))?;

    let reader = std::io::Cursor::new(zip_bytes);
    let mut archive = zip::ZipArchive::new(reader)
        .map_err(|e| crate::errors::AppError::Unknown(format!("解压 MinerU 结果失败: {}", e)))?;

    // Look for full.md in the zip
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)
            .map_err(|e| crate::errors::AppError::Unknown(format!("读取 zip 条目失败: {}", e)))?;
        let name = file.name().to_string();
        if name.ends_with("full.md") {
            let mut content = String::new();
            use std::io::Read;
            file.read_to_string(&mut content)
                .map_err(|e| crate::errors::AppError::Unknown(format!("读取 full.md 失败: {}", e)))?;
            return Ok(content);
        }
    }

    Err(crate::errors::AppError::Unknown("MinerU 结果中未找到 full.md".to_string()))
}

/// MinerU Agent lightweight API — no login required, IP rate-limited.
/// POST /api/v1/agent/parse/file → PUT upload → poll /api/v1/agent/parse/{task_id}
pub async fn extract_text_mineru_agent(pdf_path: &Path) -> AppResult<String> {
    let file_name = pdf_path.file_name().unwrap_or_default().to_string_lossy();
    let client = reqwest::Client::new();

    // Step 1: Get signed upload URL
    let file_name_owned = format!("{}.pdf", uuid::Uuid::new_v4());
    let body = serde_json::json!({
        "file_name": file_name_owned,
        "is_ocr": true,
        "language": "ch"
    });

    eprintln!("[agent] {} 请求上传 URL...", file_name);
    let resp = client.post("https://mineru.net/api/v1/agent/parse/file")
        .header("Content-Type", "application/json")
        .json(&body)
        .send().await
        .map_err(|e| crate::errors::AppError::Unknown(format!("MinerU Agent 请求失败: {}", e)))?;

    if resp.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
        return Err(crate::errors::AppError::Unknown("MinerU 轻量解析请求过于频繁，请稍后重试".to_string()));
    }
    if !resp.status().is_success() {
        let status = resp.status();
        let err_text = resp.text().await.unwrap_or_default();
        return Err(crate::errors::AppError::Unknown(format!("MinerU Agent 错误 {}: {}", status, err_text)));
    }

    let resp_json: serde_json::Value = resp.json().await
        .map_err(|e| crate::errors::AppError::Unknown(format!("解析 MinerU Agent 响应失败: {}", e)))?;

    let task_id = resp_json["data"]["task_id"].as_str()
        .ok_or_else(|| crate::errors::AppError::Unknown("MinerU Agent 响应缺少 task_id".to_string()))?;
    let upload_url = resp_json["data"]["file_url"].as_str()
        .ok_or_else(|| crate::errors::AppError::Unknown("MinerU Agent 响应缺少 file_url".to_string()))?;

    // Step 2: Upload PDF
    let pdf_bytes = std::fs::read(pdf_path)
        .map_err(|e| crate::errors::AppError::IoError(format!("无法读取 PDF: {}", e)))?;

    eprintln!("[agent] {} 上传文件...", file_name);
    client.put(upload_url)
        .body(pdf_bytes)
        .send().await
        .map_err(|e| crate::errors::AppError::Unknown(format!("MinerU Agent 上传失败: {}", e)))?
        .error_for_status()
        .map_err(|e| crate::errors::AppError::Unknown(format!("MinerU Agent 上传错误: {}", e)))?;

    // Step 3: Poll for result
    let poll_url = format!("https://mineru.net/api/v1/agent/parse/{}", task_id);
    eprintln!("[agent] {} 等待解析...", file_name);
    let poll_interval = std::time::Duration::from_secs(3);
    let timeout = std::time::Duration::from_secs(300);
    let start = std::time::Instant::now();

    loop {
        if start.elapsed() > timeout {
            return Err(crate::errors::AppError::Unknown("MinerU Agent 解析超时 (300s)".to_string()));
        }

        tokio::time::sleep(poll_interval).await;

        let resp = client.get(&poll_url).send().await
            .map_err(|e| crate::errors::AppError::Unknown(format!("MinerU Agent 轮询失败: {}", e)))?;

        if !resp.status().is_success() {
            continue;
        }

        let json: serde_json::Value = resp.json().await
            .map_err(|e| crate::errors::AppError::Unknown(format!("MinerU Agent 响应解析失败: {}", e)))?;

        let state = json["data"]["state"].as_str().unwrap_or("");
        match state {
            "done" => {
                let md_url = json["data"]["markdown_url"].as_str()
                    .ok_or_else(|| crate::errors::AppError::Unknown("MinerU Agent 缺少 markdown_url".to_string()))?;
                eprintln!("[agent] {} 解析完成，下载 Markdown...", file_name);
                let md_resp = client.get(md_url).send().await
                    .map_err(|e| crate::errors::AppError::Unknown(format!("下载 Markdown 失败: {}", e)))?;
                let text = md_resp.text().await
                    .map_err(|e| crate::errors::AppError::Unknown(format!("读取 Markdown 失败: {}", e)))?;
                return Ok(text);
            }
            "failed" => {
                let err = json["data"]["err_msg"].as_str().unwrap_or("未知错误");
                return Err(crate::errors::AppError::Unknown(format!("MinerU Agent 解析失败: {}", err)));
            }
            _ => {
                eprintln!("[agent] {} 状态: {}", file_name, state);
            }
        }
    }
}
