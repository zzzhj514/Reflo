use crate::application::translation::{
    self, SaveTranslationSettings, TranslationResult, TranslationSettingsStatus,
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
