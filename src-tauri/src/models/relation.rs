use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Relation {
    pub id: String,
    pub source_id: String,
    pub target_id: String,
    pub r#type: String,
    pub evidence: String,
    pub is_manual: bool,
    pub confidence: Option<f64>,
    pub created_at: String,
    pub updated_at: String,
}

impl Relation {
    pub fn new(source_id: String, target_id: String, r#type: String, evidence: String) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            source_id,
            target_id,
            r#type,
            evidence,
            is_manual: true,
            confidence: None,
            created_at: now.clone(),
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RelationRecommendation {
    pub source_id: String,
    pub target_id: String,
    pub r#type: String,
    pub confidence: f64,
    pub evidence: String,
    #[serde(default)]
    pub discovery_method: Option<String>,
}
