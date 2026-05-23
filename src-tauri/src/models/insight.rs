use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Insight {
    pub id: String,
    pub r#type: String,
    pub title: String,
    pub description: String,
    pub related_paper_ids: Vec<String>,
    pub created_at: String,
}
