use crate::errors::AppResult;
use crate::http_server;
use crate::models::{AiSettings, ExtractedMetadata, Insight, RelationRecommendation};
use crate::services::{ai_manager, pdf_text_extractor, project_service};
use crate::state::{self, AppState};
use std::path::Path;
use std::sync::Arc;
use tauri::{Emitter, State};

#[tauri::command]
pub async fn ai_parse_pdf(
    state: State<'_, AppState>,
    project_path: String,
    paper_id: String,
) -> AppResult<ExtractedMetadata> {
    let mut project = project_service::open_project(&project_path)?;
    let paper = project
        .papers
        .iter()
        .find(|p| p.id == paper_id)
        .ok_or_else(|| crate::errors::AppError::FileNotFound(paper_id.clone()))?;

    let pdf_path = Path::new(&project_path)
        .join("papers")
        .join(&paper.file_path);

    let settings = state.ai_settings.lock().unwrap().clone();

    // Step 1: Extract text from PDF (with automatic OCR fallback for scanned PDFs)
    let text = match settings.ocr_method {
        crate::models::OcrMethod::Local => {
            pdf_text_extractor::extract_text_with_ocr_fallback(&pdf_path)
                .await
                .unwrap_or_default()
        }
        crate::models::OcrMethod::Mineru => {
            if let Some(ref mineru) = settings.mineru {
                pdf_text_extractor::extract_text_mineru(
                    &pdf_path,
                    &mineru.api_key,
                    &mineru.api_base,
                )
                .await
                .unwrap_or_default()
            } else {
                pdf_text_extractor::extract_text_with_ocr_fallback(&pdf_path)
                    .await
                    .unwrap_or_default()
            }
        }
    };

    // Fallback: if text extraction returned nothing, use abstract/title
    let text = if text.trim().is_empty() {
        if let Some(ref abstract_text) = paper.abstract_text {
            if !abstract_text.is_empty() {
                abstract_text.clone()
            } else {
                format!("Title: {}", paper.title)
            }
        } else {
            format!("Title: {}", paper.title)
        }
    } else {
        text
    };

    // Step 2: Send extracted text to AI (extracts title/authors/year/abstract + 8 structured fields)
    let metadata = ai_manager::parse_text(&text, &settings).await?;

    // Step 3: Update paper's basic fields from AI result
    let paper_idx = project.papers.iter().position(|p| p.id == paper_id).unwrap();
    if let Some(ref title) = metadata.title {
        if !title.is_empty() {
            project.papers[paper_idx].title = title.clone();
        }
    }
    if let Some(ref authors) = metadata.authors {
        if !authors.is_empty() {
            project.papers[paper_idx].authors = authors.clone();
        }
    }
    if metadata.year.is_some() {
        project.papers[paper_idx].year = metadata.year;
    }
    if let Some(ref abstract_text) = metadata.abstract_text {
        if !abstract_text.is_empty() {
            project.papers[paper_idx].abstract_text = Some(abstract_text.clone());
        }
    }
    project.papers[paper_idx].updated_at = chrono::Utc::now().to_rfc3339();
    project_service::save_project(&project)?;

    Ok(metadata)
}

