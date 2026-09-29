CREATE TABLE markdown_translations (
    document_id TEXT PRIMARY KEY REFERENCES documents(id) ON DELETE CASCADE,
    relative_path TEXT NOT NULL,
    provider TEXT NOT NULL,
    model TEXT NOT NULL,
    target_language TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

