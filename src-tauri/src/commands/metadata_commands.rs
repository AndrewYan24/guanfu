use crate::errors::{AppError, AppResult};
use crate::models::Paper;
use crate::services::{crossref_client, project_service};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetadataCandidate {
    pub title: String,
    pub authors: Vec<String>,
    pub year: Option<i32>,
    pub abstract_text: Option<String>,
    pub doi: Option<String>,
    pub source: String,
    pub score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetadataResolveResult {
    pub candidates: Vec<MetadataCandidate>,
    pub applied: bool,
    pub paper: Option<Paper>,
}

#[tauri::command]
pub async fn resolve_metadata(
    project_path: String,
    paper_id: String,
) -> AppResult<MetadataResolveResult> {
    let mut project = project_service::open_project(&project_path)?;
    let paper = project
        .papers
        .iter()
        .find(|p| p.id == paper_id)
        .ok_or_else(|| AppError::FileNotFound(paper_id.clone()))?;

    // Try to extract DOI from paper title (filename)
    let doi = crossref_client::extract_doi_from_text(&paper.title);

    let client = crossref_client::CrossRefClient::new();

    if let Some(doi_str) = doi {
        // Found a DOI - try direct lookup
        match client.get_by_doi(&doi_str).await {
            Ok(work) => {
                let (title, authors, year, abstract_text) =
                    crossref_client::crossref_work_to_metadata(&work);

                // Auto-apply if DOI match is confident
                let paper_idx = project.papers.iter().position(|p| p.id == paper_id).unwrap();
                if !title.is_empty() {
                    project.papers[paper_idx].title = title.clone();
                }
                if !authors.is_empty() {
                    project.papers[paper_idx].authors = authors.clone();
                }
                if year.is_some() {
                    project.papers[paper_idx].year = year;
                }
                if abstract_text.is_some() {
                    project.papers[paper_idx].abstract_text = abstract_text.clone();
                }
                project.papers[paper_idx].updated_at = chrono::Utc::now().to_rfc3339();

                let updated_paper = project.papers[paper_idx].clone();
                project_service::save_project(&project)?;

                return Ok(MetadataResolveResult {
                    candidates: vec![MetadataCandidate {
                        title,
                        authors,
                        year,
                        abstract_text,
                        doi: Some(doi_str),
                        source: "crossref".to_string(),
                        score: 0.95,
                    }],
                    applied: true,
                    paper: Some(updated_paper),
                });
            }
            Err(_) => {
                // DOI lookup failed, fall through to search
            }
        }
    }

    // No DOI or DOI lookup failed - try searching by title
    let search_query = &paper.title;
    if search_query.is_empty() || search_query.len() < 5 {
        return Ok(MetadataResolveResult {
            candidates: vec![],
            applied: false,
            paper: None,
        });
    }

    match client.search(search_query, 5).await {
        Ok(works) => {
            let candidates: Vec<MetadataCandidate> = works
                .iter()
                .map(|work| {
                    let (title, authors, year, abstract_text) =
                        crossref_client::crossref_work_to_metadata(work);
                    MetadataCandidate {
                        title,
                        authors,
                        year,
                        abstract_text,
                        doi: work.doi.clone(),
                        source: "crossref".to_string(),
                        score: 0.5,
                    }
                })
                .filter(|c| !c.title.is_empty())
                .collect();

            Ok(MetadataResolveResult {
                candidates,
                applied: false,
                paper: None,
            })
        }
        Err(_) => Ok(MetadataResolveResult {
            candidates: vec![],
            applied: false,
            paper: None,
        }),
    }
}

#[tauri::command]
pub async fn search_metadata_candidates(
    project_path: String,
    paper_id: String,
) -> AppResult<Vec<MetadataCandidate>> {
    let project = project_service::open_project(&project_path)?;
    let paper = project
        .papers
        .iter()
        .find(|p| p.id == paper_id)
        .ok_or_else(|| AppError::FileNotFound(paper_id.clone()))?;

    let client = crossref_client::CrossRefClient::new();
    let query = &paper.title;

    if query.is_empty() || query.len() < 5 {
        return Ok(vec![]);
    }

    let works = client.search(query, 10).await?;
    let candidates: Vec<MetadataCandidate> = works
        .iter()
        .map(|work| {
            let (title, authors, year, abstract_text) =
                crossref_client::crossref_work_to_metadata(work);
            MetadataCandidate {
                title,
                authors,
                year,
                abstract_text,
                doi: work.doi.clone(),
                source: "crossref".to_string(),
                score: 0.5,
            }
        })
        .filter(|c| !c.title.is_empty())
        .collect();

    Ok(candidates)
}

#[tauri::command]
pub async fn apply_metadata_candidate(
    project_path: String,
    paper_id: String,
    candidate: MetadataCandidate,
) -> AppResult<Paper> {
    let mut project = project_service::open_project(&project_path)?;
    let paper_idx = project
        .papers
        .iter()
        .position(|p| p.id == paper_id)
        .ok_or_else(|| AppError::FileNotFound(paper_id.clone()))?;

    let paper = &mut project.papers[paper_idx];
    paper.title = if candidate.title.is_empty() {
        paper.title.clone()
    } else {
        candidate.title
    };
    if !candidate.authors.is_empty() {
        paper.authors = candidate.authors;
    }
    if candidate.year.is_some() {
        paper.year = candidate.year;
    }
    if candidate.abstract_text.is_some() {
        paper.abstract_text = candidate.abstract_text;
    }
    paper.updated_at = chrono::Utc::now().to_rfc3339();

    let updated_paper = paper.clone();
    project_service::save_project(&project)?;

    Ok(updated_paper)
}
