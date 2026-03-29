#![allow(dead_code)]

use crate::domain::entities::DocumentType;

pub struct ListDocumentsFilter {
    pub doc_types: Option<Vec<String>>,
    pub tags: Option<Vec<String>>,
    pub authors: Option<Vec<String>>,
}

impl ListDocumentsFilter {
    pub fn is_empty(&self) -> bool {
        self.doc_types.is_none() && self.tags.is_none() && self.authors.is_none()
    }
}

pub struct ListDocumentsInput {
    pub doc_types: Option<Vec<String>>,
    pub tags: Option<Vec<String>>,
    pub authors: Option<Vec<String>>,
}

impl From<ListDocumentsInput> for ListDocumentsFilter {
    fn from(input: ListDocumentsInput) -> Self {
        Self {
            doc_types: input.doc_types,
            tags: input.tags,
            authors: input.authors,
        }
    }
}

pub enum DocumentMetadata {
    Book {
        authors: Option<Vec<String>>,
        edition: Option<String>,
        publisher: Option<String>,
        isbn: Option<String>,
    },
    Paper {
        authors: Option<Vec<String>>,
        journal: Option<String>,
        volume: Option<String>,
        issue: Option<String>,
        doi: Option<String>,
    },
    Lecture {
        event: Option<String>,
        institution: Option<String>,
        location: Option<String>,
        topic: Option<String>,
    },
    Notes,
}

impl From<crate::domain::entities::Document> for DocumentSummaryOutput {
    fn from(doc: crate::domain::entities::Document) -> Self {
        let doc_type_str = doc.doc_type.as_str().to_string();

        let metadata = match doc.doc_type {
            DocumentType::Book(m) => DocumentMetadata::Book {
                authors: m.authors,
                edition: m.edition,
                publisher: m.publisher,
                isbn: m.isbn,
            },
            DocumentType::Paper(m) => DocumentMetadata::Paper {
                authors: m.authors,
                journal: m.journal,
                volume: m.volume,
                issue: m.issue,
                doi: m.doi,
            },
            DocumentType::Lecture(m) => DocumentMetadata::Lecture {
                event: m.event,
                institution: m.institution,
                location: m.location,
                topic: m.topic,
            },
            DocumentType::Notes(_) => DocumentMetadata::Notes,
        };

        Self {
            id: doc.id,
            title: doc.title,
            doc_type: doc_type_str,
            year: doc.year,
            source: doc.source,
            url: doc.url,
            tags: doc.tags,
            notes: doc.notes,
            created_at: doc.created_at,
            updated_at: doc.updated_at,
            metadata,
        }
    }
}

pub struct DocumentSummaryOutput {
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
    pub metadata: DocumentMetadata,
}
