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
        .route("/api/projects/recent", get(get_recent_project))
        .route("/api/projects/default-dir", get(get_default_dir))
        // Papers
        .route("/api/papers/import", post(import_papers))
        .route("/api/papers/list", get(list_papers))
        .route("/api/papers/{paper_id}", delete(delete_paper))
        .route("/api/papers/{paper_id}/update", post(update_paper_handler))
        .route("/api/papers/{paper_id}/ai-parse", post(ai_parse_paper))
        // Relations & Graph
        .route("/api/relations/add", post(add_relation_handler))
        .route("/api/relations/{relation_id}", delete(delete_relation_handler))
        .route("/api/graph/save-layout", post(save_layout_handler))
        // AI
        .route("/api/ai/recommend-relations", post(ai_recommend_relations))
        .route("/api/ai/generate-insights", post(ai_generate_insights))
        // Chat
        .route("/api/chat/ask", post(chat_ask))
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
    let mut project = project_service::open_project(&body.project_path)?;
    let paper = project
        .papers
        .iter()
        .find(|p| p.id == paper_id)
        .ok_or_else(|| AppError::FileNotFound(paper_id.clone()))?
        .clone();

    let pdf_path = std::path::Path::new(&body.project_path)
        .join("papers")
        .join(&paper.file_path);

    let settings = state.ai_settings.lock().expect("ai_settings mutex poisoned").clone();

    let text = pdf_text_extractor::extract_text_with_ocr_fallback(&pdf_path)
        .await
        .unwrap_or_default();
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
