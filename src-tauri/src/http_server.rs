use crate::errors::AppError;
use crate::models::*;
use crate::services::*;
use crate::state::AppState;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Json},
    routing::{delete, get, post},
    Router,
};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::oneshot;
use tower_http::cors::CorsLayer;

pub type SharedState = Arc<AppState>;

pub struct HttpServerHandle {
    pub task: tokio::task::JoinHandle<()>,
    pub shutdown_tx: Option<oneshot::Sender<()>>,
}

pub fn start_http_server(
    state: SharedState,
    port: u16,
) -> Result<HttpServerHandle, String> {
    let app = create_router(state);
    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));
    let listener = std::net::TcpListener::bind(addr)
        .map_err(|e| format!("Failed to bind port {}: {}", port, e))?;
    listener
        .set_nonblocking(true)
        .map_err(|e| format!("Failed to set non-blocking: {}", e))?;
    let tokio_listener = tokio::net::TcpListener::from_std(listener)
        .map_err(|e| format!("Failed to create tokio listener: {}", e))?;

    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

    let task = tokio::spawn(async move {
        let server = axum::serve(tokio_listener, app);
        server
            .with_graceful_shutdown(async {
                let _ = shutdown_rx.await;
            })
            .await
            .ok();
    });

    Ok(HttpServerHandle {
        task,
        shutdown_tx: Some(shutdown_tx),
    })
}

pub fn stop_http_server(handle: &mut HttpServerHandle) {
    if let Some(tx) = handle.shutdown_tx.take() {
        let _ = tx.send(());
    }
    handle.task.abort();
}

// ─── Error handling ───

struct ApiError(AppError);

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let (status, message) = match &self.0 {
            AppError::FileNotFound(_) => (StatusCode::NOT_FOUND, format!("{}", self.0)),
            AppError::IoError(_) => (StatusCode::INTERNAL_SERVER_ERROR, format!("{}", self.0)),
            AppError::AiError(_) => (StatusCode::BAD_GATEWAY, format!("{}", self.0)),
            _ => (StatusCode::INTERNAL_SERVER_ERROR, format!("{}", self.0)),
        };
        (status, Json(serde_json::json!({ "error": message }))).into_response()
    }
}

impl From<AppError> for ApiError {
    fn from(e: AppError) -> Self {
        ApiError(e)
    }
}

type ApiResult<T> = Result<T, ApiError>;

/// Validate and canonicalize a project path to prevent path traversal attacks.
/// Ensures the path resolves to a `.guanfu` directory.
fn validate_project_path(path: &str) -> Result<String, AppError> {
    let p = std::path::Path::new(path);
    // Reject paths with obvious traversal
    if path.contains('\0') {
        return Err(AppError::IoError("invalid path: null byte".into()));
    }
    let canonical = p.canonicalize()
        .map_err(|e| AppError::IoError(format!("invalid project path: {}", e)))?;
    if !canonical.to_string_lossy().ends_with(".guanfu") {
        return Err(AppError::IoError("path must point to a .guanfu directory".into()));
    }
    Ok(canonical.to_string_lossy().to_string())
}

// ─── Query / Body types ───

#[derive(Deserialize)]
struct ProjectPathQuery {
    #[serde(rename = "projectPath")]
    project_path: String,
}

#[derive(Deserialize)]
struct CreateProjectBody {
    name: String,
    #[serde(rename = "baseDir")]
    base_dir: String,
}

#[derive(Deserialize)]
struct OpenProjectBody {
    #[serde(rename = "projectPath")]
    project_path: String,
}

#[derive(Deserialize)]
struct ImportPapersBody {
    #[serde(rename = "projectPath")]
    project_path: String,
    #[serde(rename = "filePaths")]
    file_paths: Vec<String>,
}

#[derive(Deserialize)]
struct UpdatePaperBody {
    #[serde(rename = "projectPath")]
    project_path: String,
    paper: Paper,
}

#[derive(Deserialize)]
struct ProjectOnlyBody {
    #[serde(rename = "projectPath")]
    project_path: String,
}

#[derive(Deserialize)]
struct AddRelationBody {
    #[serde(rename = "projectPath")]
    project_path: String,
    relation: Relation,
}

