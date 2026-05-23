use serde::{Deserialize, Serialize};

use super::{Annotation, Paper, Relation};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphPosition {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphLayout {
    pub locked: bool,
    #[serde(default)]
    pub zoom: Option<f64>,
    #[serde(default)]
    pub pan: Option<GraphPosition>,
    pub positions: std::collections::HashMap<String, GraphPosition>,
}

impl Default for GraphLayout {
    fn default() -> Self {
        Self {
            locked: false,
            zoom: None,
            pan: None,
            positions: std::collections::HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSettings {
    pub active_ai_provider: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: String,
    pub name: String,
    pub path: String,
    pub papers: Vec<Paper>,
    pub relations: Vec<Relation>,
    pub annotations: Vec<Annotation>,
    pub graph_layout: GraphLayout,
    pub settings: Option<ProjectSettings>,
    pub created_at: String,
    pub updated_at: String,
}

impl Project {
    pub fn new(name: String, path: String) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            path,
            papers: Vec::new(),
            relations: Vec::new(),
            annotations: Vec::new(),
            graph_layout: GraphLayout::default(),
            settings: None,
            created_at: now.clone(),
            updated_at: now,
        }
    }
}
