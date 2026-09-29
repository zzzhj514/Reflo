use std::path::Path;

use rusqlite::{params, OptionalExtension};

use super::open_connection;

#[derive(Clone)]
pub struct RagSettingsRecord {
    pub provider: String,
    pub base_url: String,
    pub model: String,
    pub chunk_chars: usize,
    pub top_k: usize,
}

#[derive(Clone)]
pub struct RagChunkRecord {
    pub id: String,
    pub paper_id: String,
    pub document_id: String,
    pub heading_path: String,
    pub content: String,
    pub chunk_index: usize,
    pub content_hash: String,
    pub vector: Vec<f32>,
}

#[derive(Clone)]
pub struct RagMessageRecord {
    pub id: String,
    pub role: String,
    pub content: String,
    pub citations_json: String,
    pub created_at: String,
}

pub fn settings(database: &Path) -> rusqlite::Result<RagSettingsRecord> {
    open_connection(database)?.query_row(
        "SELECT provider, base_url, model, chunk_chars, top_k FROM rag_settings WHERE id = 1",
        [],
        |row| {
            Ok(RagSettingsRecord {
                provider: row.get(0)?,
                base_url: row.get(1)?,
                model: row.get(2)?,
                chunk_chars: row.get::<_, i64>(3)? as usize,
                top_k: row.get::<_, i64>(4)? as usize,
            })
        },
    )
}

pub fn save_settings(database: &Path, value: &RagSettingsRecord) -> rusqlite::Result<()> {
    open_connection(database)?.execute(
        "UPDATE rag_settings SET provider=?1, base_url=?2, model=?3, chunk_chars=?4,
         top_k=?5, updated_at=CURRENT_TIMESTAMP WHERE id=1",
        params![
            value.provider,
            value.base_url,
            value.model,
            value.chunk_chars as i64,
            value.top_k as i64
        ],
    )?;
    Ok(())
}

pub fn replace_index(
    database: &Path,
    paper_id: &str,
    chunks: &[RagChunkRecord],
    provider: &str,
    model: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut connection = open_connection(database)?;
    let transaction = connection.transaction()?;
    transaction.execute("DELETE FROM rag_chunks WHERE paper_id=?1", [paper_id])?;
    for chunk in chunks {
        transaction.execute(
            "INSERT INTO rag_chunks(id,paper_id,document_id,heading_path,content,chunk_index,
             content_hash,char_count) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",
            params![
                chunk.id,
                chunk.paper_id,
                chunk.document_id,
                chunk.heading_path,
                chunk.content,
                chunk.chunk_index as i64,
                chunk.content_hash,
                chunk.content.chars().count() as i64
            ],
        )?;
        transaction.execute(
            "INSERT INTO rag_embeddings(chunk_id,provider,model,dimensions,vector_json)
             VALUES(?1,?2,?3,?4,?5)",
            params![
                chunk.id,
                provider,
                model,
                chunk.vector.len() as i64,
                serde_json::to_string(&chunk.vector)?
            ],
        )?;
    }
    transaction.commit()?;
    Ok(())
}

