CREATE TABLE rag_settings (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    provider TEXT NOT NULL,
    base_url TEXT NOT NULL,
    model TEXT NOT NULL,
    chunk_chars INTEGER NOT NULL CHECK (chunk_chars BETWEEN 500 AND 8000),
    top_k INTEGER NOT NULL CHECK (top_k BETWEEN 2 AND 20),
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT INTO rag_settings(id, provider, base_url, model, chunk_chars, top_k)
VALUES (1, 'openai', 'https://api.openai.com/v1', 'text-embedding-3-small', 2400, 6);

CREATE TABLE rag_chunks (
    id TEXT PRIMARY KEY NOT NULL,
    paper_id TEXT NOT NULL REFERENCES papers(id) ON DELETE CASCADE,
    document_id TEXT NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
    heading_path TEXT NOT NULL,
    content TEXT NOT NULL,
    chunk_index INTEGER NOT NULL,
    content_hash TEXT NOT NULL,
    char_count INTEGER NOT NULL,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(document_id, chunk_index)
);

CREATE INDEX idx_rag_chunks_paper ON rag_chunks(paper_id, chunk_index);

CREATE TABLE rag_embeddings (
    chunk_id TEXT PRIMARY KEY NOT NULL REFERENCES rag_chunks(id) ON DELETE CASCADE,
    provider TEXT NOT NULL,
    model TEXT NOT NULL,
    dimensions INTEGER NOT NULL,
    vector_json TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE rag_sessions (
    id TEXT PRIMARY KEY NOT NULL,
    scope_type TEXT NOT NULL CHECK (scope_type IN ('paper', 'topic')),
    scope_id TEXT NOT NULL,
    title TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(scope_type, scope_id)
);

CREATE TABLE rag_messages (
    id TEXT PRIMARY KEY NOT NULL,
    session_id TEXT NOT NULL REFERENCES rag_sessions(id) ON DELETE CASCADE,
    role TEXT NOT NULL CHECK (role IN ('user', 'assistant')),
    content TEXT NOT NULL,
    citations_json TEXT NOT NULL DEFAULT '[]',
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_rag_messages_session ON rag_messages(session_id, created_at, id);
