CREATE TABLE translation_settings (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    provider TEXT NOT NULL,
    base_url TEXT NOT NULL,
    model TEXT NOT NULL,
    target_language TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT INTO translation_settings(id, provider, base_url, model, target_language)
VALUES (1, 'deepseek', 'https://api.deepseek.com', 'deepseek-chat', 'auto');