pub fn indexed_chunks(
    database: &Path,
    paper_id: &str,
) -> Result<Vec<RagChunkRecord>, Box<dyn std::error::Error>> {
    let connection = open_connection(database)?;
    let mut statement = connection.prepare(
        "SELECT c.id,c.paper_id,c.document_id,c.heading_path,c.content,c.chunk_index,
         c.content_hash,e.vector_json FROM rag_chunks c JOIN rag_embeddings e ON e.chunk_id=c.id
         WHERE c.paper_id=?1 ORDER BY c.chunk_index",
    )?;
    let rows = statement.query_map([paper_id], |row| {
        let vector_json: String = row.get(7)?;
        let vector = serde_json::from_str(&vector_json).map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(
                7,
                rusqlite::types::Type::Text,
                Box::new(error),
            )
        })?;
        Ok(RagChunkRecord {
            id: row.get(0)?,
            paper_id: row.get(1)?,
            document_id: row.get(2)?,
            heading_path: row.get(3)?,
            content: row.get(4)?,
            chunk_index: row.get::<_, i64>(5)? as usize,
            content_hash: row.get(6)?,
            vector,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

pub fn index_summary(
    database: &Path,
    paper_id: &str,
) -> rusqlite::Result<Option<(usize, String, String, String)>> {
    open_connection(database)?
        .query_row(
            "SELECT COUNT(*), MAX(c.updated_at), MIN(e.provider), MIN(e.model)
         FROM rag_chunks c JOIN rag_embeddings e ON e.chunk_id=c.id WHERE c.paper_id=?1
         HAVING COUNT(*) > 0",
            [paper_id],
            |row| {
                Ok((
                    row.get::<_, i64>(0)? as usize,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                ))
            },
        )
        .optional()
}

pub fn ensure_scope_session(
    database: &Path,
    scope_type: &str,
    scope_id: &str,
    title: &str,
) -> rusqlite::Result<String> {
    let connection = open_connection(database)?;
    if let Some(id) = connection
        .query_row(
            "SELECT id FROM rag_sessions WHERE scope_type=?1 AND scope_id=?2",
            params![scope_type, scope_id],
            |row| row.get(0),
        )
        .optional()?
    {
        return Ok(id);
    }
    let id = uuid::Uuid::new_v4().to_string();
    connection.execute(
        "INSERT INTO rag_sessions(id,scope_type,scope_id,title) VALUES(?1,?2,?3,?4)",
        params![id, scope_type, scope_id, title],
    )?;
    Ok(id)
}

pub fn append_exchange(
    database: &Path,
    session_id: &str,
    question: &str,
    answer: &str,
    citations_json: &str,
) -> rusqlite::Result<()> {
    let mut connection = open_connection(database)?;
    let transaction = connection.transaction()?;
    transaction.execute(
        "INSERT INTO rag_messages(id,session_id,role,content,citations_json) VALUES(?1,?2,'user',?3,'[]')",
        params![uuid::Uuid::new_v4().to_string(), session_id, question],
    )?;
    transaction.execute(
        "INSERT INTO rag_messages(id,session_id,role,content,citations_json) VALUES(?1,?2,'assistant',?3,?4)",
        params![uuid::Uuid::new_v4().to_string(), session_id, answer, citations_json],
    )?;
    transaction.execute(
        "UPDATE rag_sessions SET updated_at=CURRENT_TIMESTAMP WHERE id=?1",
        [session_id],
    )?;
    transaction.commit()?;
    Ok(())
}

pub fn scope_messages(
    database: &Path,
    scope_type: &str,
    scope_id: &str,
) -> rusqlite::Result<Vec<RagMessageRecord>> {
    let connection = open_connection(database)?;
    let mut statement = connection.prepare(
        "SELECT m.id,m.role,m.content,m.citations_json,m.created_at FROM rag_messages m
         JOIN rag_sessions s ON s.id=m.session_id
         WHERE s.scope_type=?1 AND s.scope_id=?2 ORDER BY m.rowid",
    )?;
    let messages = statement
        .query_map(params![scope_type, scope_id], |row| {
            Ok(RagMessageRecord {
                id: row.get(0)?,
                role: row.get(1)?,
                content: row.get(2)?,
                citations_json: row.get(3)?,
                created_at: row.get(4)?,
            })
        })?
        .collect();
    messages
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::db;

    #[test]
    fn replaces_index_and_persists_paper_chat() {
        let root = std::env::temp_dir().join(format!("reflo-rag-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&root).unwrap();
        let database = root.join("reflo.sqlite");
        db::initialize(&database).unwrap();
        let connection = rusqlite::Connection::open(&database).unwrap();
        connection.execute("PRAGMA foreign_keys=ON", []).unwrap();
        connection.execute("INSERT INTO papers(id,title,authors_json,created_at) VALUES('p','Paper','[]',CURRENT_TIMESTAMP)", []).unwrap();
        connection.execute("INSERT INTO documents(id,paper_id,original_name,relative_path,sha256,created_at) VALUES('d','p','p.pdf','library/p/p.pdf','hash',CURRENT_TIMESTAMP)", []).unwrap();
        drop(connection);

        let chunk = RagChunkRecord {
            id: "c".into(),
            paper_id: "p".into(),
            document_id: "d".into(),
            heading_path: "Method".into(),
            content: "Evidence".into(),
            chunk_index: 0,
            content_hash: "content-hash".into(),
            vector: vec![0.1, 0.2],
        };
        replace_index(&database, "p", &[chunk], "openai", "embedding-model").unwrap();
        assert_eq!(indexed_chunks(&database, "p").unwrap().len(), 1);
        assert_eq!(index_summary(&database, "p").unwrap().unwrap().0, 1);

        let session = ensure_scope_session(&database, "paper", "p", "Paper").unwrap();
        append_exchange(&database, &session, "Question", "Answer", "[]").unwrap();
        let messages = scope_messages(&database, "paper", "p").unwrap();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, "user");
        assert_eq!(messages[1].role, "assistant");
        let _ = std::fs::remove_dir_all(root);
    }
}
