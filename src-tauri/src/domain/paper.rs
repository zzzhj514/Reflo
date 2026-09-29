#[derive(Debug, Clone)]
pub struct Paper {
    pub revision: i64,
    pub id: String,
    pub title: String,
    pub authors: Vec<String>,
    pub year: Option<i32>,
    pub doi: Option<String>,
    pub source_url: Option<String>,
    pub venue: Option<String>,
    pub publisher: Option<String>,
    pub created_at: String,
}

pub struct MetadataUpdate {
    pub id: String,
    pub expected_revision: i64,
    pub title: String,
    pub authors: Vec<String>,
    pub year: Option<i32>,
    pub doi: Option<String>,
    pub source_url: Option<String>,
    pub venue: Option<String>,
    pub publisher: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Document {
    pub id: String,
    pub paper_id: String,
    pub original_name: String,
    pub relative_path: String,
    pub sha256: String,
    pub created_at: String,
}