#[derive(Deserialize)]
struct SaveLayoutBody {
    #[serde(rename = "projectPath")]
    project_path: String,
    layout: GraphLayout,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct ChatBody {
    #[serde(rename = "projectPath")]
    project_path: String,
    question: String,
    locale: Option<String>,
}

#[derive(Deserialize)]
struct UpdateRelationBody {
    #[serde(rename = "projectPath")]
    project_path: String,
    relation: Relation,
}

#[derive(Deserialize)]
struct BatchRelationsBody {
    #[serde(rename = "projectPath")]
    project_path: String,
    relations: Vec<Relation>,
}

#[derive(Deserialize)]
struct BatchAiParseBody {
    #[serde(rename = "projectPath")]
    project_path: String,
    #[serde(rename = "paperIds")]
    paper_ids: Vec<String>,
}

#[derive(Deserialize)]
struct SaveProjectBody {
    project: Project,
}

// ─── Router ───

pub fn create_router(state: SharedState) -> Router {
    Router::new()
        // Documentation
        .route("/", get(root_docs))
        .route("/health", get(health_check))
        .route("/openapi.json", get(openapi_spec))
        // Projects
        .route("/api/projects/create", post(create_project))
        .route("/api/projects/open", post(open_project))
        .route("/api/projects/save", post(save_project_handler))
        .route("/api/projects/recent", get(get_recent_project))
        .route("/api/projects/default-dir", get(get_default_dir))
        .route("/api/projects/info", get(get_project_info))
        // Papers
        .route("/api/papers/import", post(import_papers))
        .route("/api/papers/list", get(list_papers))
        .route("/api/papers/{paper_id}", delete(delete_paper))
        .route("/api/papers/{paper_id}/update", post(update_paper_handler))
        .route("/api/papers/{paper_id}/ai-parse", post(ai_parse_paper))
        .route("/api/papers/{paper_id}/extract-text", post(extract_text_handler))
        .route("/api/papers/batch-ai-parse", post(batch_ai_parse_handler))
        // Relations & Graph
        .route("/api/relations/add", post(add_relation_handler))
        .route("/api/relations/list", get(list_relations))
        .route("/api/relations/update", post(update_relation_handler))
        .route("/api/relations/batch-add", post(batch_add_relations_handler))
        .route("/api/relations/{relation_id}", delete(delete_relation_handler))
        .route("/api/graph/save-layout", post(save_layout_handler))
        // AI
        .route("/api/ai/recommend-relations", post(ai_recommend_relations))
        .route("/api/ai/generate-insights", post(ai_generate_insights))
        // Insights
        .route("/api/insights/run", post(run_insights_handler))
        .route("/api/insights/saved", get(get_saved_insights))
        // Chat
        .route("/api/chat/ask", post(chat_ask))
        .route("/api/chat/history", get(get_chat_history_handler))
        .route("/api/chat/build-embeddings", post(build_embeddings_handler))
        .layer(CorsLayer::permissive())
        .with_state(state)
}

// ─── Handlers: Documentation ───

async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok", "app": "观复", "version": "0.1.0" }))
}

async fn root_docs() -> Html<&'static str> {
    Html(include_str!("http_docs.html"))
}

async fn openapi_spec() -> Json<serde_json::Value> {
    let spec: serde_json::Value = serde_json::from_str(include_str!("openapi.json")).unwrap();
    Json(spec)
}

// ─── Handlers: Projects ───

async fn create_project(
    Json(body): Json<CreateProjectBody>,
) -> ApiResult<Json<Project>> {
    let project = project_service::create_project(&body.name, &body.base_dir)?;
    Ok(Json(project))
}

async fn open_project(
    Json(body): Json<OpenProjectBody>,
) -> ApiResult<Json<Project>> {
    let project = project_service::open_project(&body.project_path)?;
    Ok(Json(project))
}

async fn get_recent_project() -> Json<serde_json::Value> {
    let path = project_service::get_recent_project().ok().flatten();
    Json(serde_json::json!({ "projectPath": path }))
}

async fn get_default_dir() -> Json<serde_json::Value> {
    let dir = dirs_next::document_dir()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    Json(serde_json::json!({ "dir": dir }))
}

