use rusqlite::params;
use std::path::Path;

use super::open_connection;

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
