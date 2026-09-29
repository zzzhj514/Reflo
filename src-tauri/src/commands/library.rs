use serde::{Deserialize, Serialize};
use tauri::Manager;

use crate::{application::library, domain::paper::Paper};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaperDto {
    revision: i64,
    id: String,
    title: String,
    authors: Vec<String>,
    year: Option<i32>,
    doi: Option<String>,
    source_url: Option<String>,
    created_at: String,
    original_name: String,
    has_markdown: bool,
    has_translation: bool,
}

impl PaperDto {
    fn new(paper: Paper, original_name: String, has_markdown: bool, has_translation: bool) -> Self {
        Self {
            revision: paper.revision,
            id: paper.id,
            title: paper.title,
            authors: paper.authors,
            year: paper.year,
            doi: paper.doi,
            source_url: paper.source_url,
            created_at: paper.created_at,
            original_name,
            has_markdown,
            has_translation,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportDto {
    paper_id: String,
    duplicate: bool,
}

#[tauri::command]
pub async fn import_pdf(app: tauri::AppHandle, source_path: String) -> Result<ImportDto, String> {
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        library::import_pdf(std::path::Path::new(&source_path), &data_dir).map(|result| ImportDto {
            paper_id: result.paper_id,
            duplicate: result.duplicate,
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn list_papers(app: tauri::AppHandle) -> Result<Vec<PaperDto>, String> {
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        library::list_papers(&data_dir)?
            .into_iter()
            .map(|paper| {
                let artifacts = library::artifact_status(&data_dir, &paper.id)?;
                Ok(PaperDto::new(
                    paper,
                    artifacts.original_name,
                    artifacts.has_markdown,
                    artifacts.has_translation,
                ))
            })
            .collect()
    })
    .await
    .map_err(|e| e.to_string())?
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetadataInput {
    id: String,
    expected_revision: i64,
    title: String,
    authors: Vec<String>,
    year: Option<i32>,
    doi: Option<String>,
    source_url: Option<String>,
}

#[tauri::command]
pub async fn update_metadata(
    app: tauri::AppHandle,
    input: MetadataInput,
) -> Result<PaperDto, String> {
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        let paper = library::update_metadata(
            &data_dir,
            crate::domain::paper::MetadataUpdate {
                id: input.id,
                expected_revision: input.expected_revision,
                title: input.title,
                authors: input.authors,
                year: input.year,
                doi: input.doi,
                source_url: input.source_url,
            },
        )?;
        let artifacts = library::artifact_status(&data_dir, &paper.id)?;
        Ok(PaperDto::new(
            paper,
            artifacts.original_name,
            artifacts.has_markdown,
            artifacts.has_translation,
        ))
    })
    .await
    .map_err(|e| e.to_string())?
}
