use crate::errors::AppResult;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CrossRefWork {
    pub title: Option<Vec<String>>,
    pub author: Option<Vec<CrossRefAuthor>>,
    pub published_print: Option<CrossRefDate>,
    pub published_online: Option<CrossRefDate>,
    pub abstract_text: Option<String>,
    pub doi: Option<String>,
    pub container_title: Option<Vec<String>>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CrossRefAuthor {
    pub given: Option<String>,
    pub family: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CrossRefDate {
    pub date_parts: Option<Vec<Vec<i32>>>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CrossRefResponse {
    pub status: String,
    pub message: CrossRefWork,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CrossRefSearchResponse {
    pub message: CrossRefSearchMessage,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CrossRefSearchMessage {
    pub items: Vec<CrossRefWork>,
}

pub struct CrossRefClient {
    client: reqwest::Client,
    base_url: String,
}

impl CrossRefClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .user_agent("Guanfu/0.1.0 (mailto:research@guanfu.app)")
                .build()
                .expect("failed to build HTTP client"),
            base_url: "https://api.crossref.org".to_string(),
        }
    }

    pub async fn get_by_doi(&self, doi: &str) -> AppResult<CrossRefWork> {
        let url = format!("{}/works/{}", self.base_url, doi);
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| crate::errors::AppError::NetworkError(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(crate::errors::AppError::MetadataError(format!(
                "CrossRef returned status {} for DOI {}",
                resp.status(),
                doi
            )));
        }

        let data: CrossRefResponse = resp
            .json()
            .await
            .map_err(|e| crate::errors::AppError::NetworkError(e.to_string()))?;

        Ok(data.message)
    }

    pub async fn search(&self, query: &str, limit: usize) -> AppResult<Vec<CrossRefWork>> {
        let url = format!(
            "{}/works?query={}&rows={}",
            self.base_url,
            urlencoding::encode(query),
            limit
        );
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| crate::errors::AppError::NetworkError(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(crate::errors::AppError::MetadataError(format!(
                "CrossRef search returned status {}",
                resp.status()
            )));
        }

        let data: CrossRefSearchResponse = resp
            .json()
            .await
            .map_err(|e| crate::errors::AppError::NetworkError(e.to_string()))?;

        Ok(data.message.items)
    }
}

pub fn extract_doi_from_text(text: &str) -> Option<String> {
    // Match DOI patterns like 10.xxxx/xxxxx
    let re = regex::Regex::new(r"(10\.\d{4,}/[^\s]+)").ok()?;
    re.captures(text).map(|cap| cap[1].to_string())
}

pub fn crossref_work_to_metadata(work: &CrossRefWork) -> (String, Vec<String>, Option<i32>, Option<String>) {
    let title = work
        .title
        .as_ref()
        .and_then(|t| t.first())
        .cloned()
        .unwrap_or_default();

    let authors: Vec<String> = work
        .author
        .as_ref()
        .map(|authors| {
            authors
                .iter()
                .map(|a| {
                    match (&a.family, &a.given) {
                        (Some(f), Some(g)) => format!("{} {}", f, g),
                        (Some(f), None) => f.clone(),
                        (None, Some(g)) => g.clone(),
                        (None, None) => String::new(),
                    }
                })
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default();

    let year = work
        .published_print
        .as_ref()
        .or(work.published_online.as_ref())
        .and_then(|d| d.date_parts.as_ref())
        .and_then(|dp| dp.first())
        .and_then(|parts| parts.first())
        .copied();

    let abstract_text = work.abstract_text.clone();

    (title, authors, year, abstract_text)
}
