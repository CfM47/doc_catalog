#![allow(dead_code)]

use rusqlite::params;

use crate::application::repositories::DocumentRepository;
use crate::domain::entities::{
    BookMetadata, Document, DocumentType, LectureMetadata, Metadata, NotesMetadata, PaperMetadata,
};
use crate::infrastructure::database::ConnectionRef;

pub struct SqliteDocumentRepository {
    conn: ConnectionRef,
}

impl SqliteDocumentRepository {
    pub fn new(conn: ConnectionRef) -> Self {
        Self { conn }
    }

    fn doc_type_to_string(doc_type: &DocumentType) -> String {
        match doc_type {
            DocumentType::Book(_) => "book".to_string(),
            DocumentType::Paper(_) => "paper".to_string(),
            DocumentType::Lecture(_) => "lecture".to_string(),
            DocumentType::Notes(_) => "notes".to_string(),
        }
    }

    fn string_to_doc_type(s: &str, metadata: Vec<Metadata>) -> DocumentType {
        match s {
            "book" => DocumentType::Book(BookMetadata {
                authors: metadata
                    .iter()
                    .find(|m| m.key == "authors")
                    .and_then(|m| serde_json::from_str(&m.value).ok()),
                edition: metadata
                    .iter()
                    .find(|m| m.key == "edition")
                    .map(|m| m.value.clone()),
                publisher: metadata
                    .iter()
                    .find(|m| m.key == "publisher")
                    .map(|m| m.value.clone()),
                isbn: metadata
                    .iter()
                    .find(|m| m.key == "isbn")
                    .map(|m| m.value.clone()),
            }),
            "paper" => DocumentType::Paper(PaperMetadata {
                authors: metadata
                    .iter()
                    .find(|m| m.key == "authors")
                    .and_then(|m| serde_json::from_str(&m.value).ok()),
                journal: metadata
                    .iter()
                    .find(|m| m.key == "journal")
                    .map(|m| m.value.clone()),
                volume: metadata
                    .iter()
                    .find(|m| m.key == "volume")
                    .map(|m| m.value.clone()),
                issue: metadata
                    .iter()
                    .find(|m| m.key == "issue")
                    .map(|m| m.value.clone()),
                doi: metadata
                    .iter()
                    .find(|m| m.key == "doi")
                    .map(|m| m.value.clone()),
            }),
            "lecture" => DocumentType::Lecture(LectureMetadata {
                event: metadata
                    .iter()
                    .find(|m| m.key == "event")
                    .map(|m| m.value.clone()),
                institution: metadata
                    .iter()
                    .find(|m| m.key == "institution")
                    .map(|m| m.value.clone()),
                location: metadata
                    .iter()
                    .find(|m| m.key == "location")
                    .map(|m| m.value.clone()),
                topic: metadata
                    .iter()
                    .find(|m| m.key == "topic")
                    .map(|m| m.value.clone()),
            }),
            _ => DocumentType::Notes(NotesMetadata {}),
        }
    }

    fn save_metadata(&self, document_id: i64, doc_type: &DocumentType) -> anyhow::Result<()> {
        let conn = self.conn.lock().unwrap();
        let metadata = match doc_type {
            DocumentType::Book(m) => vec![
                m.authors
                    .clone()
                    .map(|v| ("authors".to_string(), serde_json::to_string(&v).unwrap())),
                m.edition.clone().map(|v| ("edition".to_string(), v)),
                m.publisher.clone().map(|v| ("publisher".to_string(), v)),
                m.isbn.clone().map(|v| ("isbn".to_string(), v)),
            ],
            DocumentType::Paper(m) => vec![
                m.authors
                    .clone()
                    .map(|v| ("authors".to_string(), serde_json::to_string(&v).unwrap())),
                m.journal.clone().map(|v| ("journal".to_string(), v)),
                m.volume.clone().map(|v| ("volume".to_string(), v)),
                m.issue.clone().map(|v| ("issue".to_string(), v)),
                m.doi.clone().map(|v| ("doi".to_string(), v)),
            ],
            DocumentType::Lecture(m) => vec![
                m.event.clone().map(|v| ("event".to_string(), v)),
                m.institution
                    .clone()
                    .map(|v| ("institution".to_string(), v)),
                m.location.clone().map(|v| ("location".to_string(), v)),
                m.topic.clone().map(|v| ("topic".to_string(), v)),
            ],
            DocumentType::Notes(_) => vec![],
        };

        for (key, value) in metadata.into_iter().flatten() {
            conn.execute(
                "INSERT INTO document_metadata (document_id, key, value) VALUES (?1, ?2, ?3)",
                params![document_id, key, value],
            )?;
        }
        Ok(())
    }

