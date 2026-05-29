use crate::errors::{AppError, AppResult};
use crate::models::Paper;
use crate::services::{paper_service, project_service};

#[tauri::command]
pub async fn import_pdfs(project_path: String, file_paths: Vec<String>) -> AppResult<Vec<Paper>> {
    let mut project = project_service::open_project(&project_path)?;
    let papers = paper_service::import_pdfs(&mut project, file_paths).await?;
    project_service::save_project(&project)?;
    Ok(papers)
}

#[tauri::command]
pub async fn update_paper(project_path: String, paper: Paper) -> AppResult<Paper> {
    let mut project = project_service::open_project(&project_path)?;
    let paper = paper_service::update_paper(&mut project, paper)?;
    project_service::save_project(&project)?;
    Ok(paper)
}

#[tauri::command]
pub async fn delete_paper(project_path: String, paper_id: String) -> AppResult<()> {
    let mut project = project_service::open_project(&project_path)?;
    paper_service::delete_paper(&mut project, &paper_id)?;
    project_service::save_project(&project)?;
    Ok(())
}

#[tauri::command]
pub async fn extract_pdf_text(project_path: String, paper_id: String) -> AppResult<String> {
    let project = project_service::open_project(&project_path)?;
    let paper = project
        .papers
        .iter()
        .find(|p| p.id == paper_id)
        .ok_or_else(|| AppError::FileNotFound(paper_id.clone()))?;

    let pdf_path = std::path::Path::new(&project_path)
        .join("papers")
        .join(&paper.file_path);

    crate::services::pdf_text_extractor::extract_text_with_ocr_fallback(&pdf_path).await
}

#[tauri::command]
pub async fn get_pdf_file_url(project_path: String, paper_id: String) -> AppResult<String> {
    let project = project_service::open_project(&project_path)?;
    paper_service::get_pdf_file_url(&project_path, &paper_id, &project.papers)
}

#[tauri::command]
pub async fn read_pdf_file(project_path: String, paper_id: String) -> AppResult<Vec<u8>> {
    let project = project_service::open_project(&project_path)?;
    let paper = project
        .papers
        .iter()
        .find(|p| p.id == paper_id)
        .ok_or_else(|| AppError::FileNotFound(paper_id.clone()))?;

    let pdf_path = std::path::Path::new(&project_path)
        .join("papers")
        .join(&paper.file_path);

    std::fs::read(&pdf_path).map_err(|e| {
        AppError::IoError(format!("无法读取 PDF 文件 {}: {}", pdf_path.display(), e))
    })
}
