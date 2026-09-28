use crate::application::reader;
use serde::{Deserialize, Serialize};
use tauri::Manager;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentDto {
    document_id: String,
    path: String,
    page_index: u32,
    scale: f64,
}

#[tauri::command]
pub async fn open_document(app: tauri::AppHandle, paper_id: String) -> Result<DocumentDto, String> {
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let opened =
        tauri::async_runtime::spawn_blocking(move || reader::open_document(&data_dir, &paper_id))
            .await
            .map_err(|e| e.to_string())??;
    app.asset_protocol_scope()
        .allow_file(&opened.path)
        .map_err(|e| e.to_string())?;
    Ok(DocumentDto {
        document_id: opened.document.id,
        path: opened.path.to_str().ok_or("文件路径无法编码")?.to_owned(),
        page_index: opened.document.page_index,
        scale: opened.document.scale,
    })
}

#[tauri::command]
pub async fn save_reading_position(
    app: tauri::AppHandle,
    document_id: String,
    page_index: u32,
    scale: f64,
) -> Result<(), String> {
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        reader::save_position(&data_dir, &document_id, page_index, scale)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnnotationInputDto {
    document_id: String,
    page_index: u32,
    kind: String,
    selected_text: String,
    note: Option<String>,
    rects: Vec<crate::infrastructure::db::reader::AnnotationRect>,
    color: String,
}

#[tauri::command]
pub async fn list_annotations(
    app: tauri::AppHandle,
    document_id: String,
) -> Result<Vec<crate::infrastructure::db::reader::Annotation>, String> {
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || reader::list_annotations(&data_dir, &document_id))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn create_annotation(
    app: tauri::AppHandle,
    input: AnnotationInputDto,
) -> Result<crate::infrastructure::db::reader::Annotation, String> {
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        reader::create_annotation(
            &data_dir,
            reader::AnnotationInput {
                document_id: input.document_id,
                page_index: input.page_index,
                kind: input.kind,
                selected_text: input.selected_text,
                note: input.note,
                rects: input.rects,
                color: input.color,
            },
        )
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn delete_annotation(app: tauri::AppHandle, id: String) -> Result<(), String> {
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || reader::delete_annotation(&data_dir, &id))
        .await
        .map_err(|e| e.to_string())?
}
