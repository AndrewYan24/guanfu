use crate::errors::{AppError, AppResult};
use crate::models::{ChatMessage, ChatSource, EmbeddingChunk, EmbeddingStore};
use crate::services::{embedding_service, project_service};
use crate::state::AppState;
use std::path::Path;
use tauri::{State, AppHandle};

#[tauri::command]
pub async fn chat_build_embeddings(
    state: State<'_, AppState>,
    project_path: String,
) -> AppResult<usize> {
    let project = project_service::open_project(&project_path)?;
    let settings = state.ai_settings.lock().unwrap().clone();

    let papers_with_meta: Vec<_> = project
        .papers
        .iter()
        .filter(|p| p.metadata.is_some())
        .collect();

    if papers_with_meta.is_empty() {
        return Err(AppError::Unknown("没有已解析的论文，请先完成论文解析".to_string()));
    }

    // Build all chunks
    let mut all_chunks: Vec<(String, String, String)> = Vec::new(); // (paper_id, text, tag)
    for paper in &papers_with_meta {
        let chunks = embedding_service::build_chunks_from_paper(paper);
        for (text, tag) in chunks {
            all_chunks.push((paper.id.clone(), text, tag));
        }
    }

    if all_chunks.is_empty() {
        return Err(AppError::Unknown("没有可嵌入的文本内容".to_string()));
    }

    // Embed all texts
    let texts: Vec<String> = all_chunks.iter().map(|(_, t, _)| t.clone()).collect();
    let embeddings = embedding_service::embed_texts(&texts, &settings).await?;

    // Build embedding chunks
    let mut chunk_index = 0usize;
    let mut embedding_chunks = Vec::new();
    for (i, (paper_id, text, _)) in all_chunks.iter().enumerate() {
        if i < embeddings.len() {
            embedding_chunks.push(EmbeddingChunk {
                paper_id: paper_id.clone(),
                chunk_index,
                text: text.clone(),
                embedding: embeddings[i].clone(),
            });
            chunk_index += 1;
        }
    }

    // Save to project directory
    let store = EmbeddingStore {
        chunks: embedding_chunks,
        model: settings.embedding_model.unwrap_or_else(|| "text-embedding-3-small".to_string()),
        updated_at: chrono::Utc::now().to_rfc3339(),
    };

    let store_path = embeddings_file_path(&project_path);
    let json = serde_json::to_string_pretty(&store)
        .map_err(|e| AppError::Unknown(format!("序列化嵌入数据失败: {}", e)))?;
    std::fs::write(&store_path, json)?;

    Ok(store.chunks.len())
}

#[tauri::command]
pub async fn chat_ask(
    state: State<'_, AppState>,
    project_path: String,
    question: String,
) -> AppResult<ChatMessage> {
    let project = project_service::open_project(&project_path)?;
    let settings = state.ai_settings.lock().unwrap().clone();

    // Load embeddings
    let store = load_embeddings(&project_path)?;

    if store.chunks.is_empty() {
        return Err(AppError::Unknown("尚未构建知识库索引，请先点击构建".to_string()));
    }

    // Embed the question
    let query_embedding = embedding_service::embed_query(&question, &settings).await?;

    // Search for relevant chunks
    let results = embedding_service::search_similar(&query_embedding, &store.chunks, 5);

    // Build context from retrieved chunks
    let mut sources = Vec::new();
    let mut context_parts = Vec::new();

    for (idx, similarity) in &results {
        let chunk = &store.chunks[*idx];
        if *similarity < 0.15 {
            continue;
        }

        let paper = project.papers.iter().find(|p| p.id == chunk.paper_id);
        let paper_title = paper.map(|p| p.title.clone()).unwrap_or_default();

        sources.push(ChatSource {
            paper_id: chunk.paper_id.clone(),
            paper_title: paper_title.clone(),
            chunk_text: chunk.text.clone(),
            similarity: *similarity,
        });

        // Add full metadata for high-similarity chunks
        if let Some(p) = paper {
            let meta_info = build_paper_context(p);
            context_parts.push(format!("【论文】{}\n{}", paper_title, meta_info));
        }
    }

    // Add graph relations summary
    let relations_summary = build_relations_summary(&project);
    if !relations_summary.is_empty() {
        context_parts.push(format!("【图谱关系摘要】\n{}", relations_summary));
    }

    // Add insights summary (from rule-based analysis)
    let insights = crate::commands::graph_commands::run_rule_based_insights(&project);
    if !insights.is_empty() {
        let insights_text: Vec<String> = insights
            .iter()
            .take(5)
            .map(|i| format!("- [{}] {}", i.title, i.description))
            .collect();
        context_parts.push(format!("【研究洞察】\n{}", insights_text.join("\n")));
    }

    let context = context_parts.join("\n\n---\n\n");

    // Build conversation prompt
    let user_prompt = format!(
        "## 参考资料\n\n{}\n\n## 用户问题\n\n{}",
        context, question
    );

    // Call chat AI
    let answer = crate::services::ai_manager::call_chat(&user_prompt, &settings).await?;

    // Save to chat history
    let message = ChatMessage {
        role: "assistant".to_string(),
        content: answer,
        sources,
        created_at: chrono::Utc::now().to_rfc3339(),
    };

    save_chat_message(&project_path, &ChatMessage {
        role: "user".to_string(),
        content: question,
        sources: vec![],
        created_at: chrono::Utc::now().to_rfc3339(),
    })?;
    save_chat_message(&project_path, &message)?;

    Ok(message)
}

