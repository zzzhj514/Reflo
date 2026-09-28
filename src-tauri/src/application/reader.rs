use crate::infrastructure::db::reader;
use std::path::{Path, PathBuf};

pub struct OpenedDocument {
    pub document: reader::ReadingDocument,
    pub path: PathBuf,
}

pub fn open_document(data_dir: &Path, paper_id: &str) -> Result<OpenedDocument, String> {
    let document = reader::document_for_paper(&data_dir.join("reflo.sqlite"), paper_id)
        .map_err(|e| e.to_string())?
        .ok_or("这篇文献没有 PDF 文件")?;
    // Reject paths outside the managed library, including symlinks escaping it.
    let library = data_dir
        .join("library")
        .canonicalize()
        .map_err(|_| "找不到本地文献库")?;
    let path = data_dir
        .join(&document.relative_path)
        .canonicalize()
        .map_err(|_| "PDF 文件已丢失或无法访问")?;
    if !path.starts_with(&library) || !path.is_file() {
        return Err("PDF 不在允许访问的文献库范围内".into());
    }
    Ok(OpenedDocument { document, path })
}

pub fn save_position(data_dir: &Path, id: &str, page_index: u32, scale: f64) -> Result<(), String> {
    if !scale.is_finite() || !(0.25..=3.0).contains(&scale) {
        return Err("缩放比例必须在 25% 到 300% 之间".into());
    }
    reader::save_position(&data_dir.join("reflo.sqlite"), id, page_index, scale)
        .map_err(|e| e.to_string())
}

pub struct AnnotationInput {
    pub document_id: String,
    pub page_index: u32,
    pub kind: String,
    pub selected_text: String,
    pub note: Option<String>,
    pub rects: Vec<reader::AnnotationRect>,
    pub color: String,
}

pub fn list_annotations(
    data_dir: &Path,
    document_id: &str,
) -> Result<Vec<reader::Annotation>, String> {
    reader::list_annotations(&data_dir.join("reflo.sqlite"), document_id).map_err(|e| e.to_string())
}

pub fn create_annotation(
    data_dir: &Path,
    input: AnnotationInput,
) -> Result<reader::Annotation, String> {
    let selected_text = input.selected_text.trim();
    if selected_text.is_empty() || selected_text.chars().count() > 20_000 {
        return Err("批注文字不能为空且不能超过 20000 字".into());
    }
    if !matches!(input.kind.as_str(), "highlight" | "note") {
        return Err("不支持的批注类型".into());
    }
    if input.kind == "note" && input.note.as_deref().unwrap_or("").trim().is_empty() {
        return Err("批注内容不能为空".into());
    }
    if input
        .note
        .as_deref()
        .map(str::chars)
        .map(Iterator::count)
        .unwrap_or(0)
        > 10_000
    {
        return Err("批注内容不能超过 10000 字".into());
    }
    if input.rects.is_empty()
        || input.rects.len() > 200
        || input.rects.iter().any(|rect| {
            [rect.x, rect.y, rect.width, rect.height]
                .iter()
                .any(|value| !value.is_finite())
                || rect.x < 0.0
                || rect.y < 0.0
                || rect.width <= 0.0
                || rect.height <= 0.0
                || rect.x + rect.width > 1.001
                || rect.y + rect.height > 1.001
        })
    {
        return Err("文字选择区域无效".into());
    }
    if !matches!(input.color.as_str(), "#fde047" | "#fb923c") {
        return Err("不支持的高亮颜色".into());
    }
    let id = uuid::Uuid::new_v4().to_string();
    reader::create_annotation(
        &data_dir.join("reflo.sqlite"),
        reader::NewAnnotation {
            id: &id,
            document_id: &input.document_id,
            page_index: input.page_index,
            kind: &input.kind,
            selected_text,
            note: input.note.as_deref().map(str::trim),
            rects: &input.rects,
            color: &input.color,
        },
    )
    .map_err(|e| e.to_string())
}

