use std::fs;
use std::io::Write;
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

    protect_project_dir(&project_dir);

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

/// Protect the .guanfu project directory so users don't casually browse or
/// modify its internals. Best-effort; errors are silently ignored.
fn protect_project_dir(dir: &Path) {
    // 1. .hidden file — macOS Finder hides entries listed here.
    //    Harmless on other platforms.
    let hidden_path = dir.join(".hidden");
    if let Ok(mut f) = fs::File::create(&hidden_path) {
        let _ = writeln!(f, "papers");
        let _ = writeln!(f, "project.json");
    }

    // 2. Warning file — visible if someone opens the folder directly.
    let readme_path = dir.join("DO NOT EDIT - 观复项目文件");
    if let Ok(mut f) = fs::File::create(&readme_path) {
        let _ = write!(
            f,
            "此目录由「观复」应用自动管理，请勿手动修改或删除其中的文件。\n\
             This directory is managed by the Guanfu app. Do not modify or delete files.\n\n\
             修改内部文件可能导致项目损坏。\n\
             Modifying files may corrupt your project."
        );
    }

    // 3. On macOS, try to hide internal directories via chflags.
    //    This makes Finder treat them as hidden even without .hidden file.
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("chflags")
            .arg("hidden")
            .arg(dir.join(PAPERS_DIR))
            .status();
    }
}
