use crate::errors::{AppError, AppResult};
use crate::models::{AiSettings, EmbeddingChunk};

/// Generate embeddings for a batch of texts using OpenAI-compatible API.
pub async fn embed_texts(
    texts: &[String],
    settings: &AiSettings,
) -> AppResult<Vec<Vec<f32>>> {
    let (api_key, base_url, model) = resolve_embedding_config(settings)?;

    let client = reqwest::Client::new();
    let url = format!("{}/embeddings", base_url.trim_end_matches('/'));

    let mut all_embeddings = Vec::with_capacity(texts.len());

    // Batch in groups of 96 to stay within API limits
    for chunk in texts.chunks(96) {
        let body = serde_json::json!({
            "model": model,
            "input": chunk,
        });

        let resp = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| AppError::Unknown(format!("Embedding 请求失败: {}", e)))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let err_text = resp.text().await.unwrap_or_default();
            return Err(AppError::Unknown(format!(
                "Embedding API 错误 {}: {}",
                status, err_text
            )));
        }

        let resp_json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| AppError::Unknown(format!("解析 Embedding 响应失败: {}", e)))?;

        let data = resp_json["data"]
            .as_array()
            .ok_or_else(|| AppError::Unknown("Embedding 响应缺少 data 字段".to_string()))?;

        for item in data {
            let embedding: Vec<f32> = item["embedding"]
                .as_array()
                .ok_or_else(|| AppError::Unknown("Embedding 项缺少 embedding 字段".to_string()))?
                .iter()
                .filter_map(|v| v.as_f64().map(|f| f as f32))
                .collect();
            all_embeddings.push(embedding);
        }
    }

    Ok(all_embeddings)
}

/// Embed a single query text, returning the embedding vector.
pub async fn embed_query(
    query: &str,
    settings: &AiSettings,
) -> AppResult<Vec<f32>> {
    let results = embed_texts(&[query.to_string()], settings).await?;
    results
        .into_iter()
        .next()
        .ok_or_else(|| AppError::Unknown("Embedding 返回为空".to_string()))
}

/// Compute cosine similarity between two vectors.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f64 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }

    let mut dot = 0.0f64;
    let mut norm_a = 0.0f64;
    let mut norm_b = 0.0f64;

    for i in 0..a.len() {
        let ai = a[i] as f64;
        let bi = b[i] as f64;
        dot += ai * bi;
        norm_a += ai * ai;
        norm_b += bi * bi;
    }

    let denom = norm_a.sqrt() * norm_b.sqrt();
    if denom == 0.0 {
        0.0
    } else {
        dot / denom
    }
}

/// Search for the top-k most similar chunks to the query embedding.
pub fn search_similar(
    query_embedding: &[f32],
    chunks: &[EmbeddingChunk],
    top_k: usize,
) -> Vec<(usize, f64)> {
    let mut scored: Vec<(usize, f64)> = chunks
        .iter()
        .enumerate()
        .map(|(i, chunk)| (i, cosine_similarity(query_embedding, &chunk.embedding)))
        .collect();

    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(top_k);
    scored
}

/// Resolve embedding API config from AiSettings.
fn resolve_embedding_config(settings: &AiSettings) -> AppResult<(String, String, String)> {
    let model = settings
        .embedding_model
        .clone()
        .unwrap_or_else(|| "text-embedding-3-small".to_string());

    // Prefer dedicated embedding API key, fall back to any provider's key
    let api_key = settings.embedding_api_key.as_ref()
        .filter(|k| !k.is_empty())
        .cloned()
        .or_else(|| {
            settings.openai_compatible.as_ref()
                .filter(|c| !c.api_key.is_empty())
                .map(|c| c.api_key.clone())
        })
        .or_else(|| {
            settings.anthropic.as_ref()
                .filter(|c| !c.api_key.is_empty())
                .map(|c| c.api_key.clone())
        });

    let api_key = api_key.ok_or_else(|| AppError::AiError(
        "未配置嵌入 API Key，请在设置中填写".to_string()
    ))?;

    // Base URL: embedding_base_url > openai_compatible base_url > default
    let base_url = settings.embedding_base_url.clone()
        .unwrap_or_else(|| {
            settings.openai_compatible.as_ref()
                .and_then(|c| c.base_url.clone())
                .unwrap_or_else(|| "https://api.openai.com/v1".to_string())
        });

    Ok((api_key, base_url, model))
}

/// Generate text chunks from paper metadata for embedding.
pub fn build_chunks_from_paper(paper: &crate::models::Paper) -> Vec<(String, String)> {
    let mut chunks = Vec::new();

    // Basic info chunk
    let basic = format!(
        "{}. {}. {}.",
        paper.title,
        paper.authors.join(", "),
        paper.year.map(|y| y.to_string()).unwrap_or_default()
    );
    if let Some(ref abstract_text) = paper.abstract_text {
        if !abstract_text.trim().is_empty() {
            chunks.push((format!("{} {}", basic, truncate(abstract_text, 500)), "basic".to_string()));
        } else {
            chunks.push((basic, "basic".to_string()));
        }
    } else {
        chunks.push((basic, "basic".to_string()));
    }

    if let Some(ref meta) = paper.metadata {
        if !meta.research_question.trim().is_empty() {
            chunks.push((format!("研究问题: {}", truncate(&meta.research_question, 500)), "research_question".to_string()));
        }
        if !meta.core_claim.trim().is_empty() {
            chunks.push((format!("核心主张: {}", truncate(&meta.core_claim, 500)), "core_claim".to_string()));
        }

        let theory_method = format!(
            "理论视角: {}. 研究方法: {}.",
            truncate(&meta.theoretical_perspective, 300),
            truncate(&meta.methodology, 300)
        );
        if meta.theoretical_perspective.trim().len() + meta.methodology.trim().len() > 0 {
            chunks.push((theory_method, "theory_method".to_string()));
        }

        let findings_limits = format!(
            "主要发现: {}. 局限性: {}.",
            truncate(&meta.findings, 300),
            truncate(&meta.limitations, 300)
        );
        if meta.findings.trim().len() + meta.limitations.trim().len() > 0 {
            chunks.push((findings_limits, "findings_limits".to_string()));
        }

        if !meta.self_positioning.trim().is_empty() {
            chunks.push((format!("学术定位: {}", truncate(&meta.self_positioning, 500)), "positioning".to_string()));
        }
    }

    chunks
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max).collect();
        format!("{}…", truncated)
    }
}
