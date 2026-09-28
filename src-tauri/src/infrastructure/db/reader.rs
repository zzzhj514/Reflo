use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::path::Path;

pub struct ReadingDocument {
    pub id: String,
    pub relative_path: String,
    pub page_index: u32,
    pub scale: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AnnotationRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Annotation {
    pub id: String,
    pub document_id: String,
    pub page_index: u32,
    pub kind: String,
    pub selected_text: String,
    pub note: Option<String>,
    pub rects: Vec<AnnotationRect>,
    pub color: String,
    pub created_at: String,
}

pub struct NewAnnotation<'a> {
    pub id: &'a str,
    pub document_id: &'a str,
    pub page_index: u32,
    pub kind: &'a str,
    pub selected_text: &'a str,
    pub note: Option<&'a str>,
    pub rects: &'a [AnnotationRect],
    pub color: &'a str,
}

pub fn document_for_paper(db: &Path, paper_id: &str) -> rusqlite::Result<Option<ReadingDocument>> {
    super::open_connection(db)?.query_row(
        "SELECT d.id, d.relative_path, COALESCE(r.page_index,0), COALESCE(r.scale,1) FROM documents d LEFT JOIN reading_positions r ON r.document_id=d.id WHERE d.paper_id=?1 ORDER BY d.created_at DESC, d.id LIMIT 1",
        params![paper_id], |row| Ok(ReadingDocument { id: row.get(0)?, relative_path: row.get(1)?, page_index: row.get(2)?, scale: row.get(3)? }),
    ).optional()
}

pub fn save_position(
    db: &Path,
    document_id: &str,
    page_index: u32,
    scale: f64,
) -> rusqlite::Result<()> {
    super::open_connection(db)?.execute(
        "INSERT INTO reading_positions(document_id,page_index,scale) VALUES(?1,?2,?3) ON CONFLICT(document_id) DO UPDATE SET page_index=excluded.page_index, scale=excluded.scale, updated_at=CURRENT_TIMESTAMP",
        params![document_id, page_index, scale],
    )?;
    Ok(())
}

pub fn list_annotations(db: &Path, document_id: &str) -> rusqlite::Result<Vec<Annotation>> {
    let connection = super::open_connection(db)?;
    let mut statement = connection.prepare(
        "SELECT id,document_id,page_index,kind,selected_text,note,rects_json,color,created_at FROM annotations WHERE document_id=?1 ORDER BY page_index,created_at,id",
    )?;
    let rows = statement.query_map([document_id], |row| {
        let rects_json: String = row.get(6)?;
        let rects = serde_json::from_str(&rects_json).map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(
                6,
                rusqlite::types::Type::Text,
                Box::new(error),
            )
        })?;
        Ok(Annotation {
            id: row.get(0)?,
            document_id: row.get(1)?,
            page_index: row.get(2)?,
            kind: row.get(3)?,
            selected_text: row.get(4)?,
            note: row.get(5)?,
            rects,
            color: row.get(7)?,
            created_at: row.get(8)?,
        })
    })?;
    rows.collect()
}

pub fn create_annotation(db: &Path, annotation: NewAnnotation<'_>) -> rusqlite::Result<Annotation> {
    let connection = super::open_connection(db)?;
    let rects_json = serde_json::to_string(annotation.rects)
        .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
    connection.execute(
        "INSERT INTO annotations(id,document_id,page_index,kind,selected_text,note,rects_json,color) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",
        params![annotation.id, annotation.document_id, annotation.page_index, annotation.kind, annotation.selected_text, annotation.note, rects_json, annotation.color],
    )?;
    list_annotations(db, annotation.document_id)?
        .into_iter()
        .find(|item| item.id == annotation.id)
        .ok_or(rusqlite::Error::QueryReturnedNoRows)
}

pub fn delete_annotation(db: &Path, id: &str) -> rusqlite::Result<bool> {
    Ok(super::open_connection(db)?.execute("DELETE FROM annotations WHERE id=?1", [id])? == 1)
}
