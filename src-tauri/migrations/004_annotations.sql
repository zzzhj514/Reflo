CREATE TABLE annotations (
    id TEXT PRIMARY KEY NOT NULL,
    document_id TEXT NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
    page_index INTEGER NOT NULL CHECK (page_index >= 0),
    kind TEXT NOT NULL CHECK (kind IN ('highlight', 'note')),
    selected_text TEXT NOT NULL,
    note TEXT,
    rects_json TEXT NOT NULL,
    color TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_annotations_document_page
    ON annotations(document_id, page_index, created_at);
