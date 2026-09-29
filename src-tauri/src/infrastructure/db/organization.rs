use std::path::Path;

use rusqlite::{params, OptionalExtension};

use super::open_connection;

#[derive(Debug)]
pub struct PaperGroupRecord {
    pub id: String,
    pub name: String,
    pub paper_count: usize,
}

#[derive(Debug)]
pub struct TodoRecord {
    pub id: String,
    pub title: String,
    pub completed: bool,
    pub created_at: String,
}

pub fn list_groups(path: &Path) -> rusqlite::Result<Vec<PaperGroupRecord>> {
    let connection = open_connection(path)?;
    let mut statement = connection.prepare(
        "SELECT g.id, g.name, COUNT(p.id) FROM paper_groups g
         LEFT JOIN papers p ON p.group_id = g.id
         GROUP BY g.id, g.name, g.created_at ORDER BY g.created_at, g.name",
    )?;
    let rows = statement
        .query_map([], |row| {
            let paper_count: i64 = row.get(2)?;
            Ok(PaperGroupRecord {
                id: row.get(0)?,
                name: row.get(1)?,
                paper_count: paper_count as usize,
            })
        })?
        .collect();
    rows
}

pub fn create_group(path: &Path, id: &str, name: &str) -> rusqlite::Result<()> {
    open_connection(path)?.execute(
        "INSERT INTO paper_groups(id, name) VALUES(?1, ?2)",
        params![id, name],
    )?;
    Ok(())
}

pub fn rename_group(path: &Path, id: &str, name: &str) -> rusqlite::Result<bool> {
    Ok(open_connection(path)?.execute(
        "UPDATE paper_groups SET name=?1 WHERE id=?2",
        params![name, id],
    )? == 1)
}

pub fn delete_group(path: &Path, id: &str) -> rusqlite::Result<bool> {
    Ok(open_connection(path)?.execute("DELETE FROM paper_groups WHERE id=?1", [id])? == 1)
}

pub fn assign_paper(path: &Path, paper_id: &str, group_id: Option<&str>) -> rusqlite::Result<bool> {
    Ok(open_connection(path)?.execute(
        "UPDATE papers SET group_id=?1 WHERE id=?2",
        params![group_id, paper_id],
    )? == 1)
}

pub fn group_exists(path: &Path, id: &str) -> rusqlite::Result<bool> {
    Ok(open_connection(path)?
        .query_row("SELECT 1 FROM paper_groups WHERE id=?1", [id], |_| Ok(()))
        .optional()?
        .is_some())
}

pub fn list_todos(path: &Path) -> rusqlite::Result<Vec<TodoRecord>> {
    let connection = open_connection(path)?;
    let mut statement = connection.prepare(
        "SELECT id, title, completed, created_at FROM todos ORDER BY completed, created_at DESC, id",
    )?;
    let rows = statement
        .query_map([], |row| {
            Ok(TodoRecord {
                id: row.get(0)?,
                title: row.get(1)?,
                completed: row.get(2)?,
                created_at: row.get(3)?,
            })
        })?
        .collect();
    rows
}

pub fn create_todo(path: &Path, id: &str, title: &str) -> rusqlite::Result<()> {
    open_connection(path)?.execute(
        "INSERT INTO todos(id, title) VALUES(?1, ?2)",
        params![id, title],
    )?;
    Ok(())
}

pub fn set_todo_completed(path: &Path, id: &str, completed: bool) -> rusqlite::Result<bool> {
    Ok(open_connection(path)?.execute(
        "UPDATE todos SET completed=?1, updated_at=CURRENT_TIMESTAMP WHERE id=?2",
        params![completed, id],
    )? == 1)
}

pub fn delete_todo(path: &Path, id: &str) -> rusqlite::Result<bool> {
    Ok(open_connection(path)?.execute("DELETE FROM todos WHERE id=?1", [id])? == 1)
}