#[tauri::command]
pub async fn chat_ask_stream(
    state: State<'_, AppState>,
    app_handle: AppHandle,
    project_path: String,
    question: String,
) -> AppResult<ChatMessage> {
    let project = project_service::open_project(&project_path)?;
    let settings = state.ai_settings.lock().unwrap().clone();

    // Load embeddings
    let store = load_embeddings(&project_path)?;

    if store.chunks.is_empty() {
        return Err(AppError::Unknown("尚未构建知识库索引，请先点击构建".to_string()));
    }

    // Embed the question
    let query_embedding = embedding_service::embed_query(&question, &settings).await?;

    // Search for relevant chunks
    let results = embedding_service::search_similar(&query_embedding, &store.chunks, 5);

    // Build context from retrieved chunks
    let mut sources = Vec::new();
    let mut context_parts = Vec::new();

    for (idx, similarity) in &results {
        let chunk = &store.chunks[*idx];
        if *similarity < 0.15 {
            continue;
        }

        let paper = project.papers.iter().find(|p| p.id == chunk.paper_id);
        let paper_title = paper.map(|p| p.title.clone()).unwrap_or_default();

        sources.push(ChatSource {
            paper_id: chunk.paper_id.clone(),
            paper_title: paper_title.clone(),
            chunk_text: chunk.text.clone(),
            similarity: *similarity,
        });

        if let Some(p) = paper {
            let meta_info = build_paper_context(p);
            context_parts.push(format!("【论文】{}\n{}", paper_title, meta_info));
        }
    }

    let relations_summary = build_relations_summary(&project);
    if !relations_summary.is_empty() {
        context_parts.push(format!("【图谱关系摘要】\n{}", relations_summary));
    }

    let insights = crate::commands::graph_commands::run_rule_based_insights(&project);
    if !insights.is_empty() {
        let insights_text: Vec<String> = insights
            .iter()
            .take(5)
            .map(|i| format!("- [{}] {}", i.title, i.description))
            .collect();
        context_parts.push(format!("【研究洞察】\n{}", insights_text.join("\n")));
    }

    let context = context_parts.join("\n\n---\n\n");

    let user_prompt = format!(
        "## 参考资料\n\n{}\n\n## 用户问题\n\n{}",
        context, question
    );

    // Call streaming chat AI
    let answer = crate::services::ai_manager::call_chat_stream(
        &user_prompt, &settings, &app_handle
    ).await?;

    let message = ChatMessage {
        role: "assistant".to_string(),
        content: answer,
        sources,
        created_at: chrono::Utc::now().to_rfc3339(),
    };

    save_chat_message(&project_path, &ChatMessage {
        role: "user".to_string(),
        content: question,
        sources: vec![],
        created_at: chrono::Utc::now().to_rfc3339(),
    })?;
    save_chat_message(&project_path, &message)?;

    Ok(message)
}

