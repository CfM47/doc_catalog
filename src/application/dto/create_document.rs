#![allow(dead_code)]

use crate::domain::entities::{Document, DocumentType};

pub struct CreateDocumentInput {
    pub title: String,
    pub doc_type: DocumentType,
    pub year: Option<i32>,
    pub source: Option<String>,
    pub url: Option<String>,
    pub tags: Vec<String>,
    pub notes: Option<String>,
}

pub struct CreateDocumentOutput {
    pub id: i64,
    pub title: String,
    pub doc_type: String,
    pub year: Option<i32>,
    pub source: Option<String>,
    pub url: Option<String>,
    pub tags: Vec<String>,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<Document> for CreateDocumentOutput {
    fn from(doc: Document) -> Self {
        Self {
            id: doc.id,
            title: doc.title,
            doc_type: doc.doc_type.as_str().to_string(),
            year: doc.year,
            source: doc.source,
            url: doc.url,
            tags: doc.tags,
            notes: doc.notes,
            created_at: doc.created_at,
            updated_at: doc.updated_at,
        }
    }
}
