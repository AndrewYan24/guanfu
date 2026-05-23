use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtractedMetadata {
    pub title: Option<String>,
    pub authors: Option<Vec<String>>,
    pub year: Option<i32>,
    pub abstract_text: Option<String>,
    pub research_question: String,
    pub core_claim: String,
    pub assumptions: String,
    pub theoretical_perspective: String,
    pub methodology: String,
    pub findings: String,
    pub limitations: String,
    pub self_positioning: String,
    pub version: i32,
    pub last_updated: String,
    pub source: String,
    pub is_ai_generated: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Paper {
    pub id: String,
    pub title: String,
    pub authors: Vec<String>,
    pub year: Option<i32>,
    #[serde(rename = "abstract")]
    pub abstract_text: Option<String>,
    pub file_path: String,
    pub metadata: Option<ExtractedMetadata>,
    pub tags: Vec<String>,
    pub notes: String,
    pub created_at: String,
    pub updated_at: String,
}

impl Paper {
    pub fn new(title: String, file_path: String) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            title,
            authors: Vec::new(),
            year: None,
            abstract_text: None,
            file_path,
            metadata: None,
            tags: Vec::new(),
            notes: String::new(),
            created_at: now.clone(),
            updated_at: now,
        }
    }
}