// ─── Handlers: Papers ───

async fn import_papers(
    Json(body): Json<ImportPapersBody>,
) -> ApiResult<Json<Vec<Paper>>> {
    let mut project = project_service::open_project(&body.project_path)?;
    let papers = paper_service::import_pdfs(&mut project, body.file_paths).await?;
    project_service::save_project(&project)?;
    Ok(Json(papers))
}

async fn list_papers(
    Query(q): Query<ProjectPathQuery>,
) -> ApiResult<Json<Vec<Paper>>> {
    let project = project_service::open_project(&q.project_path)?;
    Ok(Json(project.papers))
}

async fn delete_paper(
    Path(paper_id): Path<String>,
    Query(q): Query<ProjectPathQuery>,
) -> ApiResult<StatusCode> {
    let mut project = project_service::open_project(&q.project_path)?;
    paper_service::delete_paper(&mut project, &paper_id)?;
    project_service::save_project(&project)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn update_paper_handler(
    Path(paper_id): Path<String>,
    Json(body): Json<UpdatePaperBody>,
) -> ApiResult<Json<Paper>> {
    if body.paper.id != paper_id {
        return Err(ApiError(AppError::Unknown("paper ID mismatch".to_string())));
    }
    let mut project = project_service::open_project(&body.project_path)?;
    let paper = paper_service::update_paper(&mut project, body.paper)?;
    project_service::save_project(&project)?;
    Ok(Json(paper))
}

async fn ai_parse_paper(
    Path(paper_id): Path<String>,
    State(state): State<SharedState>,
    Json(body): Json<ProjectOnlyBody>,
) -> ApiResult<Json<ExtractedMetadata>> {
    let pp = validate_project_path(&body.project_path)?;
    let mut project = project_service::open_project(&pp)?;
    let paper = project
        .papers
        .iter()
        .find(|p| p.id == paper_id)
        .ok_or_else(|| AppError::FileNotFound(paper_id.clone()))?
        .clone();

    let pdf_path = std::path::Path::new(&pp)
        .join("papers")
        .join(&paper.file_path);

    let settings = state.ai_settings.lock().expect("ai_settings mutex poisoned").clone();

    let text = match settings.ocr_method {
        crate::models::OcrMethod::Local => {
            pdf_text_extractor::extract_text_with_ocr_fallback(&pdf_path)
                .await
                .unwrap_or_default()
        }
        crate::models::OcrMethod::Mineru => {
            if let Some(ref mineru) = settings.mineru {
                pdf_text_extractor::extract_text_mineru(
                    &pdf_path, &mineru.api_key, &mineru.api_base,
                ).await.unwrap_or_default()
            } else {
                pdf_text_extractor::extract_text_with_ocr_fallback(&pdf_path)
                    .await.unwrap_or_default()
            }
        }
        crate::models::OcrMethod::Agent => {
            pdf_text_extractor::extract_text_mineru_agent(&pdf_path)
                .await.unwrap_or_default()
        }
    };
    let text = if text.trim().is_empty() {
        paper.abstract_text.clone().unwrap_or_else(|| format!("Title: {}", paper.title))
    } else {
        text
    };

    let metadata = ai_manager::parse_text(&text, &settings).await?;

    // Save parsed data back to project
    if let Some(paper_idx) = project.papers.iter().position(|p| p.id == paper_id) {
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
    }

    Ok(Json(metadata))
}

// ─── Handlers: Relations & Graph ───

async fn add_relation_handler(
    Json(body): Json<AddRelationBody>,
) -> ApiResult<Json<Relation>> {
    let mut project = project_service::open_project(&body.project_path)?;
    project.relations.push(body.relation.clone());
    project_service::save_project(&project)?;
    Ok(Json(body.relation))
}

async fn delete_relation_handler(
    Path(relation_id): Path<String>,
    Query(q): Query<ProjectPathQuery>,
) -> ApiResult<StatusCode> {
    let mut project = project_service::open_project(&q.project_path)?;
    project.relations.retain(|r| r.id != relation_id);
    project_service::save_project(&project)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn save_layout_handler(
    Json(body): Json<SaveLayoutBody>,
) -> ApiResult<StatusCode> {
    let mut project = project_service::open_project(&body.project_path)?;
    project.graph_layout = body.layout;
    project_service::save_project(&project)?;
    Ok(StatusCode::OK)
}

// ─── Handlers: AI ───

async fn ai_recommend_relations(
    State(state): State<SharedState>,
    Json(body): Json<ProjectOnlyBody>,
) -> ApiResult<Json<Vec<RelationRecommendation>>> {
    let project = project_service::open_project(&body.project_path)?;
    let papers_with_meta: Vec<_> = project
        .papers
        .iter()
        .filter(|p| p.metadata.is_some())
        .cloned()
        .collect();

    if papers_with_meta.len() < 2 {
        return Ok(Json(vec![]));
    }

    let settings = state.ai_settings.lock().expect("ai_settings mutex poisoned").clone();
    let recommendations = ai_manager::recommend_relations(&papers_with_meta, &settings).await?;
    Ok(Json(recommendations))
}

async fn ai_generate_insights(
    State(state): State<SharedState>,
    Json(body): Json<ProjectOnlyBody>,
) -> ApiResult<Json<Vec<Insight>>> {
    let project = project_service::open_project(&body.project_path)?;
    let settings = state.ai_settings.lock().expect("ai_settings mutex poisoned").clone();
    let insights = ai_manager::generate_insights(&project, &settings).await?;
    Ok(Json(insights))
}

// ─── Handlers: Chat ───

async fn chat_ask(
    State(state): State<SharedState>,
    Json(body): Json<ChatBody>,
) -> ApiResult<Json<serde_json::Value>> {
    let mut settings = state.ai_settings.lock().expect("ai_settings mutex poisoned").clone();
    // Apply locale from request if provided
    if let Some(locale) = body.locale {
        settings.locale = Some(locale);
    }
    let answer = ai_manager::call_chat(&body.question, &settings).await?;
    Ok(Json(serde_json::json!({ "answer": answer })))
}

// ─── Handlers: New endpoints for full agent operability ───

async fn save_project_handler(
    Json(body): Json<SaveProjectBody>,
) -> ApiResult<StatusCode> {
    project_service::save_project(&body.project)?;
    Ok(StatusCode::OK)
}

async fn get_project_info(
    Query(q): Query<ProjectPathQuery>,
) -> ApiResult<Json<Project>> {
    let project = project_service::open_project(&q.project_path)?;
    Ok(Json(project))
}

async fn list_relations(
    Query(q): Query<ProjectPathQuery>,
) -> ApiResult<Json<Vec<Relation>>> {
    let project = project_service::open_project(&q.project_path)?;
    Ok(Json(project.relations))
}

async fn update_relation_handler(
    Json(body): Json<UpdateRelationBody>,
) -> ApiResult<Json<Relation>> {
    let mut project = project_service::open_project(&body.project_path)?;
    let idx = project
        .relations
        .iter()
        .position(|r| r.id == body.relation.id)
        .ok_or_else(|| AppError::Unknown(format!("relation not found: {}", body.relation.id)))?;
    project.relations[idx] = body.relation.clone();
    project_service::save_project(&project)?;
    Ok(Json(body.relation))
}

async fn batch_add_relations_handler(
    Json(body): Json<BatchRelationsBody>,
) -> ApiResult<Json<Vec<Relation>>> {
    let mut project = project_service::open_project(&body.project_path)?;
    project.relations.extend(body.relations.clone());
    project_service::save_project(&project)?;
    Ok(Json(body.relations))
}

async fn extract_text_handler(
    Path(paper_id): Path<String>,
    Json(body): Json<ProjectOnlyBody>,
) -> ApiResult<Json<serde_json::Value>> {
    let pp = validate_project_path(&body.project_path)?;
    let project = project_service::open_project(&pp)?;
    let paper = project
        .papers
        .iter()
        .find(|p| p.id == paper_id)
        .ok_or_else(|| AppError::FileNotFound(paper_id.clone()))?;

    let pdf_path = std::path::Path::new(&pp)
        .join("papers")
        .join(&paper.file_path);

    let text = pdf_text_extractor::extract_text_with_ocr_fallback(&pdf_path)
        .await
        .unwrap_or_default();
    Ok(Json(serde_json::json!({ "text": text })))
}

async fn batch_ai_parse_handler(
    State(state): State<SharedState>,
    Json(body): Json<BatchAiParseBody>,
) -> ApiResult<Json<HashMap<String, ExtractedMetadata>>> {
    let pp = validate_project_path(&body.project_path)?;
    let mut project = project_service::open_project(&pp)?;
    let settings = state.ai_settings.lock().expect("ai_settings mutex poisoned").clone();

    let concurrency = settings
        .advanced
        .as_ref()
        .map(|a| a.concurrency as usize)
        .unwrap_or(3);

    let mut work_items = Vec::new();
    for pid in &body.paper_ids {
        if let Some(paper) = project.papers.iter().find(|p| &p.id == pid) {
            let pdf_path = std::path::Path::new(&pp)
                .join("papers")
                .join(&paper.file_path);
            if pdf_path.exists() {
                work_items.push((pid.clone(), pdf_path));
            }
        }
    }

    let retry_count = settings
        .advanced
        .as_ref()
        .map(|a| a.retry_count as usize)
        .unwrap_or(1);
    let sem = std::sync::Arc::new(tokio::sync::Semaphore::new(concurrency));
    let mut handles = Vec::new();

    for (paper_id, pdf_path) in work_items {
        let sem = sem.clone();
        let s = settings.clone();
        handles.push(tokio::spawn(async move {
            let _permit = sem.acquire().await.expect("semaphore closed");
            let text = match s.ocr_method {
                crate::models::OcrMethod::Local => {
                    pdf_text_extractor::extract_text_with_ocr_fallback(&pdf_path)
                        .await.unwrap_or_default()
                }
                crate::models::OcrMethod::Mineru => {
                    if let Some(ref mineru) = s.mineru {
                        pdf_text_extractor::extract_text_mineru(
                            &pdf_path, &mineru.api_key, &mineru.api_base,
                        ).await.unwrap_or_default()
                    } else {
                        pdf_text_extractor::extract_text_with_ocr_fallback(&pdf_path)
                            .await.unwrap_or_default()
                    }
                }
                crate::models::OcrMethod::Agent => {
                    pdf_text_extractor::extract_text_mineru_agent(&pdf_path)
                        .await.unwrap_or_default()
                }
            };
            let text = if text.trim().is_empty() {
                format!("Paper ID: {}", paper_id)
            } else {
                text
            };

            let mut last_err = None;
            for attempt in 0..=retry_count {
                if attempt > 0 {
                    let delay_ms = 1000u64 * (1 << (attempt - 1).min(3));
                    tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
                }
                match ai_manager::parse_text(&text, &s).await {
                    Ok(m) => return (paper_id, Ok(m)),
                    Err(e) => last_err = Some(e),
                }
            }
            (paper_id, Err(last_err.unwrap()))
        }));
    }

    let mut results = HashMap::new();
    for h in handles {
        if let Ok((paper_id, result)) = h.await {
            if let Ok(metadata) = result {
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
        }
    }

    project_service::save_project(&project)?;
    Ok(Json(results))
}

async fn run_insights_handler(
    State(state): State<SharedState>,
    Json(body): Json<ProjectOnlyBody>,
) -> ApiResult<Json<Vec<Insight>>> {
    let project = project_service::open_project(&body.project_path)?;
    let mut insights = crate::commands::graph_commands::run_rule_based_insights(&project);

    let settings = state.ai_settings.lock().expect("ai_settings mutex poisoned").clone();
    let papers_with_meta: Vec<_> = project
        .papers
        .iter()
        .filter(|p| p.metadata.is_some())
        .collect();
    let has_ai = settings.active_provider.is_some()
        && (settings.openai_compatible.as_ref().map_or(false, |c| c.enabled && !c.api_key.is_empty())
            || settings.anthropic.as_ref().map_or(false, |c| c.enabled && !c.api_key.is_empty()));

    if has_ai && papers_with_meta.len() >= 2 {
        match ai_manager::generate_insights(&project, &settings).await {
            Ok(ai_insights) => insights.extend(ai_insights),
            Err(_) => { /* AI failed, keep rule-based insights */ }
        }
    }

    insights.dedup_by(|a, b| {
        a.r#type == b.r#type && a.related_paper_ids == b.related_paper_ids
    });

    Ok(Json(insights))
}

async fn get_saved_insights(
    Query(q): Query<ProjectPathQuery>,
) -> ApiResult<Json<Vec<Insight>>> {
    let pp = validate_project_path(&q.project_path)?;
    let path = std::path::Path::new(&pp).join("insights.json");
    if !path.exists() {
        return Ok(Json(vec![]));
    }
    let content = std::fs::read_to_string(&path)
        .map_err(|e| AppError::IoError(format!("failed to read insights: {}", e)))?;
    let insights: Vec<Insight> = serde_json::from_str(&content)
        .map_err(|e| AppError::Unknown(format!("failed to parse insights: {}", e)))?;
    Ok(Json(insights))
}

async fn get_chat_history_handler(
    Query(q): Query<ProjectPathQuery>,
) -> ApiResult<Json<Vec<crate::models::ChatMessage>>> {
    let pp = validate_project_path(&q.project_path)?;
    let path = std::path::Path::new(&pp).join("chat_history.json");
    if !path.exists() {
        return Ok(Json(vec![]));
    }
    let content = std::fs::read_to_string(&path)
        .map_err(|e| AppError::IoError(format!("failed to read chat history: {}", e)))?;
    let messages: Vec<crate::models::ChatMessage> = serde_json::from_str(&content)
        .map_err(|e| AppError::Unknown(format!("failed to parse chat history: {}", e)))?;
    Ok(Json(messages))
}

async fn build_embeddings_handler(
    State(state): State<SharedState>,
    Json(body): Json<ProjectOnlyBody>,
) -> ApiResult<Json<serde_json::Value>> {
    let pp = validate_project_path(&body.project_path)?;
    let project = project_service::open_project(&pp)?;
    let settings = state.ai_settings.lock().expect("ai_settings mutex poisoned").clone();

    let papers_with_meta: Vec<_> = project
        .papers
        .iter()
        .filter(|p| p.metadata.is_some())
        .collect();

    if papers_with_meta.is_empty() {
        return Err(ApiError(AppError::Unknown("no parsed papers found".to_string())));
    }

    let mut all_chunks: Vec<(String, String, String)> = Vec::new();
    for paper in &papers_with_meta {
        let chunks = crate::services::embedding_service::build_chunks_from_paper(paper);
        for (text, tag) in chunks {
            all_chunks.push((paper.id.clone(), text, tag));
        }
    }

    if all_chunks.is_empty() {
        return Err(ApiError(AppError::Unknown("no embeddable content found".to_string())));
    }

    let texts: Vec<String> = all_chunks.iter().map(|(_, t, _)| t.clone()).collect();
    let embeddings = crate::services::embedding_service::embed_texts(&texts, &settings).await?;

    let mut chunk_index = 0usize;
    let mut embedding_chunks = Vec::new();
    for (i, (paper_id, text, _)) in all_chunks.iter().enumerate() {
        if i < embeddings.len() {
            embedding_chunks.push(crate::models::EmbeddingChunk {
                paper_id: paper_id.clone(),
                chunk_index,
                text: text.clone(),
                embedding: embeddings[i].clone(),
            });
            chunk_index += 1;
        }
    }

    let store = crate::models::EmbeddingStore {
        chunks: embedding_chunks,
        model: settings.embedding_model.unwrap_or_else(|| "text-embedding-3-small".to_string()),
        updated_at: chrono::Utc::now().to_rfc3339(),
    };

    let store_path = std::path::Path::new(&pp).join("embeddings.json");
    let json = serde_json::to_string_pretty(&store)
        .map_err(|e| AppError::Unknown(format!("failed to serialize embeddings: {}", e)))?;
    std::fs::write(&store_path, json)
        .map_err(|e| AppError::IoError(format!("failed to write embeddings: {}", e)))?;

    Ok(Json(serde_json::json!({ "chunkCount": store.chunks.len() })))
}
