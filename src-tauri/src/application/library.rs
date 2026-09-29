use std::fs;
use std::path::Path;

use crate::domain::paper::{Document, Paper};
use crate::infrastructure::{db::papers, storage};

pub struct ImportOutcome {
    pub paper_id: String,
    pub duplicate: bool,
}

pub fn import_pdf(source: &Path, data_dir: &Path) -> Result<ImportOutcome, String> {
    let database = data_dir.join("reflo.sqlite");
    let staged = storage::stage_pdf(source, data_dir).map_err(|e| e.to_string())?;
    let destination_dir = data_dir.join("library").join(&staged.document_id);
    let destination = destination_dir.join("original.pdf");
    let mut created_destination = false;

    let result = (|| -> Result<ImportOutcome, Box<dyn std::error::Error>> {
        if let Some(paper_id) = papers::find_paper_id_by_sha256(&database, &staged.sha256)? {
            return Ok(ImportOutcome {
                paper_id,
                duplicate: true,
            });
        }

        let paper_id = uuid::Uuid::new_v4().to_string();
        let created_at = papers::current_timestamp(&database)?;
        let title = source
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let paper = Paper {
            revision: 0,
            id: paper_id.clone(),
            title,
            authors: vec![],
            year: None,
            doi: None,
            source_url: None,
            venue: None,
            publisher: None,
            group_id: None,
            created_at: created_at.clone(),
        };
        let document = Document {
            id: staged.document_id.clone(),
            paper_id: paper_id.clone(),
            original_name: staged.original_name.clone(),
            relative_path: format!("library/{}/original.pdf", staged.document_id),
            sha256: staged.sha256.clone(),
            created_at,
        };

        fs::create_dir_all(data_dir.join("library"))?;
        fs::create_dir(&destination_dir)?;
        created_destination = true;
        fs::rename(&staged.staged_path, &destination)?;

        if let Err(error) = papers::insert_paper_with_document(&database, &paper, &document) {
            // A concurrent import may have committed the same fingerprint first.
            if let Some(existing) = papers::find_paper_id_by_sha256(&database, &staged.sha256)? {
                return Ok(ImportOutcome {
                    paper_id: existing,
                    duplicate: true,
                });
            }
            return Err(error);
        }
        Ok(ImportOutcome {
            paper_id,
            duplicate: false,
        })
    })();

    let keep_file = matches!(&result, Ok(outcome) if !outcome.duplicate);
    let mut cleanup_errors = Vec::new();
    if !keep_file {
        if let Err(e) = fs::remove_file(&staged.staged_path) {
            if e.kind() != std::io::ErrorKind::NotFound {
                cleanup_errors.push(e.to_string());
            }
        }
        if created_destination {
            if let Err(e) = fs::remove_file(&destination) {
                if e.kind() != std::io::ErrorKind::NotFound {
                    cleanup_errors.push(e.to_string());
                }
            }
            if let Err(e) = fs::remove_dir(&destination_dir) {
                cleanup_errors.push(e.to_string());
            }
        }
    }
    if !cleanup_errors.is_empty() {
        return Err(format!(
            "{}；临时文件清理失败：{}",
            result
                .err()
                .map(|e| e.to_string())
                .unwrap_or_else(|| "检测到重复文件".into()),
            cleanup_errors.join("；")
        ));
    }
    result.map_err(|e| e.to_string())
}

pub fn list_papers(data_dir: &Path) -> Result<Vec<Paper>, String> {
    papers::list_papers(&data_dir.join("reflo.sqlite")).map_err(|e| e.to_string())
}

pub fn artifact_status(
    data_dir: &Path,
    paper_id: &str,
) -> Result<papers::PaperArtifactStatus, String> {
    papers::artifact_status(&data_dir.join("reflo.sqlite"), paper_id)
        .map_err(|error| error.to_string())
}

