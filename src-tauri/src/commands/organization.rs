use serde::Serialize;
use tauri::Manager;

use crate::application::organization;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaperGroupDto {
    id: String,
    name: String,
    paper_count: usize,
}

impl From<organization::PaperGroupRecord> for PaperGroupDto {
    fn from(value: organization::PaperGroupRecord) -> Self {
        Self {
            id: value.id,
            name: value.name,
            paper_count: value.paper_count,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TodoDto {
    id: String,
    title: String,
    completed: bool,
    created_at: String,
}

impl From<organization::TodoRecord> for TodoDto {
    fn from(value: organization::TodoRecord) -> Self {
        Self {
            id: value.id,
            title: value.title,
            completed: value.completed,
            created_at: value.created_at,
        }
    }
}

fn data_dir(app: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
    app.path().app_data_dir().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_paper_groups(app: tauri::AppHandle) -> Result<Vec<PaperGroupDto>, String> {
    let dir = data_dir(&app)?;
    tauri::async_runtime::spawn_blocking(move || {
        organization::list_groups(&dir).map(|items| items.into_iter().map(Into::into).collect())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn create_paper_group(
    app: tauri::AppHandle,
    name: String,
) -> Result<PaperGroupDto, String> {
    let dir = data_dir(&app)?;
    tauri::async_runtime::spawn_blocking(move || {
        organization::create_group(&dir, name).map(Into::into)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn rename_paper_group(
    app: tauri::AppHandle,
    id: String,
    name: String,
) -> Result<(), String> {
    let dir = data_dir(&app)?;
    tauri::async_runtime::spawn_blocking(move || organization::rename_group(&dir, &id, name))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn delete_paper_group(app: tauri::AppHandle, id: String) -> Result<(), String> {
    let dir = data_dir(&app)?;
    tauri::async_runtime::spawn_blocking(move || organization::delete_group(&dir, &id))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn assign_paper_group(
    app: tauri::AppHandle,
    paper_id: String,
    group_id: Option<String>,
) -> Result<(), String> {
    let dir = data_dir(&app)?;
    tauri::async_runtime::spawn_blocking(move || {
        organization::assign_paper(&dir, &paper_id, group_id)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn list_todos(app: tauri::AppHandle) -> Result<Vec<TodoDto>, String> {
    let dir = data_dir(&app)?;
    tauri::async_runtime::spawn_blocking(move || {
        organization::list_todos(&dir).map(|items| items.into_iter().map(Into::into).collect())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn create_todo(app: tauri::AppHandle, title: String) -> Result<TodoDto, String> {
    let dir = data_dir(&app)?;
    tauri::async_runtime::spawn_blocking(move || {
        organization::create_todo(&dir, title).map(Into::into)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn set_todo_completed(
    app: tauri::AppHandle,
    id: String,
    completed: bool,
) -> Result<(), String> {
    let dir = data_dir(&app)?;
    tauri::async_runtime::spawn_blocking(move || {
        organization::set_todo_completed(&dir, &id, completed)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn delete_todo(app: tauri::AppHandle, id: String) -> Result<(), String> {
    let dir = data_dir(&app)?;
    tauri::async_runtime::spawn_blocking(move || organization::delete_todo(&dir, &id))
        .await
        .map_err(|e| e.to_string())?
}