    fn load_metadata(&self, document_id: i64) -> anyhow::Result<Vec<Metadata>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt =
            conn.prepare("SELECT key, value FROM document_metadata WHERE document_id = ?1")?;
        let rows = stmt.query_map([document_id], |row| {
            Ok(Metadata {
                document_id,
                key: row.get(0)?,
                value: row.get(1)?,
            })
        })?;
        let mut metadata = Vec::new();
        for row in rows {
            metadata.push(row?);
        }
        Ok(metadata)
    }
}

impl DocumentRepository for SqliteDocumentRepository {
    fn create(&self, document: Document) -> Result<Document, anyhow::Error> {
        let conn = self.conn.lock().unwrap();
        let doc_type_str = Self::doc_type_to_string(&document.doc_type);
        let tags_json = serde_json::to_string(&document.tags)?;

        conn.execute(
            "INSERT INTO documents (title, doc_type, year, source, url, tags, notes, created_at, updated_at) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                document.title,
                doc_type_str,
                document.year,
                document.source,
                document.url,
                tags_json,
                document.notes,
                document.created_at,
                document.updated_at,
            ],
        )?;

        let id = conn.last_insert_rowid();
        drop(conn);

        self.save_metadata(id, &document.doc_type)?;

        let mut doc = document;
        doc.id = id;
        Ok(doc)
    }

    fn find_by_id(&self, id: i64) -> Result<Document, anyhow::Error> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, title, doc_type, year, source, url, tags, notes, created_at, updated_at 
             FROM documents WHERE id = ?1",
        )?;

        let doc = stmt.query_row([id], |row| {
            let tags_json: Option<String> = row.get(6)?;
            let tags: Vec<String> = tags_json
                .map(|t| serde_json::from_str(&t).unwrap_or_default())
                .unwrap_or_default();
            Ok(Document {
                id: row.get(0)?,
                title: row.get(1)?,
                doc_type: DocumentType::Notes(NotesMetadata {}),
                year: row.get(3)?,
                source: row.get(4)?,
                url: row.get(5)?,
                tags,
                notes: row.get(7)?,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
            })
        })?;

        let doc_type_str: String = conn.query_row(
            "SELECT doc_type FROM documents WHERE id = ?1",
            [id],
            |row| row.get(0),
        )?;

        let metadata = {
            let mut stmt =
                conn.prepare("SELECT key, value FROM document_metadata WHERE document_id = ?1")?;
            let rows = stmt.query_map([id], |row| {
                Ok(Metadata {
                    document_id: id,
                    key: row.get(0)?,
                    value: row.get(1)?,
                })
            })?;
            rows.filter_map(|r| r.ok()).collect()
        };

        let doc_type = Self::string_to_doc_type(&doc_type_str, metadata);

        let mut result = doc;
        result.doc_type = doc_type;
        Ok(result)
    }

    fn find_all(&self) -> Result<Vec<Document>, anyhow::Error> {
        let raw_documents = {
            let conn = self.conn.lock().unwrap();
            let mut stmt = conn.prepare(
                "SELECT id, title, doc_type, year, source, url, tags, notes, created_at, updated_at 
                 FROM documents ORDER BY id",
            )?;

            let rows = stmt.query_map([], |row| {
                let tags_json: Option<String> = row.get(6)?;
                let tags: Vec<String> = tags_json
                    .map(|t| serde_json::from_str(&t).unwrap_or_default())
                    .unwrap_or_default();
                Ok((
                    Document {
                        id: row.get(0)?,
                        title: row.get(1)?,
                        doc_type: DocumentType::Notes(NotesMetadata {}),
                        year: row.get(3)?,
                        source: row.get(4)?,
                        url: row.get(5)?,
                        tags,
                        notes: row.get(7)?,
                        created_at: row.get(8)?,
                        updated_at: row.get(9)?,
                    },
                    row.get::<_, String>(2)?,
                ))
            })?;

            rows.filter_map(|r| r.ok()).collect::<Vec<_>>()
        };

        let mut documents = Vec::new();
        for (mut doc, doc_type_str) in raw_documents {
            let metadata = self.load_metadata(doc.id)?;
            doc.doc_type = Self::string_to_doc_type(&doc_type_str, metadata);
            documents.push(doc);
        }

        Ok(documents)
    }

    fn find_all_with_filter(
        &self,
        filter: crate::application::dto::ListDocumentsFilter,
    ) -> Result<Vec<Document>, anyhow::Error> {
        if filter.is_empty() {
            return self.find_all();
        }

        let raw_documents = {
            let conn = self.conn.lock().unwrap();

            let mut sql = String::from(
                "SELECT DISTINCT d.id, d.title, d.doc_type, d.year, d.source, d.url, d.tags, d.notes, d.created_at, d.updated_at 
                 FROM documents d"
            );

            let need_metadata_join = filter.authors.is_some();
            if need_metadata_join {
                sql.push_str(" LEFT JOIN document_metadata m ON d.id = m.document_id");
            }

            let mut conditions = Vec::new();
            let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

            if let Some(doc_types) = &filter.doc_types
                && !doc_types.is_empty()
            {
                let placeholders: Vec<String> = doc_types.iter().map(|_| "?".to_string()).collect();
                conditions.push(format!("d.doc_type IN ({})", placeholders.join(",")));
                for dt in doc_types {
                    params.push(Box::new(dt.clone()));
                }
            }

            if let Some(tags) = &filter.tags
                && !tags.is_empty()
            {
                for tag in tags {
                    conditions.push(format!("d.tags LIKE '%\"{}\"%'", tag));
                }
            }

            if need_metadata_join
                && let Some(authors) = &filter.authors
                && !authors.is_empty()
            {
                let author_conditions: Vec<String> = authors
                    .iter()
                    .map(|a| {
                        format!(
                            "(m.key = 'authors' AND LOWER(m.value) LIKE '%{}%')",
                            a.to_lowercase()
                        )
                    })
                    .collect();
                conditions.push(format!("({})", author_conditions.join(" OR ")));
            }

            if !conditions.is_empty() {
                sql.push_str(" WHERE ");
                sql.push_str(&conditions.join(" AND "));
            }

            sql.push_str(" ORDER BY d.id");

            let params_ref: Vec<&dyn rusqlite::ToSql> = params.iter().map(|b| b.as_ref()).collect();
            let mut stmt = conn.prepare(&sql)?;

            let rows = stmt.query_map(params_ref.as_slice(), |row| {
                let tags_json: Option<String> = row.get(6)?;
                let tags: Vec<String> = tags_json
                    .map(|t| serde_json::from_str(&t).unwrap_or_default())
                    .unwrap_or_default();
                Ok((
                    Document {
                        id: row.get(0)?,
                        title: row.get(1)?,
                        doc_type: crate::domain::entities::DocumentType::Notes(
                            crate::domain::entities::NotesMetadata {},
                        ),
                        year: row.get(3)?,
                        source: row.get(4)?,
                        url: row.get(5)?,
                        tags,
                        notes: row.get(7)?,
                        created_at: row.get(8)?,
                        updated_at: row.get(9)?,
                    },
                    row.get::<_, String>(2)?,
                ))
            })?;

            rows.filter_map(|r| r.ok()).collect::<Vec<_>>()
        };

        let mut documents = Vec::new();
        for (mut doc, doc_type_str) in raw_documents {
            let metadata = self.load_metadata(doc.id)?;
            doc.doc_type = Self::string_to_doc_type(&doc_type_str, metadata);
            documents.push(doc);
        }

        Ok(documents)
    }

    fn search(&self, query: &str) -> Result<Vec<Document>, anyhow::Error> {
        let query_lower = query.to_lowercase();

        let raw_documents = {
            let conn = self.conn.lock().unwrap();

            let sql = String::from(
                "SELECT DISTINCT d.id, d.title, d.doc_type, d.year, d.source, d.url, d.tags, d.notes, d.created_at, d.updated_at 
                 FROM documents d
                 LEFT JOIN document_metadata m ON d.id = m.document_id
                 WHERE LOWER(d.title) LIKE ?1
                    OR LOWER(d.tags) LIKE ?1
                    OR LOWER(d.notes) LIKE ?1
                    OR LOWER(m.value) LIKE ?1
                 ORDER BY d.id"
            );

            let pattern = format!("%{}%", query_lower);
            let mut stmt = conn.prepare(&sql)?;

            let rows = stmt.query_map([&pattern], |row| {
                let tags_json: Option<String> = row.get(6)?;
                let tags: Vec<String> = tags_json
                    .map(|t| serde_json::from_str(&t).unwrap_or_default())
                    .unwrap_or_default();
                Ok((
                    Document {
                        id: row.get(0)?,
                        title: row.get(1)?,
                        doc_type: crate::domain::entities::DocumentType::Notes(
                            crate::domain::entities::NotesMetadata {},
                        ),
                        year: row.get(3)?,
                        source: row.get(4)?,
                        url: row.get(5)?,
                        tags,
                        notes: row.get(7)?,
                        created_at: row.get(8)?,
                        updated_at: row.get(9)?,
                    },
                    row.get::<_, String>(2)?,
                ))
            })?;

            rows.filter_map(|r| r.ok()).collect::<Vec<_>>()
        };

        let mut documents = Vec::new();
        for (mut doc, doc_type_str) in raw_documents {
            let metadata = self.load_metadata(doc.id)?;
            doc.doc_type = Self::string_to_doc_type(&doc_type_str, metadata);
            documents.push(doc);
        }

        Ok(documents)
    }

    fn update(&self, document: Document) -> Result<Document, anyhow::Error> {
        let conn = self.conn.lock().unwrap();
        let doc_type_str = Self::doc_type_to_string(&document.doc_type);
        let tags_json = serde_json::to_string(&document.tags)?;

        let rows_affected = conn.execute(
            "UPDATE documents SET title = ?1, doc_type = ?2, year = ?3, source = ?4, url = ?5, tags = ?6, notes = ?7, updated_at = ?8 WHERE id = ?9",
            params![
                document.title,
                doc_type_str,
                document.year,
                document.source,
                document.url,
                tags_json,
                document.notes,
                document.updated_at,
                document.id,
            ],
        )?;

        if rows_affected == 0 {
            return Err(anyhow::anyhow!("Document not found: {}", document.id));
        }

        conn.execute(
            "DELETE FROM document_metadata WHERE document_id = ?1",
            [document.id],
        )?;
        drop(conn);

        self.save_metadata(document.id, &document.doc_type)?;

        Ok(document)
    }

    fn delete(&self, id: i64) -> Result<(), anyhow::Error> {
        let conn = self.conn.lock().unwrap();

        let rows_affected = conn.execute("DELETE FROM documents WHERE id = ?1", [id])?;

        if rows_affected == 0 {
            return Err(anyhow::anyhow!("Document not found: {}", id));
        }

        Ok(())
    }
}
