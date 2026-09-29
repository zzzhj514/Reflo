use rusqlite::{params, OptionalExtension};
use std::path::Path;

use super::open_connection;

#[derive(Clone)]
pub struct MarkdownRecord {
    pub document_id: String,
    pub relative_path: String,
    pub processor: String,
    pub model: String,
    pub updated_at: String,
}

#[derive(Clone)]
pub struct MinerUPreferencesRecord {
    pub model: String,
    pub language: String,
    pub enable_ocr: bool,
    pub enable_formula: bool,
    pub enable_table: bool,
}

pub fn mineru_preferences(db: &Path) -> rusqlite::Result<MinerUPreferencesRecord> {
    open_connection(db)?.query_row(
        "SELECT model,language,enable_ocr,enable_formula,enable_table FROM conversion_settings WHERE provider='mineru'",
        [],
        |row| Ok(MinerUPreferencesRecord {
            model: row.get(0)?,
            language: row.get(1)?,
            enable_ocr: row.get(2)?,
            enable_formula: row.get(3)?,
            enable_table: row.get(4)?,
        }),
    )
}

pub fn save_mineru_preferences(
    db: &Path,
    preferences: &MinerUPreferencesRecord,
) -> rusqlite::Result<()> {
    open_connection(db)?.execute(
        "UPDATE conversion_settings SET model=?1,language=?2,enable_ocr=?3,enable_formula=?4,enable_table=?5,updated_at=CURRENT_TIMESTAMP WHERE provider='mineru'",
        params![
            preferences.model,
            preferences.language,
            preferences.enable_ocr,
            preferences.enable_formula,
            preferences.enable_table,
        ],
    )?;
    Ok(())
}

pub fn save_markdown(
    db: &Path,
    document_id: &str,
    relative_path: &str,
    processor: &str,
    model: &str,
) -> rusqlite::Result<MarkdownRecord> {
    let connection = open_connection(db)?;
    connection.execute(
        "INSERT INTO markdown_documents(document_id,relative_path,processor,model) VALUES(?1,?2,?3,?4) ON CONFLICT(document_id) DO UPDATE SET relative_path=excluded.relative_path, processor=excluded.processor, model=excluded.model, updated_at=CURRENT_TIMESTAMP",
        params![document_id, relative_path, processor, model],
    )?;
    connection.execute(
        "DELETE FROM markdown_translations WHERE document_id=?1",
        [document_id],
    )?;
    connection.execute("DELETE FROM rag_chunks WHERE document_id=?1", [document_id])?;
    find_markdown(db, document_id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)
}

pub fn find_markdown(db: &Path, document_id: &str) -> rusqlite::Result<Option<MarkdownRecord>> {
    open_connection(db)?.query_row(
        "SELECT document_id,relative_path,processor,model,updated_at FROM markdown_documents WHERE document_id=?1",
        [document_id],
        |row| Ok(MarkdownRecord {
            document_id: row.get(0)?,
            relative_path: row.get(1)?,
            processor: row.get(2)?,
            model: row.get(3)?,
            updated_at: row.get(4)?,
        }),
    ).optional()
}
