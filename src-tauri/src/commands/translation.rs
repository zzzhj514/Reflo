use crate::application::translation::{
    self, MarkdownTranslationDocument, SaveTranslationSettings, TranslationResult,
    TranslationSettingsStatus,
};
use tauri::Manager;

#[tauri::command]
pub async fn get_translation_settings(
    app: tauri::AppHandle,
) -> Result<TranslationSettingsStatus, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?;
    tauri::async_runtime::spawn_blocking(move || translation::get_settings(&data_dir))
        .await
        .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn save_translation_settings(
    app: tauri::AppHandle,
    settings: SaveTranslationSettings,
) -> Result<TranslationSettingsStatus, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?;
    tauri::async_runtime::spawn_blocking(move || translation::save_settings(&data_dir, settings))
        .await
        .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn translate_text(
    app: tauri::AppHandle,
    text: String,
) -> Result<TranslationResult, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?;
    tauri::async_runtime::spawn_blocking(move || translation::translate(&data_dir, &text))
        .await
        .map_err(|error| error.to_string())?
}

fn allow_markdown_assets(
    app: &tauri::AppHandle,
    document: &MarkdownTranslationDocument,
) -> Result<(), String> {
    app.asset_protocol_scope()
        .allow_directory(&document.asset_base_path, true)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn get_markdown_translation(
    app: tauri::AppHandle,
    paper_id: String,
) -> Result<Option<MarkdownTranslationDocument>, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?;
    let document = tauri::async_runtime::spawn_blocking(move || {
        translation::get_markdown_translation(&data_dir, &paper_id)
    })
    .await
    .map_err(|error| error.to_string())??;
    if let Some(document) = &document {
        allow_markdown_assets(&app, document)?;
    }
    Ok(document)
}

#[tauri::command]
pub async fn translate_markdown_document(
    app: tauri::AppHandle,
    paper_id: String,
) -> Result<MarkdownTranslationDocument, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?;
    let document = tauri::async_runtime::spawn_blocking(move || {
        translation::translate_markdown(&data_dir, &paper_id)
    })
    .await
    .map_err(|error| error.to_string())??;
    allow_markdown_assets(&app, &document)?;
    Ok(document)
}