pub fn delete_annotation(data_dir: &Path, id: &str) -> Result<(), String> {
    if reader::delete_annotation(&data_dir.join("reflo.sqlite"), id).map_err(|e| e.to_string())? {
        Ok(())
    } else {
        Err("找不到这条批注".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{application::library, infrastructure::db};
    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> (Self, String) {
            let root = std::env::temp_dir().join(format!("reflo-reader-{}", uuid::Uuid::new_v4()));
            std::fs::create_dir(&root).unwrap();
            db::initialize(&root.join("reflo.sqlite")).unwrap();
            let source = root.join("sample.pdf");
            std::fs::write(&source, b"%PDF-1.7\nreader fixture").unwrap();
            let result = library::import_pdf(&source, &root).unwrap();
            (Self(root), result.paper_id)
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn opens_imported_file_and_restores_position_after_reinitialization() {
        let (fixture, paper) = Fixture::new();
        let opened = open_document(&fixture.0, &paper).unwrap();
        assert!(opened.path.is_file());
        assert_eq!(opened.document.page_index, 0);
        save_position(&fixture.0, &opened.document.id, 2, 1.5).unwrap();
        db::initialize(&fixture.0.join("reflo.sqlite")).unwrap();
        let reopened = open_document(&fixture.0, &paper).unwrap();
        assert_eq!(reopened.document.page_index, 2);
        assert_eq!(reopened.document.scale, 1.5);
        assert!(save_position(&fixture.0, "missing", 0, 1.0).is_err());
        assert!(save_position(&fixture.0, &opened.document.id, 0, f64::NAN).is_err());
        assert!(save_position(&fixture.0, &opened.document.id, 0, 100.0).is_err());
    }

    #[test]
    fn rejects_missing_documents_and_paths_outside_library() {
        let (fixture, paper) = Fixture::new();
        assert!(open_document(&fixture.0, "missing").is_err());
        let connection = rusqlite::Connection::open(fixture.0.join("reflo.sqlite")).unwrap();
        connection
            .execute(
                "UPDATE documents SET relative_path='sample.pdf' WHERE paper_id=?1",
                [&paper],
            )
            .unwrap();
        assert!(open_document(&fixture.0, &paper)
            .err()
            .unwrap()
            .contains("范围"));
    }

    #[test]
    fn annotations_persist_and_validate_selection_geometry() {
        let (fixture, paper) = Fixture::new();
        let document = open_document(&fixture.0, &paper).unwrap().document;
        let saved = create_annotation(
            &fixture.0,
            AnnotationInput {
                document_id: document.id.clone(),
                page_index: 1,
                kind: "note".into(),
                selected_text: "selected sentence".into(),
                note: Some("important result".into()),
                rects: vec![reader::AnnotationRect {
                    x: 0.1,
                    y: 0.2,
                    width: 0.3,
                    height: 0.04,
                }],
                color: "#fb923c".into(),
            },
        )
        .unwrap();
        db::initialize(&fixture.0.join("reflo.sqlite")).unwrap();
        let loaded = list_annotations(&fixture.0, &document.id).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].selected_text, "selected sentence");
        assert_eq!(loaded[0].note.as_deref(), Some("important result"));
        delete_annotation(&fixture.0, &saved.id).unwrap();
        assert!(list_annotations(&fixture.0, &document.id)
            .unwrap()
            .is_empty());

        let invalid = create_annotation(
            &fixture.0,
            AnnotationInput {
                document_id: document.id,
                page_index: 0,
                kind: "highlight".into(),
                selected_text: "bad rectangle".into(),
                note: None,
                rects: vec![reader::AnnotationRect {
                    x: 0.9,
                    y: 0.2,
                    width: 0.3,
                    height: 0.04,
                }],
                color: "#fde047".into(),
            },
        );
        assert!(invalid.unwrap_err().contains("选择区域"));
    }
}
