#![allow(dead_code)]

use std::sync::Mutex;

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

    fn find_all_with_filter(
        &self,
        _filter: crate::application::dto::ListDocumentsFilter,
    ) -> anyhow::Result<Vec<Document>> {
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
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_deps() -> CliDependencies<MockRepository> {
        CliDependencies::new(MockRepository::new())
    }

    #[test]
    fn test_cli_dependencies_have_create_use_case() {
        let deps = create_deps();
        let _ = deps.create_document;
    }

    #[test]
    fn test_cli_dependencies_have_list_use_case() {
        let deps = create_deps();
        let _ = deps.list_documents;
    }

    #[test]
    fn test_cli_dependencies_have_search_use_case() {
        let deps = create_deps();
        let _ = deps.search_documents;
    }

    #[test]
    fn test_cli_dependencies_have_update_use_case() {
        let deps = create_deps();
        let _ = deps.update_document;
    }

    #[test]
    fn test_cli_dependencies_have_delete_use_case() {
        let deps = create_deps();
        let _ = deps.delete_document;
    }

    #[test]
    fn test_document_type_book_metadata_keys() {
        let doc_type = DocumentType::Book(BookMetadata::default());
        let keys = doc_type.metadata_keys();
        assert!(keys.contains(&"authors"));
        assert!(keys.contains(&"edition"));
        assert!(keys.contains(&"publisher"));
        assert!(keys.contains(&"isbn"));
    }

    #[test]
    fn test_document_type_notes_has_no_metadata() {
        let doc_type = DocumentType::Notes(crate::domain::entities::NotesMetadata::default());
        let keys = doc_type.metadata_keys();
        assert!(keys.is_empty());
    }
}