/// Batch parse multiple papers — loads project once, parses concurrently, saves once.
/// Emits "parse_progress" events: { paperId, done, total }
#[tauri::command]
pub async fn ai_parse_pdfs_batch(
    state: State<'_, AppState>,
    app: tauri::AppHandle,
    project_path: String,
    paper_ids: Vec<String>,
) -> AppResult<std::collections::HashMap<String, ExtractedMetadata>> {
    let mut project = project_service::open_project(&project_path)?;
    let settings = state.ai_settings.lock().unwrap().clone();

    let concurrency = settings
        .advanced
        .as_ref()
        .map(|a| a.concurrency as usize)
        .unwrap_or(3);

    // Build work items: (paper_id, pdf_path)
    let mut work_items = Vec::new();
    for pid in &paper_ids {
        if let Some(paper) = project.papers.iter().find(|p| &p.id == pid) {
            let pdf_path = Path::new(&project_path)
                .join("papers")
                .join(&paper.file_path);
            if pdf_path.exists() {
                work_items.push((pid.clone(), pdf_path));
            }
        }
    }

    let total = work_items.len();
    let sem = Arc::new(tokio::sync::Semaphore::new(concurrency));
    let mut handles = Vec::new();

    for (paper_id, pdf_path) in work_items {
        let sem = sem.clone();
        let s = settings.clone();
        handles.push(tokio::spawn(async move {
            let _permit = sem.acquire().await.unwrap();
            let text = pdf_text_extractor::extract_text_with_ocr_fallback(&pdf_path)
                .await
                .unwrap_or_default();
            let text = if text.trim().is_empty() {
                format!("Paper ID: {}", paper_id)
            } else {
                text
            };
            let result = ai_manager::parse_text(&text, &s).await;
            (paper_id, result)
        }));
    }

    let mut results = std::collections::HashMap::new();
    let mut done_count = 0usize;
    for h in handles {
        if let Ok((paper_id, result)) = h.await {
            match result {
                Ok(metadata) => {
                    // Update project in-memory
                    if let Some(paper) = project.papers.iter_mut().find(|p| p.id == paper_id) {
                        if let Some(ref title) = metadata.title {
                            if !title.is_empty() { paper.title = title.clone(); }
                        }
                        if let Some(ref authors) = metadata.authors {
                            if !authors.is_empty() { paper.authors = authors.clone(); }
                        }
                        if metadata.year.is_some() { paper.year = metadata.year; }
                        if let Some(ref abstract_text) = metadata.abstract_text {
                            if !abstract_text.is_empty() {
                                paper.abstract_text = Some(abstract_text.clone());
                            }
                        }
                        paper.updated_at = chrono::Utc::now().to_rfc3339();
                    }
                    results.insert(paper_id, metadata);
                }
                Err(e) => {
                    eprintln!("[ai_parse_pdfs_batch] paper {} failed: {}", paper_id, e);
                }
            }
        }
        done_count += 1;
        let _ = app.emit("parse_progress", serde_json::json!({
            "done": done_count,
            "total": total,
        }));
    }

    // Save once at the end
    project_service::save_project(&project)?;

    Ok(results)
}

#[tauri::command]
pub async fn ai_recommend_relations(
    state: State<'_, AppState>,
    project_path: String,
    new_paper_ids: Option<Vec<String>>,
) -> AppResult<Vec<RelationRecommendation>> {
    let project = project_service::open_project(&project_path)?;

    // Only analyze papers that have been parsed (have metadata)
    let papers_with_meta: Vec<_> = project
        .papers
        .iter()
        .filter(|p| p.metadata.is_some())
        .cloned()
        .collect();

    if papers_with_meta.len() < 2 {
        return Ok(vec![]);
    }

    let settings = state.ai_settings.lock().unwrap().clone();

    if let Some(ref ids) = new_paper_ids {
        // Only find relations involving new papers
        let new_set: std::collections::HashSet<&str> = ids.iter().map(|s| s.as_str()).collect();
        let new_papers: Vec<_> = papers_with_meta.iter().filter(|p| new_set.contains(p.id.as_str())).cloned().collect();
        if new_papers.is_empty() {
            // Fallback: new papers don't have metadata yet, analyze all papers
            return ai_manager::recommend_relations(&papers_with_meta, &settings).await;
        }
        ai_manager::recommend_relations_for_new(&new_papers, &papers_with_meta, &settings).await
    } else {
        ai_manager::recommend_relations(&papers_with_meta, &settings).await
    }
}

#[tauri::command]
pub async fn ai_generate_insights(
    state: State<'_, AppState>,
    project_path: String,
) -> AppResult<Vec<Insight>> {
    let project = project_service::open_project(&project_path)?;
    let settings = state.ai_settings.lock().unwrap().clone();
    ai_manager::generate_insights(&project, &settings).await
}

#[tauri::command]
pub async fn test_ai_connection(settings: AiSettings) -> AppResult<bool> {
    ai_manager::test_connection(&settings).await
}

#[tauri::command]
pub async fn test_ai_connection_stored(state: State<'_, AppState>) -> AppResult<bool> {
    let settings = state.ai_settings.lock().unwrap().clone();
    ai_manager::test_connection(&settings).await
}

