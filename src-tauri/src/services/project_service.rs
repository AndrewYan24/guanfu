use std::fs;
use std::path::{Path, PathBuf};

use crate::errors::{AppError, AppResult};
use crate::models::Project;

const PROJECT_FILE: &str = "project.json";
const PAPERS_DIR: &str = "papers";
const RECENT_FILE: &str = "recent_project.json";

pub fn create_project(name: &str, base_dir: &str) -> AppResult<Project> {
    let project_dir_name = format!("{}.guanfu", name);
    let project_dir = Path::new(base_dir).join(&project_dir_name);

    if project_dir.exists() {
        return Err(AppError::Unknown(format!(
            "项目目录已存在: {}",
            project_dir.display()
        )));
    }

    fs::create_dir_all(project_dir.join(PAPERS_DIR))?;

    let project = Project::new(name.to_string(), project_dir.to_string_lossy().to_string());
    let json = serde_json::to_string_pretty(&project)?;
    fs::write(project_dir.join(PROJECT_FILE), json)?;

    Ok(project)
}

pub fn open_project(project_path: &str) -> AppResult<Project> {
    let project_dir = Path::new(project_path);
    let project_file = project_dir.join(PROJECT_FILE);

    if !project_file.exists() {
        return Err(AppError::ProjectNotFound(project_path.to_string()));
    }

    let json = fs::read_to_string(project_file)?;
    let project: Project = serde_json::from_str(&json)?;

    Ok(project)
}

pub fn save_project(project: &Project) -> AppResult<()> {
    let project_dir = Path::new(&project.path);
    let project_file = project_dir.join(PROJECT_FILE);

    let json = serde_json::to_string_pretty(project)?;
    fs::write(project_file, json)?;

    Ok(())
}

fn recent_project_path() -> AppResult<PathBuf> {
    let config_dir = dirs_next::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("guanfu");
    fs::create_dir_all(&config_dir)?;
    Ok(config_dir.join(RECENT_FILE))
}

pub fn get_recent_project() -> AppResult<Option<String>> {
    let path = recent_project_path()?;
    if !path.exists() {
        return Ok(None);
    }
    let json = fs::read_to_string(path)?;
    let recent: Option<String> = serde_json::from_str(&json)?;
    Ok(recent)
}

pub fn set_recent_project(project_path: &str) -> AppResult<()> {
    let path = recent_project_path()?;
    let json = serde_json::to_string(&Some(project_path.to_string()))?;
    fs::write(path, json)?;
    Ok(())
}
