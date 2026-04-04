#![allow(dead_code)]

use std::sync::Mutex;

use crate::application::dto::{ListDocumentsFilter, ListDocumentsInput};
use crate::application::repositories::DocumentRepository;
use crate::cli::dependencies::CliDependencies;
use crate::domain::entities::{BookMetadata, Document, DocumentType};

struct MockRepository {
    documents: Mutex<Vec<Document>>,
}

impl MockRepository {
    fn new() -> Self {
        Self {
            documents: Mutex::new(Vec::new()),
        }
    }

    fn with_documents(docs: Vec<Document>) -> Self {
        Self {
            documents: Mutex::new(docs),
        }
    }
}

impl DocumentRepository for MockRepository {
    fn create(&self, document: Document) -> anyhow::Result<Document> {
        let mut docs = self.documents.lock().unwrap();
        let mut doc = document;
        doc.id = docs.len() as i64 + 1;
        docs.push(doc.clone());
        Ok(doc)
    }

    fn find_by_id(&self, id: i64) -> anyhow::Result<Document> {
        let docs = self.documents.lock().unwrap();
        docs.iter()
            .find(|d| d.id == id)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Not found"))
    }

    fn find_all(&self) -> anyhow::Result<Vec<Document>> {
        Ok(self.documents.lock().unwrap().clone())
    }

    fn find_all_with_filter(&self, _filter: ListDocumentsFilter) -> anyhow::Result<Vec<Document>> {
        Ok(self.documents.lock().unwrap().clone())
    }

    fn search(&self, _query: &str) -> anyhow::Result<Vec<Document>> {
        Ok(self.documents.lock().unwrap().clone())
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

    fn delete(&self, _id: i64) -> anyhow::Result<()> {
        Ok(())
    }
}

impl Clone for MockRepository {
    fn clone(&self) -> Self {
        Self::with_documents(self.documents.lock().unwrap().clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_deps_with_docs(docs: Vec<Document>) -> CliDependencies<MockRepository> {
        CliDependencies::new(MockRepository::with_documents(docs))
    }

    #[test]
    fn test_list_documents_returns_documents() {
        let docs = vec![Document {
            id: 1,
            title: "Test Book".to_string(),
            doc_type: DocumentType::Book(BookMetadata::default()),
            year: Some(2024),
            source: None,
            url: None,
            tags: vec!["rust".to_string()],
            notes: None,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        }];

        let deps = create_deps_with_docs(docs);

        let input = ListDocumentsInput {
            doc_types: None,
            tags: None,
            authors: None,
        };

        let result = deps.list_documents.execute(input);
        assert!(result.is_ok());
        let docs = result.unwrap();
        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0].title, "Test Book");
        assert_eq!(docs[0].doc_type, "book");
        assert_eq!(docs[0].tags, vec!["rust"]);
    }

    #[test]
    fn test_list_empty_returns_empty() {
        let deps = create_deps_with_docs(vec![]);

        let input = ListDocumentsInput {
            doc_types: None,
            tags: None,
            authors: None,
        };

        let result = deps.list_documents.execute(input);
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }
}