#[tauri::command]
pub async fn save_ai_settings(
    state: State<'_, AppState>,
    settings: AiSettings,
) -> AppResult<()> {
    let old = state.ai_settings.lock().unwrap().clone();
    let mut merged = settings.clone();

    // Preserve API keys if new ones are empty
    if let Some(ref new_cfg) = merged.openai_compatible {
        if new_cfg.api_key.is_empty() {
            if let Some(ref old_cfg) = old.openai_compatible {
                merged.openai_compatible.as_mut().unwrap().api_key = old_cfg.api_key.clone();
            }
        }
    }
    if let Some(ref new_cfg) = merged.anthropic {
        if new_cfg.api_key.is_empty() {
            if let Some(ref old_cfg) = old.anthropic {
                merged.anthropic.as_mut().unwrap().api_key = old_cfg.api_key.clone();
            }
        }
    }
    if let Some(ref new_mineru) = merged.mineru {
        if new_mineru.api_key.is_empty() {
            if let Some(ref old_mineru) = old.mineru {
                merged.mineru.as_mut().unwrap().api_key = old_mineru.api_key.clone();
            }
        }
    }
    if merged.embedding_api_key.as_ref().is_none_or(|k| k.is_empty()) {
        merged.embedding_api_key = old.embedding_api_key;
    }
    if merged.embedding_model.as_ref().is_none_or(|m| m.is_empty()) {
        merged.embedding_model = old.embedding_model;
    }
    if merged.embedding_base_url.as_ref().is_none_or(|u| u.is_empty()) {
        merged.embedding_base_url = old.embedding_base_url;
    }
    if merged.default_project_dir.as_ref().is_none_or(|d| d.is_empty()) {
        merged.default_project_dir = old.default_project_dir;
    }

    *state.ai_settings.lock().unwrap() = merged.clone();
    state::persist_settings(&merged);
    Ok(())
}

#[tauri::command]
pub async fn get_ai_settings_masked(
    state: State<'_, AppState>,
) -> AppResult<crate::models::MaskedAiSettings> {
    let s = state.ai_settings.lock().unwrap();
    Ok(crate::models::MaskedAiSettings {
        openai_compatible: s.openai_compatible.as_ref().map(|c| c.to_masked()),
        anthropic: s.anthropic.as_ref().map(|c| c.to_masked()),
        active_provider: s.active_provider.clone(),
        ocr_method: s.ocr_method.clone(),
        mineru: s.mineru.as_ref().map(|m| m.to_masked()),
        embedding_model: s.embedding_model.clone(),
        embedding_base_url: s.embedding_base_url.clone(),
        masked_embedding_api_key: s.embedding_api_key.as_ref().map(|k| {
            if k.len() <= 8 { "****".to_string() }
            else { format!("{}****{}", &k[..4], &k[k.len()-4..]) }
        }),
        default_project_dir: s.default_project_dir.clone(),
        http_api_enabled: s.http_api_enabled,
        http_api_port: s.http_api_port,
        advanced: s.advanced.clone(),
    })
}

#[tauri::command]
pub async fn toggle_http_server(
    state: State<'_, AppState>,
    enabled: bool,
) -> AppResult<bool> {
    let mut settings = state.ai_settings.lock().unwrap().clone();
    settings.http_api_enabled = enabled;

    if enabled {
        // Stop existing server if running
        let mut handle = state.http_server_handle.lock().unwrap();
        if let Some(mut h) = handle.take() {
            http_server::stop_http_server(&mut h);
        }

        // Start new server
        let shared = Arc::new(AppState {
            ai_settings: std::sync::Mutex::new(settings.clone()),
            http_server_handle: std::sync::Mutex::new(None),
        });
        // Share the actual ai_settings from the running state
        *shared.ai_settings.lock().unwrap() = state.ai_settings.lock().unwrap().clone();

        match http_server::start_http_server(shared, settings.http_api_port) {
            Ok(h) => {
                *handle = Some(h);
                *state.ai_settings.lock().unwrap() = settings;
                state::persist_settings(&state.ai_settings.lock().unwrap());
                Ok(true)
            }
            Err(e) => Err(crate::errors::AppError::Unknown(format!(
                "Failed to start HTTP server: {}",
                e
            ))),
        }
    } else {
        // Stop server
        let mut handle = state.http_server_handle.lock().unwrap();
        if let Some(mut h) = handle.take() {
            http_server::stop_http_server(&mut h);
        }
        *state.ai_settings.lock().unwrap() = settings;
        state::persist_settings(&state.ai_settings.lock().unwrap());
        Ok(false)
    }
}

#[tauri::command]
pub async fn get_http_server_status(
    state: State<'_, AppState>,
) -> AppResult<(bool, u16)> {
    let settings = state.ai_settings.lock().unwrap();
    Ok((settings.http_api_enabled, settings.http_api_port))
}
