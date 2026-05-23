use crate::errors::{AppError, AppResult};
use crate::models::{GraphLayout, Insight, Relation};
use crate::services::{ai_manager, project_service};
use crate::state::AppState;
use std::path::Path;
use tauri::State;

#[tauri::command]
pub async fn add_relation(project_path: String, relation: Relation) -> AppResult<Relation> {
    let mut project = project_service::open_project(&project_path)?;
    project.relations.push(relation.clone());
    project_service::save_project(&project)?;
    Ok(relation)
}

#[tauri::command]
pub async fn update_relation(project_path: String, relation: Relation) -> AppResult<Relation> {
    let mut project = project_service::open_project(&project_path)?;
    let idx = project
        .relations
        .iter()
        .position(|r| r.id == relation.id)
        .ok_or_else(|| AppError::Unknown(format!("关系不存在: {}", relation.id)))?;
    project.relations[idx] = relation.clone();
    project_service::save_project(&project)?;
    Ok(relation)
}

#[tauri::command]
pub async fn delete_relation(project_path: String, relation_id: String) -> AppResult<()> {
    let mut project = project_service::open_project(&project_path)?;
    project.relations.retain(|r| r.id != relation_id);
    project_service::save_project(&project)?;
    Ok(())
}

#[tauri::command]
pub async fn save_graph_layout(project_path: String, layout: GraphLayout) -> AppResult<()> {
    let mut project = project_service::open_project(&project_path)?;
    project.graph_layout = layout;
    project_service::save_project(&project)?;
    Ok(())
}

#[tauri::command]
pub async fn run_insight_analysis(
    state: State<'_, AppState>,
    project_path: String,
) -> AppResult<Vec<Insight>> {
    let project = project_service::open_project(&project_path)?;
    let mut insights = run_rule_based_insights(&project);

    // AI insights (if configured)
    let settings = state.ai_settings.lock().unwrap().clone();
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

    // Deduplicate by (type, related_paper_ids) to avoid duplicates from rules + AI
    insights.dedup_by(|a, b| {
        a.r#type == b.r#type && a.related_paper_ids == b.related_paper_ids
    });

    Ok(insights)
}

