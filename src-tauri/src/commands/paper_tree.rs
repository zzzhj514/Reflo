use tauri::Manager;

use crate::{
    application::paper_tree::{self, PaperTreeGeneration},
    domain::paper_tree::{PaperTree, SavePaperTree},
};

#[tauri::command]
pub async fn get_paper_tree(
    app: tauri::AppHandle,
    paper_id: String,
    paper_title: String,
) -> Result<PaperTree, String> {
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        paper_tree::get(&data_dir, &paper_id, &paper_title)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn generate_paper_tree(
    app: tauri::AppHandle,
    paper_id: String,
) -> Result<PaperTreeGeneration, String> {
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || paper_tree::generate(&data_dir, &paper_id))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn save_paper_tree(
    app: tauri::AppHandle,
    input: SavePaperTree,
) -> Result<PaperTree, String> {
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || paper_tree::save(&data_dir, input))
        .await
        .map_err(|e| e.to_string())?
}
