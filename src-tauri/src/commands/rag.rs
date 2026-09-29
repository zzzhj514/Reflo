use tauri::Manager;

use crate::application::rag::{
    self, RagAnswer, RagIndexStatus, RagMessage, RagSettingsStatus, SaveRagSettings,
};

#[tauri::command]
pub async fn get_rag_settings(app: tauri::AppHandle) -> Result<RagSettingsStatus, String> {
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || rag::get_settings(&data_dir))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn save_rag_settings(
    app: tauri::AppHandle,
    settings: SaveRagSettings,
) -> Result<RagSettingsStatus, String> {
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || rag::save_settings(&data_dir, settings))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_rag_index_status(
    app: tauri::AppHandle,
    paper_id: String,
) -> Result<RagIndexStatus, String> {
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || rag::index_status(&data_dir, &paper_id))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn index_paper_rag(
    app: tauri::AppHandle,
    paper_id: String,
) -> Result<RagIndexStatus, String> {
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || rag::index_paper(&data_dir, &paper_id))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn list_rag_messages(
    app: tauri::AppHandle,
    paper_id: String,
) -> Result<Vec<RagMessage>, String> {
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || rag::messages(&data_dir, &paper_id))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn ask_paper_rag(
    app: tauri::AppHandle,
    paper_id: String,
    question: String,
) -> Result<RagAnswer, String> {
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || rag::ask(&data_dir, &paper_id, &question))
        .await
        .map_err(|e| e.to_string())?
}
