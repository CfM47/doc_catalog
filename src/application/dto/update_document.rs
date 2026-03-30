#![allow(dead_code)]

use crate::domain::entities::DocumentType;

pub struct UpdateDocumentInput {
    pub id: i64,
    pub title: Option<String>,
    pub doc_type: Option<DocumentType>,
    pub year: Option<i32>,
    pub source: Option<String>,
    pub url: Option<String>,
    pub tags: Option<Vec<String>>,
    pub notes: Option<String>,
}