pub fn update_metadata(
    data_dir: &Path,
    mut update: crate::domain::paper::MetadataUpdate,
) -> Result<Paper, String> {
    update.title = update.title.trim().to_owned();
    if update.title.is_empty() {
        return Err("标题不能为空".into());
    }
    if update.title.chars().count() > 2000 {
        return Err("标题不能超过 2000 个字符".into());
    }
    if update.year.is_some_and(|year| !(1..=9999).contains(&year)) {
        return Err("年份必须是 1 到 9999 之间的整数".into());
    }
    update.authors = update
        .authors
        .into_iter()
        .map(|author| author.trim().to_owned())
        .filter(|author| !author.is_empty())
        .collect();
    update.doi = update
        .doi
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty());
    update.source_url = update
        .source_url
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty());
    update.venue = update
        .venue
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty());
    update.publisher = update
        .publisher
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty());
    if let Some(value) = &update.source_url {
        let url = url::Url::parse(value).map_err(|_| "链接必须是有效的 http 或 https 地址")?;
        if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
            return Err("链接必须是有效的 http 或 https 地址".into());
        }
    }
    papers::update_metadata(&data_dir.join("reflo.sqlite"), &update)
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::db;

    struct Fixture(std::path::PathBuf);
    impl Fixture {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!("reflo-test-{}", uuid::Uuid::new_v4()));
            fs::create_dir(&path).unwrap();
            db::initialize(&path.join("reflo.sqlite")).unwrap();
            Self(path)
        }
        fn pdf(&self) -> std::path::PathBuf {
            let path = self.0.join("example.pdf");
            // Header-only fixture tests storage, not PDF renderability.
            fs::write(&path, b"%PDF-1.7\nfixture\n%%EOF").unwrap();
            path
        }
        fn staging_empty(&self) -> bool {
            let path = self.0.join("staging");
            !path.exists() || fs::read_dir(path).unwrap().next().is_none()
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn imports_persists_and_deduplicates_identical_content_under_another_name() {
        let fixture = Fixture::new();
        let source = fixture.pdf();
        let imported = import_pdf(&source, &fixture.0).unwrap();
        assert!(!imported.duplicate);
        db::initialize(&fixture.0.join("reflo.sqlite")).unwrap();
        let papers = list_papers(&fixture.0).unwrap();
        assert_eq!(papers.len(), 1);
        assert_eq!(papers[0].title, "example");
        let artifacts = artifact_status(&fixture.0, &imported.paper_id).unwrap();
        assert_eq!(artifacts.original_name, "example.pdf");
        assert!(!artifacts.has_markdown);
        assert!(!artifacts.has_translation);
        let renamed = fixture.0.join("renamed.pdf");
        fs::copy(&source, &renamed).unwrap();
        let duplicate = import_pdf(&renamed, &fixture.0).unwrap();
        assert!(duplicate.duplicate);
        assert_eq!(duplicate.paper_id, imported.paper_id);
        let connection = rusqlite::Connection::open(fixture.0.join("reflo.sqlite")).unwrap();
        let relative: String = connection
            .query_row("SELECT relative_path FROM documents", [], |r| r.get(0))
            .unwrap();
        assert_eq!(
            fs::read(fixture.0.join(relative)).unwrap(),
            fs::read(source).unwrap()
        );
        assert_eq!(list_papers(&fixture.0).unwrap().len(), 1);
        assert!(fixture.staging_empty());
        assert_eq!(fs::read_dir(fixture.0.join("library")).unwrap().count(), 1);
    }

    #[test]
    fn rejects_non_pdf_without_creating_a_record() {
        let fixture = Fixture::new();
        let source = fixture.0.join("fake.pdf");
        fs::write(&source, "not a pdf").unwrap();
        assert!(import_pdf(&source, &fixture.0).is_err());
        assert!(list_papers(&fixture.0).unwrap().is_empty());
        assert!(fixture.staging_empty());
    }

    #[test]
    fn database_failure_rolls_back_paper_and_cleans_copied_file() {
        let fixture = Fixture::new();
        let connection = rusqlite::Connection::open(fixture.0.join("reflo.sqlite")).unwrap();
        connection.execute_batch("CREATE TRIGGER reject_document BEFORE INSERT ON documents BEGIN SELECT RAISE(ABORT, 'test failure'); END;").unwrap();
        let source = fixture.pdf();
        assert!(import_pdf(&source, &fixture.0).is_err());
        assert!(source.exists());
        assert!(list_papers(&fixture.0).unwrap().is_empty());
        assert!(fixture.staging_empty());
        assert_eq!(fs::read_dir(fixture.0.join("library")).unwrap().count(), 0);
    }

    #[test]
    fn lookup_failure_cleans_staging_and_does_not_mean_no_duplicate() {
        let fixture = Fixture::new();
        let connection = rusqlite::Connection::open(fixture.0.join("reflo.sqlite")).unwrap();
        connection.execute_batch("DROP TABLE documents;").unwrap();
        assert!(import_pdf(&fixture.pdf(), &fixture.0).is_err());
        assert!(fixture.staging_empty());
        assert!(list_papers(&fixture.0).unwrap().is_empty());
    }

    #[test]
    fn concurrent_identical_imports_leave_one_record_and_file() {
        let fixture = Fixture::new();
        let source = fixture.pdf();
        let results = std::thread::scope(|scope| {
            let first = scope.spawn(|| import_pdf(&source, &fixture.0));
            let second = scope.spawn(|| import_pdf(&source, &fixture.0));
            (
                first.join().unwrap().unwrap(),
                second.join().unwrap().unwrap(),
            )
        });
        assert_eq!(results.0.paper_id, results.1.paper_id);
        assert_ne!(results.0.duplicate, results.1.duplicate);
        assert_eq!(list_papers(&fixture.0).unwrap().len(), 1);
        assert!(fixture.staging_empty());
        assert_eq!(fs::read_dir(fixture.0.join("library")).unwrap().count(), 1);
    }
    fn edited(paper: &Paper) -> crate::domain::paper::MetadataUpdate {
        crate::domain::paper::MetadataUpdate {
            id: paper.id.clone(),
            expected_revision: paper.revision,
            title: "  新的论文标题  ".into(),
            authors: vec![" 张三 ".into(), " ".into(), "李四".into()],
            year: Some(2026),
            doi: Some(" 10.1234/example ".into()),
            source_url: Some("https://example.org/paper".into()),
            venue: Some(" Test Conference ".into()),
            publisher: Some(" Test Publisher ".into()),
        }
    }

    #[test]
    fn metadata_persists_and_preserves_document_and_import_identity() {
        let fixture = Fixture::new();
        let source = fixture.pdf();
        import_pdf(&source, &fixture.0).unwrap();
        let original = list_papers(&fixture.0).unwrap().remove(0);
        let saved = update_metadata(&fixture.0, edited(&original)).unwrap();
        assert_eq!(saved.revision, 1);
        assert_eq!(saved.title, "新的论文标题");
        assert_eq!(saved.authors, vec!["张三", "李四"]);
        assert_eq!(saved.created_at, original.created_at);
        db::initialize(&fixture.0.join("reflo.sqlite")).unwrap();
        let loaded = list_papers(&fixture.0).unwrap().remove(0);
        assert_eq!(loaded.year, Some(2026));
        assert_eq!(loaded.doi.as_deref(), Some("10.1234/example"));
        assert_eq!(loaded.venue.as_deref(), Some("Test Conference"));
        assert_eq!(loaded.publisher.as_deref(), Some("Test Publisher"));
        assert_eq!(
            loaded.source_url.as_deref(),
            Some("https://example.org/paper")
        );
        assert_eq!(loaded.title, saved.title);
        assert!(import_pdf(&source, &fixture.0).unwrap().duplicate);
        assert_eq!(
            list_papers(&fixture.0).unwrap().remove(0).title,
            saved.title
        );
    }

    #[test]
    fn invalid_metadata_and_stale_writes_do_not_overwrite_saved_data() {
        let fixture = Fixture::new();
        import_pdf(&fixture.pdf(), &fixture.0).unwrap();
        let original = list_papers(&fixture.0).unwrap().remove(0);
        let mut invalid = edited(&original);
        invalid.title = "  ".into();
        assert!(update_metadata(&fixture.0, invalid).is_err());
        let mut invalid = edited(&original);
        invalid.year = Some(0);
        assert!(update_metadata(&fixture.0, invalid).is_err());
        let mut invalid = edited(&original);
        invalid.source_url = Some("javascript:alert(1)".into());
        assert!(update_metadata(&fixture.0, invalid).is_err());
        assert_eq!(list_papers(&fixture.0).unwrap().remove(0).revision, 0);
        update_metadata(&fixture.0, edited(&original)).unwrap();
        assert!(update_metadata(&fixture.0, edited(&original)).is_err());
        assert_eq!(list_papers(&fixture.0).unwrap().remove(0).revision, 1);
    }

    #[test]
    fn optional_metadata_can_be_cleared_and_missing_id_is_an_error() {
        let fixture = Fixture::new();
        import_pdf(&fixture.pdf(), &fixture.0).unwrap();
        let original = list_papers(&fixture.0).unwrap().remove(0);
        let saved = update_metadata(&fixture.0, edited(&original)).unwrap();
        let mut blank = edited(&saved);
        blank.authors = vec![];
        blank.doi = Some("  ".into());
        blank.source_url = None;
        blank.year = None;
        let cleared = update_metadata(&fixture.0, blank).unwrap();
        assert!(cleared.doi.is_none() && cleared.source_url.is_none() && cleared.year.is_none());
        assert!(cleared.authors.is_empty());
        let mut missing = edited(&cleared);
        missing.id = "missing".into();
        assert!(update_metadata(&fixture.0, missing).is_err());
    }

    #[test]
    fn version_one_database_upgrades_without_losing_papers() {
        let fixture = Fixture::new();
        let path = fixture.0.join("old.sqlite");
        let connection = rusqlite::Connection::open(&path).unwrap();
        connection
            .execute_batch(include_str!("../../migrations/001_initial.sql"))
            .unwrap();
        connection.execute_batch("PRAGMA user_version = 1; INSERT INTO papers(id,title) VALUES('old','Existing paper');").unwrap();
        drop(connection);
        db::initialize(&path).unwrap();
        db::initialize(&path).unwrap();
        let papers = papers::list_papers(&path).unwrap();
        assert_eq!(papers.len(), 1);
        assert_eq!(papers[0].title, "Existing paper");
        assert_eq!(papers[0].revision, 0);
    }
}
