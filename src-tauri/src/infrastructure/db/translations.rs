use rusqlite::params;
use std::path::Path;

use super::open_connection;

#[derive(Clone)]
pub struct MarkdownTranslationRecord {
    pub document_id: String,
    pub relative_path: String,
    pub provider: String,
    pub model: String,
    pub target_language: String,
    pub updated_at: String,
}

pub fn find_markdown_translation(
    db: &Path,
    document_id: &str,
) -> rusqlite::Result<Option<MarkdownTranslationRecord>> {
    use rusqlite::OptionalExtension;
    open_connection(db)?.query_row(
        "SELECT document_id,relative_path,provider,model,target_language,updated_at FROM markdown_translations WHERE document_id=?1",
        [document_id],
        |row| Ok(MarkdownTranslationRecord {
            document_id: row.get(0)?,
            relative_path: row.get(1)?,
            provider: row.get(2)?,
            model: row.get(3)?,
            target_language: row.get(4)?,
            updated_at: row.get(5)?,
        }),
    ).optional()
}

pub fn save_markdown_translation(
    db: &Path,
    record: &MarkdownTranslationRecord,
) -> rusqlite::Result<MarkdownTranslationRecord> {
    open_connection(db)?.execute(
        "INSERT INTO markdown_translations(document_id,relative_path,provider,model,target_language) VALUES(?1,?2,?3,?4,?5) ON CONFLICT(document_id) DO UPDATE SET relative_path=excluded.relative_path,provider=excluded.provider,model=excluded.model,target_language=excluded.target_language,updated_at=CURRENT_TIMESTAMP",
        params![record.document_id, record.relative_path, record.provider, record.model, record.target_language],
    )?;
    find_markdown_translation(db, &record.document_id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)
}

#[derive(Clone)]
pub struct TranslationSettingsRecord {
    pub provider: String,
    pub base_url: String,
    pub model: String,
    pub target_language: String,
}

pub fn settings(db: &Path) -> rusqlite::Result<TranslationSettingsRecord> {
    open_connection(db)?.query_row(
        "SELECT provider,base_url,model,target_language FROM translation_settings WHERE id=1",
        [],
        |row| {
            Ok(TranslationSettingsRecord {
                provider: row.get(0)?,
                base_url: row.get(1)?,
                model: row.get(2)?,
                target_language: row.get(3)?,
            })
        },
    )
}

pub fn save_settings(db: &Path, settings: &TranslationSettingsRecord) -> rusqlite::Result<()> {
    open_connection(db)?.execute(
        "UPDATE translation_settings SET provider=?1,base_url=?2,model=?3,target_language=?4,updated_at=CURRENT_TIMESTAMP WHERE id=1",
        params![settings.provider, settings.base_url, settings.model, settings.target_language],
    )?;
    Ok(())
}
