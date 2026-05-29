use std::collections::HashSet;
use std::fs;
use std::path::Path;

use crate::errors::{AppError, AppResult};
use crate::models::{Paper, Project};

pub async fn import_pdfs(project: &mut Project, file_paths: Vec<String>) -> AppResult<Vec<Paper>> {
    let papers_dir = Path::new(&project.path).join("papers");
    fs::create_dir_all(&papers_dir)?;

    // Build set of existing paper IDs for duplicate detection
    let existing_ids: HashSet<&str> =
        project.papers.iter().map(|p| p.id.as_str()).collect();

    // Validate and filter files
    let mut work = Vec::new();
    for file_path in file_paths {
        let source = Path::new(&file_path);
        if !source.exists() {
            return Err(AppError::FileNotFound(file_path));
        }

        // Check for duplicate by file stem (same as paper ID generation)
        let stem = source
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        if existing_ids.contains(stem.as_str()) {
            eprintln!("[import] skipping duplicate: {}", stem);
            continue;
        }

        work.push((file_path, stem));
    }

    if work.is_empty() {
        return Ok(vec![]);
    }

    // Copy files concurrently
    let mut handles = Vec::new();
    for (file_path, stem) in work {
        let papers_dir = papers_dir.clone();
        handles.push(tokio::spawn(async move {
            let source = Path::new(&file_path);
            let paper = Paper::new(stem, String::new());
            let dest_filename = format!("{}.pdf", paper.id);
            let dest_path = papers_dir.join(&dest_filename);
            tokio::fs::copy(source, &dest_path).await?;
            Ok::<(Paper, String), AppError>((paper, dest_filename))
        }));
    }

    let mut new_papers = Vec::new();
    for h in handles {
        let (mut paper, dest_filename) = h.await.map_err(|e| {
            AppError::Unknown(format!("文件复制任务失败: {}", e))
        })??;
        paper.file_path = dest_filename;
        paper.title = std::path::Path::new(&paper.file_path)
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        // Strip the .pdf suffix from title
        if paper.title.ends_with(".pdf") {
            paper.title.truncate(paper.title.len() - 4);
        }
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
