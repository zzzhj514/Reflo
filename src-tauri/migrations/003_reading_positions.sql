CREATE TABLE reading_positions (
    document_id TEXT PRIMARY KEY NOT NULL REFERENCES documents(id),
    page_index INTEGER NOT NULL CHECK (page_index >= 0),
    scale REAL NOT NULL CHECK (scale >= 0.25 AND scale <= 3),
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