/// Run rule-based insight analysis without AI. Returns Vec<Insight>.
pub fn run_rule_based_insights(project: &crate::models::Project) -> Vec<Insight> {
    let mut insights = Vec::new();

    let papers_with_meta: Vec<_> = project
        .papers
        .iter()
        .filter(|p| p.metadata.is_some())
        .collect();

    // Rule 1: Potential fault lines — related papers with no relations
    // Uses keyword overlap instead of exact match
    for i in 0..papers_with_meta.len() {
        for j in (i + 1)..papers_with_meta.len() {
            let a = papers_with_meta[i];
            let b = papers_with_meta[j];

            let has_relation = project.relations.iter().any(|r| {
                (r.source_id == a.id && r.target_id == b.id)
                    || (r.source_id == b.id && r.target_id == a.id)
            });

            if has_relation {
                continue;
            }

            let ma = a.metadata.as_ref().unwrap();
            let mb = b.metadata.as_ref().unwrap();

            let perspective_overlap = keyword_overlap(&ma.theoretical_perspective, &mb.theoretical_perspective);
            let claim_overlap = keyword_overlap(&ma.core_claim, &mb.core_claim);
            let method_overlap = keyword_overlap(&ma.methodology, &mb.methodology);

            // Trigger if any dimension has significant overlap
            if perspective_overlap >= 0.3 || claim_overlap >= 0.25 || method_overlap >= 0.35 {
                let reason = if perspective_overlap >= 0.3 {
                    format!("理论视角存在共通之处")
                } else if claim_overlap >= 0.25 {
                    format!("核心主张存在关联")
                } else {
                    format!("研究方法相似")
                };

                insights.push(Insight {
                    id: uuid::Uuid::new_v4().to_string(),
                    r#type: "potential-fault-line".to_string(),
                    title: "潜在断裂带".to_string(),
                    description: format!(
                        "「{}」和「{}」{}，但之间没有建立论争关系。这可能是一个未被发现的研究连接点。",
                        a.title, b.title, reason
                    ),
                    related_paper_ids: vec![a.id.clone(), b.id.clone()],
                    created_at: chrono::Utc::now().to_rfc3339(),
                });
            }
        }
    }

    // Rule 2: Lack of pluralistic testing
    for paper in &project.papers {
        let supports_count = project
            .relations
            .iter()
            .filter(|r| r.target_id == paper.id && r.r#type == "supports")
            .count();
        let opposes_count = project
            .relations
            .iter()
            .filter(|r| r.target_id == paper.id && r.r#type == "opposes")
            .count();
        let modifies_count = project
            .relations
            .iter()
            .filter(|r| r.target_id == paper.id && r.r#type == "modifies")
            .count();

        let total_incoming = supports_count + opposes_count + modifies_count;

        if total_incoming >= 2 && opposes_count == 0 && modifies_count == 0 {
            insights.push(Insight {
                id: uuid::Uuid::new_v4().to_string(),
                r#type: "lack-pluralistic-testing".to_string(),
                title: "缺乏多元检验".to_string(),
                description: format!(
                    "「{}」被 {} 篇文献支持，但没有任何反对或修正。学术观点的可靠性需要反面证据的检验。",
                    paper.title, supports_count
                ),
                related_paper_ids: vec![paper.id.clone()],
                created_at: chrono::Utc::now().to_rfc3339(),
            });
        }
    }

    // Rule 3: Method homogeneity — cluster of papers all using similar methods
    let papers_with_methods: Vec<_> = papers_with_meta
        .iter()
        .filter(|p| !p.metadata.as_ref().unwrap().methodology.is_empty())
        .collect();

    if papers_with_methods.len() >= 3 {
        for i in 0..papers_with_methods.len() {
            for j in (i + 1)..papers_with_methods.len() {
                for k in (j + 1)..papers_with_methods.len() {
                    let a = papers_with_methods[i];
                    let b = papers_with_methods[j];
                    let c = papers_with_methods[k];

                    let ma = a.metadata.as_ref().unwrap();
                    let mb = b.metadata.as_ref().unwrap();
                    let mc = c.metadata.as_ref().unwrap();

                    let ab = keyword_overlap(&ma.methodology, &mb.methodology);
                    let ac = keyword_overlap(&ma.methodology, &mc.methodology);
                    let bc = keyword_overlap(&mb.methodology, &mc.methodology);

                    if ab >= 0.35 && ac >= 0.35 && bc >= 0.35 {
                        // Check if there's variety in relation types among them
                        let rel_types: Vec<_> = project.relations.iter()
                            .filter(|r| {
                                (r.source_id == a.id || r.source_id == b.id || r.source_id == c.id)
                                    && (r.target_id == a.id || r.target_id == b.id || r.target_id == c.id)
                            })
                            .map(|r| r.r#type.clone())
                            .collect();

                        let has_opposes = rel_types.iter().any(|t| t == "opposes");
                        let has_modifies = rel_types.iter().any(|t| t == "modifies");

                        if !has_opposes && !has_modifies {
                            insights.push(Insight {
                                id: uuid::Uuid::new_v4().to_string(),
                                r#type: "method-homogeneity".to_string(),
                                title: "方法单一".to_string(),
                                description: format!(
                                    "「{}」「{}」「{}」的研究方法高度相似，缺乏方法论多样性。可考虑补充不同方法（如定量/定性对比）的研究。",
                                    a.title, b.title, c.title
                                ),
                                related_paper_ids: vec![a.id.clone(), b.id.clone(), c.id.clone()],
                                created_at: chrono::Utc::now().to_rfc3339(),
                            });
                        }
                    }
                }
            }
        }
    }

    // Rule 4: Isolated papers — papers with no relations at all
    for paper in &project.papers {
        let relation_count = project.relations.iter()
            .filter(|r| r.source_id == paper.id || r.target_id == paper.id)
            .count();

        if relation_count == 0 && papers_with_meta.len() >= 3 {
            insights.push(Insight {
                id: uuid::Uuid::new_v4().to_string(),
                r#type: "potential-fault-line".to_string(),
                title: "孤立文献".to_string(),
                description: format!(
                    "「{}」在图谱中没有任何关系连接。这篇文献可能需要与其他文献建立论争关系。",
                    paper.title
                ),
                related_paper_ids: vec![paper.id.clone()],
                created_at: chrono::Utc::now().to_rfc3339(),
            });
        }
    }

    // General overview if nothing else found
    if insights.is_empty() && !project.papers.is_empty() {
        insights.push(Insight {
            id: uuid::Uuid::new_v4().to_string(),
            r#type: "potential-fault-line".to_string(),
            title: "图谱概览".to_string(),
            description: format!(
                "项目包含 {} 篇文献和 {} 条关系。继续添加文献和关系以发现更多研究空白与机会。",
                project.papers.len(),
                project.relations.len()
            ),
            related_paper_ids: vec![],
            created_at: chrono::Utc::now().to_rfc3339(),
        });
    }

    insights
}

/// Extract meaningful keywords from text (Chinese + English), filtering stopwords.
fn extract_keywords(text: &str) -> Vec<String> {
    let stopwords: &[&str] = &[
        "的", "了", "在", "是", "我", "有", "和", "就", "不", "人", "都", "一", "一个",
        "上", "也", "很", "到", "说", "要", "去", "你", "会", "着", "没有", "看", "好",
        "自己", "这", "他", "她", "它", "们", "那", "被", "从", "把", "对", "为", "以",
        "与", "及", "或", "等", "中", "通过", "利用", "采用", "使用", "基于", "进行",
        "研究", "分析", "本文", "论文", "文章", "作者", "认为", "指出", "表明", "发现",
        "提出", "探讨", "阐述", "论述", "以及", "因此", "然而", "但是", "不过",
        "the", "and", "for", "are", "but", "not", "you", "all", "can", "her", "was",
        "one", "our", "out", "this", "that", "with", "have", "from", "they", "been",
        "said", "each", "which", "their", "will", "other", "about", "many", "then",
        "them", "these", "some", "would", "make", "like", "into", "has", "more",
    ];

    text.chars()
        .collect::<Vec<_>>()
        .windows(2)
        .map(|w| w.iter().collect::<String>())
        .chain(
            text.split_whitespace()
                .map(|w| w.to_lowercase())
                .filter(|w| w.len() > 1),
        )
        .filter(|w| !stopwords.iter().any(|s| s == w))
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect()
}

/// Compute Jaccard-like overlap between two texts based on keyword extraction.
fn keyword_overlap(a: &str, b: &str) -> f64 {
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }

    let ka = extract_keywords(a);
    let kb = extract_keywords(b);

    if ka.is_empty() || kb.is_empty() {
        return 0.0;
    }

    let ka_set: std::collections::HashSet<&String> = ka.iter().collect();
    let kb_set: std::collections::HashSet<&String> = kb.iter().collect();

    let intersection = ka_set.intersection(&kb_set).count();
    let union = ka_set.union(&kb_set).count();

    if union == 0 { 0.0 } else { intersection as f64 / union as f64 }
}

fn insights_file_path(project_path: &str) -> std::path::PathBuf {
    Path::new(project_path).join("insights.json")
}

#[tauri::command]
pub async fn load_saved_insights(project_path: String) -> AppResult<Vec<Insight>> {
    let path = insights_file_path(&project_path);
    if !path.exists() {
        return Ok(vec![]);
    }
    let content = std::fs::read_to_string(&path)?;
    let insights: Vec<Insight> = serde_json::from_str(&content)
        .map_err(|e| AppError::Unknown(format!("解析洞察数据失败: {}", e)))?;
    Ok(insights)
}

#[tauri::command]
pub async fn save_insights(project_path: String, insights: Vec<Insight>) -> AppResult<()> {
    let path = insights_file_path(&project_path);
    let json = serde_json::to_string_pretty(&insights)
        .map_err(|e| AppError::Unknown(format!("序列化洞察数据失败: {}", e)))?;
    std::fs::write(&path, json)?;
    Ok(())
}
