use std::path::Path;

use crate::infrastructure::db::organization;

pub use organization::{PaperGroupRecord, TodoRecord};

fn clean_title(value: String, label: &str, max: usize) -> Result<String, String> {
    let value = value.trim().to_owned();
    if value.is_empty() {
        return Err(format!("{label}不能为空"));
    }
    if value.chars().count() > max {
        return Err(format!("{label}不能超过 {max} 个字符"));
    }
    Ok(value)
}

pub fn list_groups(data_dir: &Path) -> Result<Vec<PaperGroupRecord>, String> {
    organization::list_groups(&data_dir.join("reflo.sqlite")).map_err(|e| e.to_string())
}

pub fn create_group(data_dir: &Path, name: String) -> Result<PaperGroupRecord, String> {
    let name = clean_title(name, "分组名称", 100)?;
    let id = uuid::Uuid::new_v4().to_string();
    organization::create_group(&data_dir.join("reflo.sqlite"), &id, &name).map_err(|e| {
        if e.to_string().contains("UNIQUE") {
            "已存在同名分组".into()
        } else {
            e.to_string()
        }
    })?;
    Ok(PaperGroupRecord {
        id,
        name,
        paper_count: 0,
    })
}

pub fn rename_group(data_dir: &Path, id: &str, name: String) -> Result<(), String> {
    let name = clean_title(name, "分组名称", 100)?;
    if organization::rename_group(&data_dir.join("reflo.sqlite"), id, &name).map_err(|e| {
        if e.to_string().contains("UNIQUE") {
            "已存在同名分组".into()
        } else {
            e.to_string()
        }
    })? {
        Ok(())
    } else {
        Err("找不到该分组".into())
    }
}

pub fn delete_group(data_dir: &Path, id: &str) -> Result<(), String> {
    if organization::delete_group(&data_dir.join("reflo.sqlite"), id).map_err(|e| e.to_string())? {
        Ok(())
    } else {
        Err("找不到该分组".into())
    }
}

pub fn assign_paper(
    data_dir: &Path,
    paper_id: &str,
    group_id: Option<String>,
) -> Result<(), String> {
    let database = data_dir.join("reflo.sqlite");
    if let Some(id) = group_id.as_deref() {
        if !organization::group_exists(&database, id).map_err(|e| e.to_string())? {
            return Err("找不到目标分组".into());
        }
    }
    if organization::assign_paper(&database, paper_id, group_id.as_deref())
        .map_err(|e| e.to_string())?
    {
        Ok(())
    } else {
        Err("找不到这篇文献".into())
    }
}

pub fn list_todos(data_dir: &Path) -> Result<Vec<TodoRecord>, String> {
    organization::list_todos(&data_dir.join("reflo.sqlite")).map_err(|e| e.to_string())
}

pub fn create_todo(data_dir: &Path, title: String) -> Result<TodoRecord, String> {
    let title = clean_title(title, "待办内容", 500)?;
    let id = uuid::Uuid::new_v4().to_string();
    organization::create_todo(&data_dir.join("reflo.sqlite"), &id, &title)
        .map_err(|e| e.to_string())?;
    let created_at =
        crate::infrastructure::db::papers::current_timestamp(&data_dir.join("reflo.sqlite"))
            .map_err(|e| e.to_string())?;
    Ok(TodoRecord {
        id,
        title,
        completed: false,
        created_at,
    })
}

pub fn set_todo_completed(data_dir: &Path, id: &str, completed: bool) -> Result<(), String> {
    if organization::set_todo_completed(&data_dir.join("reflo.sqlite"), id, completed)
        .map_err(|e| e.to_string())?
    {
        Ok(())
    } else {
        Err("找不到该待办".into())
    }
}

pub fn delete_todo(data_dir: &Path, id: &str) -> Result<(), String> {
    if organization::delete_todo(&data_dir.join("reflo.sqlite"), id).map_err(|e| e.to_string())? {
        Ok(())
    } else {
        Err("找不到该待办".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{application::library, infrastructure::db};

    #[test]
    fn groups_papers_and_persists_todos() {
        let root =
            std::env::temp_dir().join(format!("reflo-organization-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&root).unwrap();
        db::initialize(&root.join("reflo.sqlite")).unwrap();
        let pdf = root.join("paper.pdf");
        std::fs::write(&pdf, b"%PDF-1.7\nfixture").unwrap();
        let paper = library::import_pdf(&pdf, &root).unwrap();
        let group = create_group(&root, "机器学习".into()).unwrap();
        assign_paper(&root, &paper.paper_id, Some(group.id.clone())).unwrap();
        assert_eq!(
            library::list_papers(&root).unwrap()[0].group_id.as_deref(),
            Some(group.id.as_str())
        );
        assert_eq!(list_groups(&root).unwrap()[0].paper_count, 1);
        delete_group(&root, &group.id).unwrap();
        assert!(library::list_papers(&root).unwrap()[0].group_id.is_none());

        let todo = create_todo(&root, "阅读论文".into()).unwrap();
        set_todo_completed(&root, &todo.id, true).unwrap();
        assert!(list_todos(&root).unwrap()[0].completed);
        delete_todo(&root, &todo.id).unwrap();
        assert!(list_todos(&root).unwrap().is_empty());
        let _ = std::fs::remove_dir_all(root);
    }
}
