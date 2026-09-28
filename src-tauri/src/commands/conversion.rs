use crate::application::conversion::{
    self, MarkdownDocument, MinerUSettingsStatus, SaveMinerUSettings,
};
use tauri::Manager;

fn allow_markdown_assets(
    app: &tauri::AppHandle,
    document: &MarkdownDocument,
) -> Result<(), String> {
    app.asset_protocol_scope()
        .allow_directory(&document.asset_base_path, true)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn get_mineru_settings(app: tauri::AppHandle) -> Result<MinerUSettingsStatus, String> {
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || conversion::get_settings(&data_dir))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn save_mineru_settings(
    app: tauri::AppHandle,
    settings: SaveMinerUSettings,
) -> Result<MinerUSettingsStatus, String> {
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || conversion::save_settings(&data_dir, settings))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_markdown_document(
    app: tauri::AppHandle,
    paper_id: String,
) -> Result<Option<MarkdownDocument>, String> {
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let document = tauri::async_runtime::spawn_blocking(move || {
        conversion::get_markdown(&data_dir, &paper_id)
    })
    .await
    .map_err(|e| e.to_string())??;
    if let Some(document) = &document {
        allow_markdown_assets(&app, document)?;
    }
    Ok(document)
}

#[tauri::command]
pub async fn convert_pdf_to_markdown(
    app: tauri::AppHandle,
    paper_id: String,
) -> Result<MarkdownDocument, String> {
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let document =
        tauri::async_runtime::spawn_blocking(move || conversion::convert(&data_dir, &paper_id))
            .await
            .map_err(|e| e.to_string())??;
    allow_markdown_assets(&app, &document)?;
    Ok(document)
}
