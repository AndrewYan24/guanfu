use crate::errors::AppResult;
use crate::models::Project;
use crate::services::project_service;

#[tauri::command]
pub async fn create_project(name: String, base_dir: String) -> AppResult<Project> {
    project_service::create_project(&name, &base_dir)
}

#[tauri::command]
pub async fn open_project(project_path: String) -> AppResult<Project> {
    project_service::open_project(&project_path)
}

#[tauri::command]
pub async fn save_project(project: Project) -> AppResult<()> {
    project_service::save_project(&project)
}

#[tauri::command]
pub async fn get_recent_project() -> AppResult<Option<String>> {
    project_service::get_recent_project()
}

#[tauri::command]
pub async fn set_recent_project(project_path: String) -> AppResult<()> {
    project_service::set_recent_project(&project_path)
}
