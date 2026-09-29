use std::path::Path;

use rusqlite::{params, OptionalExtension};

use super::open_connection;

pub fn get(database: &Path, name: &str) -> rusqlite::Result<Option<String>> {
    open_connection(database)?
        .query_row(
            "SELECT secret FROM api_credentials WHERE name = ?1",
            [name],
            |row| row.get(0),
        )
        .optional()
}

pub fn set(database: &Path, name: &str, secret: &str) -> rusqlite::Result<()> {
    open_connection(database)?.execute(
        "INSERT INTO api_credentials(name, secret, updated_at)
         VALUES (?1, ?2, CURRENT_TIMESTAMP)
         ON CONFLICT(name) DO UPDATE SET
           secret = excluded.secret,
           updated_at = CURRENT_TIMESTAMP",
        params![name, secret],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::db;

    #[test]
    fn stores_and_replaces_a_local_api_credential() {
        let root = std::env::temp_dir().join(format!("reflo-credentials-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&root).unwrap();
        let database = root.join("reflo.sqlite");
        db::initialize(&database).unwrap();

        assert_eq!(get(&database, "provider").unwrap(), None);
        set(&database, "provider", "first").unwrap();
        set(&database, "provider", "second").unwrap();
        assert_eq!(
            get(&database, "provider").unwrap().as_deref(),
            Some("second")
        );

        let _ = std::fs::remove_dir_all(root);
    }
}
