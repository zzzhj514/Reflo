CREATE TABLE conversion_settings (
    provider TEXT PRIMARY KEY,
    model TEXT NOT NULL,
    language TEXT NOT NULL,
    enable_ocr INTEGER NOT NULL,
    enable_formula INTEGER NOT NULL,
    enable_table INTEGER NOT NULL,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT INTO conversion_settings(
    provider, model, language, enable_ocr, enable_formula, enable_table
) VALUES ('mineru', 'vlm', 'en', 0, 1, 1);

