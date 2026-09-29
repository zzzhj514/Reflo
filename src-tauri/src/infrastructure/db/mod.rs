pub mod conversions;
pub mod papers;
pub mod reader;
pub mod translations;
use rusqlite::Connection;
use std::path::Path;
use std::time::Duration;

pub(super) fn open_connection(path: &Path) -> rusqlite::Result<Connection> {
    let connection = Connection::open(path)?;

    connection.busy_timeout(Duration::from_secs(5))?;
    connection.execute_batch("PRAGMA foreign_keys = ON;")?;

    Ok(connection)
}

pub fn initialize(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut connection = open_connection(path)?;

    let transaction = connection.transaction()?;

    let version: i64 = transaction.query_row("PRAGMA user_version;", [], |row| row.get(0))?;

    if !(0..=8).contains(&version) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("不支持的数据库版本：{version}"),
        )
        .into());
    }
    if version == 0 {
        transaction.execute_batch(include_str!("../../../migrations/001_initial.sql"))?;
    }
    if version < 2 {
        transaction.execute_batch(include_str!(
            "../../../migrations/002_metadata_revision.sql"
        ))?;
        transaction.execute_batch("PRAGMA user_version = 2;")?;
    }

    if version < 3 {
        transaction.execute_batch(include_str!(
            "../../../migrations/003_reading_positions.sql"
        ))?;
        transaction.execute_batch("PRAGMA user_version = 3;")?;
    }
    if version < 4 {
        transaction.execute_batch(include_str!("../../../migrations/004_annotations.sql"))?;
        transaction.execute_batch("PRAGMA user_version = 4;")?;
    }
    if version < 5 {
        transaction.execute_batch(include_str!(
            "../../../migrations/005_markdown_documents.sql"
        ))?;
        transaction.execute_batch("PRAGMA user_version = 5;")?;
    }
    if version < 6 {
        transaction.execute_batch(include_str!(
            "../../../migrations/006_conversion_settings.sql"
        ))?;
        transaction.execute_batch("PRAGMA user_version = 6;")?;
    }
    if version < 7 {
        transaction.execute_batch(include_str!(
            "../../../migrations/007_translation_settings.sql"
        ))?;
        transaction.execute_batch("PRAGMA user_version = 7;")?;
    }
    if version < 8 {
        transaction.execute_batch(include_str!(
            "../../../migrations/008_markdown_translations.sql"
        ))?;
        transaction.execute_batch("PRAGMA user_version = 8;")?;
    }
    transaction.commit()?;

    Ok(())
}
