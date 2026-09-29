use std::path::Path;

use rusqlite::{params, OptionalExtension};

use crate::domain::paper_tree::{PaperTree, PaperTreeNode};

use super::open_connection;

pub fn get(path: &Path, paper_id: &str) -> Result<Option<PaperTree>, Box<dyn std::error::Error>> {
    let connection = open_connection(path)?;
    let stored = connection
        .query_row(
            "SELECT title, nodes_json, updated_at FROM paper_trees WHERE paper_id = ?1",
            [paper_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )
        .optional()?;

    stored
        .map(|(title, nodes_json, updated_at)| {
            Ok(PaperTree {
                paper_id: paper_id.to_owned(),
                title,
                nodes: serde_json::from_str::<Vec<PaperTreeNode>>(&nodes_json)?,
                updated_at: Some(updated_at),
                saved: true,
            })
        })
        .transpose()
}

pub fn save(
    path: &Path,
    paper_id: &str,
    title: &str,
    nodes: &[PaperTreeNode],
) -> Result<PaperTree, Box<dyn std::error::Error>> {
    let nodes_json = serde_json::to_string(nodes)?;
    let connection = open_connection(path)?;
    connection.execute(
        "INSERT INTO paper_trees(paper_id, title, nodes_json, updated_at)
         VALUES (?1, ?2, ?3, CURRENT_TIMESTAMP)
         ON CONFLICT(paper_id) DO UPDATE SET
           title = excluded.title,
           nodes_json = excluded.nodes_json,
           updated_at = CURRENT_TIMESTAMP",
        params![paper_id, title, nodes_json],
    )?;

    get(path, paper_id)?.ok_or_else(|| "Paper Tree 保存后无法读取".into())
}

pub fn paper_exists(path: &Path, paper_id: &str) -> rusqlite::Result<bool> {
    open_connection(path)?.query_row(
        "SELECT EXISTS(SELECT 1 FROM papers WHERE id = ?1)",
        [paper_id],
        |row| row.get(0),
    )
}
