use crate::infrastructure::database::connection::ConnectionRef;

pub const SCHEMA_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS documents (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    title TEXT NOT NULL,
    doc_type TEXT NOT NULL,
    year INTEGER,
    source TEXT,
    url TEXT,
    tags TEXT,
    notes TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS document_metadata (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    document_id INTEGER NOT NULL,
    key TEXT NOT NULL,
    value TEXT NOT NULL,
    FOREIGN KEY (document_id) REFERENCES documents(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_documents_doc_type ON documents(doc_type);
CREATE INDEX IF NOT EXISTS idx_document_metadata_document_id ON document_metadata(document_id);
"#;

pub fn initialize(conn: &ConnectionRef) -> anyhow::Result<()> {
    let conn = conn.lock().unwrap();
    conn.execute_batch(SCHEMA_SQL)?;
    Ok(())
}