#[tauri::command]
pub async fn get_chat_history(
    project_path: String,
) -> AppResult<Vec<ChatMessage>> {
    let history_path = chat_history_file_path(&project_path);
    if !history_path.exists() {
        return Ok(vec![]);
    }
    let content = std::fs::read_to_string(&history_path)?;
    let messages: Vec<ChatMessage> = serde_json::from_str(&content)
        .map_err(|e| AppError::Unknown(format!("解析聊天记录失败: {}", e)))?;
    Ok(messages)
}

fn embeddings_file_path(project_path: &str) -> std::path::PathBuf {
    Path::new(project_path).join("embeddings.json")
}

fn chat_history_file_path(project_path: &str) -> std::path::PathBuf {
    Path::new(project_path).join("chat_history.json")
}

fn load_embeddings(project_path: &str) -> AppResult<EmbeddingStore> {
    let path = embeddings_file_path(project_path);
    if !path.exists() {
        return Ok(EmbeddingStore {
            chunks: vec![],
            model: String::new(),
            updated_at: String::new(),
        });
    }
    let content = std::fs::read_to_string(&path)?;
    let store: EmbeddingStore = serde_json::from_str(&content)
        .map_err(|e| AppError::Unknown(format!("解析嵌入数据失败: {}", e)))?;
    Ok(store)
}

fn save_chat_message(project_path: &str, message: &ChatMessage) -> AppResult<()> {
    let path = chat_history_file_path(project_path);
    let mut messages: Vec<ChatMessage> = if path.exists() {
        let content = std::fs::read_to_string(&path)?;
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        vec![]
    };
    messages.push(message.clone());
    let json = serde_json::to_string(&messages)
        .map_err(|e| AppError::Unknown(format!("序列化聊天记录失败: {}", e)))?;
    std::fs::write(&path, json)?;
    Ok(())
}

fn build_paper_context(paper: &crate::models::Paper) -> String {
    let mut parts = Vec::new();

    if !paper.authors.is_empty() {
        parts.push(format!("作者: {}", paper.authors.join(", ")));
    }
    if let Some(year) = paper.year {
        parts.push(format!("年份: {}", year));
    }
    if let Some(ref abstract_text) = paper.abstract_text {
        if !abstract_text.trim().is_empty() {
            parts.push(format!("摘要: {}", truncate(abstract_text, 300)));
        }
    }

    if let Some(ref meta) = paper.metadata {
        if !meta.research_question.trim().is_empty() {
            parts.push(format!("研究问题: {}", truncate(&meta.research_question, 300)));
        }
        if !meta.core_claim.trim().is_empty() {
            parts.push(format!("核心主张: {}", truncate(&meta.core_claim, 300)));
        }
        if !meta.methodology.trim().is_empty() {
            parts.push(format!("方法: {}", truncate(&meta.methodology, 200)));
        }
        if !meta.findings.trim().is_empty() {
            parts.push(format!("发现: {}", truncate(&meta.findings, 300)));
        }
        if !meta.self_positioning.trim().is_empty() {
            parts.push(format!("定位: {}", truncate(&meta.self_positioning, 200)));
        }
    }

    parts.join("\n")
}

fn build_relations_summary(project: &crate::models::Project) -> String {
    if project.relations.is_empty() {
        return String::new();
    }

    let type_labels: std::collections::HashMap<&str, &str> = [
        ("supports", "支持"),
        ("opposes", "反对"),
        ("modifies", "修正"),
        ("adopts", "继承"),
        ("reinterprets", "再诠释"),
    ].iter().cloned().collect();

    let summaries: Vec<String> = project.relations.iter().take(20).map(|r| {
        let source_title = project.papers.iter()
            .find(|p| p.id == r.source_id)
            .map(|p| p.title.clone())
            .unwrap_or_else(|| r.source_id.clone());
        let target_title = project.papers.iter()
            .find(|p| p.id == r.target_id)
            .map(|p| p.title.clone())
            .unwrap_or_else(|| r.target_id.clone());
        let rel_label = type_labels.get(r.r#type.as_str()).copied().unwrap_or(r.r#type.as_str());
        format!("「{}」{}「{}」", source_title, rel_label, target_title)
    }).collect();

    summaries.join("；")
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max).collect();
        format!("{}…", truncated)
    }
}
