#![allow(dead_code)]

use std::sync::Mutex;

use crate::application::dto::ListDocumentsFilter;
use crate::application::repositories::DocumentRepository;
use crate::domain::entities::{Document, DocumentType, NotesMetadata};

pub struct MockRepository {
    documents: Mutex<Vec<Document>>,
    next_id: Mutex<i64>,
}

impl MockRepository {
    pub fn new() -> Self {
        Self {
            documents: Mutex::new(Vec::new()),
            next_id: Mutex::new(1),
        }
    }

    pub fn with_documents(docs: Vec<Document>) -> Self {
        let max_id = docs.iter().map(|d| d.id).max().unwrap_or(0);
        Self {
            documents: Mutex::new(docs),
            next_id: Mutex::new(max_id + 1),
        }
    }

    pub fn add_document(&self, doc: Document) {
        let mut docs = self.documents.lock().unwrap();
        let id = docs.len() as i64 + 1;
        let mut d = doc;
        d.id = id;
        docs.push(d);
    }

    pub fn add_test_doc(&self, title: &str) -> i64 {
        let doc = Document::new(
            0,
            title.to_string(),
            DocumentType::Notes(NotesMetadata {}),
            Some(2023),
            Some("Original Source".to_string()),
            Some("https://original.com".to_string()),
            vec!["original-tag".to_string()],
            Some("Original notes".to_string()),
            "2024-01-15T10:00:00Z".to_string(),
            "2024-01-15T10:00:00Z".to_string(),
        )
        .unwrap();

        let id = *self.next_id.lock().unwrap();
        let mut d = doc;
        d.id = id;
        self.documents.lock().unwrap().push(d);
        *self.next_id.lock().unwrap() += 1;
        id
    }
}

impl DocumentRepository for MockRepository {
    fn create(&self, mut document: Document) -> anyhow::Result<Document> {
        let id = *self.next_id.lock().unwrap();
        *self.next_id.lock().unwrap() += 1;
        document.id = id;
        self.documents.lock().unwrap().push(document.clone());
        Ok(document)
    }

    fn find_by_id(&self, id: i64) -> anyhow::Result<Document> {
        self.documents
            .lock()
            .unwrap()
            .iter()
            .find(|d| d.id == id)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Not found"))
    }

    fn find_all(&self) -> anyhow::Result<Vec<Document>> {
        Ok(self.documents.lock().unwrap().clone())
    }

    fn find_all_with_filter(&self, filter: ListDocumentsFilter) -> anyhow::Result<Vec<Document>> {
        let docs = self.documents.lock().unwrap();
        let mut results: Vec<Document> = docs.clone();

        if let Some(doc_types) = &filter.doc_types {
            if !doc_types.is_empty() {
                results.retain(|doc| doc_types.contains(&doc.doc_type.as_str().to_string()));
            }
        }

        if let Some(tags) = &filter.tags {
            if !tags.is_empty() {
                results.retain(|doc| doc.tags.iter().any(|t| tags.contains(t)));
            }
        }

        if let Some(authors) = &filter.authors {
            if !authors.is_empty() {
                results.retain(|doc| match &doc.doc_type {
                    DocumentType::Book(m) => {
                        if let Some(doc_authors) = &m.authors {
                            doc_authors.iter().any(|a| {
                                authors
                                    .iter()
                                    .any(|f| a.to_lowercase().contains(&f.to_lowercase()))
                            })
                        } else {
                            false
                        }
                    }
                    DocumentType::Paper(m) => {
                        if let Some(doc_authors) = &m.authors {
                            doc_authors.iter().any(|a| {
                                authors
                                    .iter()
                                    .any(|f| a.to_lowercase().contains(&f.to_lowercase()))
                            })
                        } else {
                            false
                        }
                    }
                    _ => false,
                });
            }
        }

        Ok(results)
    }

    fn search(&self, query: &str) -> anyhow::Result<Vec<Document>> {
        let docs = self.documents.lock().unwrap();
        let query_lower = query.to_lowercase();
        let results: Vec<Document> = docs
            .iter()
            .filter(|d| d.title.to_lowercase().contains(&query_lower))
            .cloned()
            .collect();
        Ok(results)
    }

    fn update(&self, document: Document) -> anyhow::Result<Document> {
        let mut docs = self.documents.lock().unwrap();
        if let Some(idx) = docs.iter().position(|d| d.id == document.id) {
            docs[idx] = document.clone();
            Ok(document)
        } else {
            Err(anyhow::anyhow!("Not found"))
        }
    }

    fn delete(&self, id: i64) -> anyhow::Result<()> {
        let mut docs = self.documents.lock().unwrap();
        if let Some(idx) = docs.iter().position(|d| d.id == id) {
            docs.remove(idx);
            Ok(())
        } else {
            Err(anyhow::anyhow!("Not found"))
        }
    }
}

impl Clone for MockRepository {
    fn clone(&self) -> Self {
        Self::with_documents(self.documents.lock().unwrap().clone())
    }
}
