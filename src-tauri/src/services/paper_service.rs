use std::fs;
use std::path::Path;

use crate::errors::{AppError, AppResult};
use crate::models::{Paper, Project};

pub fn import_pdfs(project: &mut Project, file_paths: Vec<String>) -> AppResult<Vec<Paper>> {
    let papers_dir = Path::new(&project.path).join("papers");
    fs::create_dir_all(&papers_dir)?;

    let mut new_papers = Vec::new();

    for file_path in file_paths {
        let source = Path::new(&file_path);
        if !source.exists() {
            return Err(AppError::FileNotFound(file_path));
        }

        let mut paper = Paper::new(
            source
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
            String::new(),
        );

        let dest_filename = format!("{}.pdf", paper.id);
        let dest_path = papers_dir.join(&dest_filename);

        fs::copy(source, &dest_path)?;

        paper.file_path = dest_filename;
        paper.title = source
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        project.papers.push(paper.clone());
        new_papers.push(paper);
    }

    Ok(new_papers)
}

pub fn update_paper(project: &mut Project, paper: Paper) -> AppResult<Paper> {
    let idx = project
        .papers
        .iter()
        .position(|p| p.id == paper.id)
        .ok_or_else(|| AppError::Unknown(format!("文献不存在: {}", paper.id)))?;

    project.papers[idx] = paper.clone();
    Ok(paper)
}

pub fn delete_paper(project: &mut Project, paper_id: &str) -> AppResult<()> {
    let idx = project
        .papers
        .iter()
        .position(|p| p.id == paper_id)
        .ok_or_else(|| AppError::Unknown(format!("文献不存在: {}", paper_id)))?;

    let paper = &project.papers[idx];
    let pdf_path = Path::new(&project.path).join("papers").join(&paper.file_path);
    if pdf_path.exists() {
        fs::remove_file(pdf_path)?;
    }

    project.papers.remove(idx);
    project.relations.retain(|r| r.source_id != paper_id && r.target_id != paper_id);

    Ok(())
}

pub fn get_pdf_file_url(project_path: &str, paper_id: &str, papers: &[Paper]) -> AppResult<String> {
    let paper = papers
        .iter()
        .find(|p| p.id == paper_id)
        .ok_or_else(|| AppError::FileNotFound(paper_id.to_string()))?;

    let pdf_path = Path::new(project_path).join("papers").join(&paper.file_path);
    if !pdf_path.exists() {
        return Err(AppError::FileNotFound(pdf_path.to_string_lossy().to_string()));
    }

    Ok(format!("file://{}", pdf_path.to_string_lossy()))
}
