use std::path::Path;

use rusqlite::{params, OptionalExtension};

use crate::domain::paper::{Document, MetadataUpdate, Paper};

use super::open_connection;

pub struct PaperArtifactStatus {
    pub original_name: String,
    pub has_markdown: bool,
}

pub fn artifact_status(path: &Path, paper_id: &str) -> rusqlite::Result<PaperArtifactStatus> {
    open_connection(path)?.query_row(
        "SELECT d.original_name, EXISTS(
            SELECT 1 FROM markdown_documents m WHERE m.document_id = d.id
         )
         FROM documents d WHERE d.paper_id = ?1 ORDER BY d.created_at LIMIT 1",
        [paper_id],
        |row| {
            Ok(PaperArtifactStatus {
                original_name: row.get(0)?,
                has_markdown: row.get(1)?,
            })
        },
    )
}

pub fn insert_paper_with_document(
    database_path: &Path,
    paper: &Paper,
    document: &Document,
) -> Result<(), Box<dyn std::error::Error>> {
    if document.paper_id != paper.id {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "PDF 记录关联的论文 ID 不匹配",
        )
        .into());
    }

    let authors_json = serde_json::to_string(&paper.authors)?;

    let mut connection = open_connection(database_path)?;
    let transaction = connection.transaction()?;

    transaction.execute(
        "INSERT INTO papers (
            id, title, authors_json, year, doi, source_url, created_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            paper.id,
            paper.title,
            authors_json,
            paper.year,
            paper.doi,
            paper.source_url,
            paper.created_at,
        ],
    )?;

    transaction.execute(
        "INSERT INTO documents (
            id, paper_id, original_name, relative_path, sha256, created_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            document.id,
            document.paper_id,
            document.original_name,
            document.relative_path,
            document.sha256,
            document.created_at,
        ],
    )?;

    transaction.commit()?;

    Ok(())
}

pub fn find_paper_id_by_sha256(path: &Path, sha256: &str) -> rusqlite::Result<Option<String>> {
    open_connection(path)?
        .query_row(
            "SELECT paper_id FROM documents WHERE sha256 = ?1",
            params![sha256],
            |row| row.get(0),
        )
        .optional()
}

pub fn list_papers(path: &Path) -> Result<Vec<Paper>, Box<dyn std::error::Error>> {
    let connection = open_connection(path)?;
    let mut statement = connection.prepare(
        "SELECT id, title, authors_json, year, doi, source_url, created_at, revision FROM papers ORDER BY created_at DESC, id"
    )?;
    let mut rows = statement.query([])?;
    let mut papers = Vec::new();
    while let Some(row) = rows.next()? {
        let authors_json: String = row.get(2)?;
        papers.push(Paper {
            revision: row.get(7)?,
            id: row.get(0)?,
            title: row.get(1)?,
            authors: serde_json::from_str(&authors_json)?,
            year: row.get(3)?,
            doi: row.get(4)?,
            source_url: row.get(5)?,
            created_at: row.get(6)?,
        });
    }
    Ok(papers)
}

pub fn current_timestamp(path: &Path) -> rusqlite::Result<String> {
    open_connection(path)?.query_row("SELECT CURRENT_TIMESTAMP", [], |row| row.get(0))
}

pub fn update_metadata(
    path: &Path,
    update: &MetadataUpdate,
) -> Result<Paper, Box<dyn std::error::Error>> {
    let mut connection = open_connection(path)?;
    let transaction = connection.transaction()?;
    let authors_json = serde_json::to_string(&update.authors)?;
    let changed = transaction.execute(
        "UPDATE papers SET title=?1, authors_json=?2, year=?3, doi=?4, source_url=?5, revision=revision+1 WHERE id=?6 AND revision=?7",
        params![update.title, authors_json, update.year, update.doi, update.source_url, update.id, update.expected_revision],
    )?;
    if changed != 1 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "文献不存在或已被其他操作修改，请先重新加载文献再保存",
        )
        .into());
    }
    let created_at = transaction.query_row(
        "SELECT created_at FROM papers WHERE id=?1",
        params![update.id],
        |row| row.get(0),
    )?;
    transaction.commit()?;
    Ok(Paper {
        id: update.id.clone(),
        revision: update.expected_revision + 1,
        title: update.title.clone(),
        authors: update.authors.clone(),
        year: update.year,
        doi: update.doi.clone(),
        source_url: update.source_url.clone(),
        created_at,
    })
}
