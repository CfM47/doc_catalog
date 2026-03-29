use std::collections::HashMap;
use std::sync::Mutex;

use crate::application::repositories::DocumentRepository;
use crate::domain::entities::{Document, DocumentType, NotesMetadata};

struct MockDocumentRepository {
    documents: Mutex<HashMap<i64, Document>>,
    next_id: Mutex<i64>,
}

impl MockDocumentRepository {
    fn new() -> Self {
        Self {
            documents: Mutex::new(HashMap::new()),
            next_id: Mutex::new(1),
        }
    }
}

impl DocumentRepository for MockDocumentRepository {
    fn create(&self, mut document: Document) -> Result<Document, anyhow::Error> {
        let id = *self.next_id.lock().unwrap();
        *self.next_id.lock().unwrap() += 1;
        document.id = id;
        self.documents.lock().unwrap().insert(id, document.clone());
        Ok(document)
    }

    fn find_by_id(&self, id: i64) -> Result<Document, anyhow::Error> {
        self.documents
            .lock()
            .unwrap()
            .get(&id)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Document not found: {}", id))
    }

    fn find_all(&self) -> Result<Vec<Document>, anyhow::Error> {
        Ok(self.documents.lock().unwrap().values().cloned().collect())
    }

    fn update(&self, document: Document) -> Result<Document, anyhow::Error> {
        if !self.documents.lock().unwrap().contains_key(&document.id) {
            return Err(anyhow::anyhow!("Document not found: {}", document.id));
        }
        self.documents
            .lock()
            .unwrap()
            .insert(document.id, document.clone());
        Ok(document)
    }

    fn delete(&self, id: i64) -> Result<(), anyhow::Error> {
        if self.documents.lock().unwrap().remove(&id).is_none() {
            return Err(anyhow::anyhow!("Document not found: {}", id));
        }
        Ok(())
    }
}

fn create_test_document(title: &str) -> Document {
    Document::new(
        0,
        title.to_string(),
        DocumentType::Notes(NotesMetadata {}),
        Some(2024),
        Some("Test Source".to_string()),
        Some("https://example.com".to_string()),
        vec!["test".to_string()],
        Some("Test notes".to_string()),
        "2024-01-15T10:00:00Z".to_string(),
        "2024-01-15T10:00:00Z".to_string(),
    )
    .unwrap()
}

#[test]
fn test_create_document() {
    let repo = MockDocumentRepository::new();
    let doc = create_test_document("Test Document");

    let result = repo.create(doc);

    assert!(result.is_ok());
    assert_eq!(result.unwrap().id, 1);
}

#[test]
fn test_find_by_id() {
    let repo = MockDocumentRepository::new();
    let doc = create_test_document("Test Document");
    let created = repo.create(doc).unwrap();

    let found = repo.find_by_id(created.id);

    assert!(found.is_ok());
    assert_eq!(found.unwrap().title, "Test Document");
}

#[test]
fn test_find_by_id_not_found() {
    let repo = MockDocumentRepository::new();

    let result = repo.find_by_id(999);

    assert!(result.is_err());
}

#[test]
fn test_find_all() {
    let repo = MockDocumentRepository::new();
    repo.create(create_test_document("Doc 1")).unwrap();
    repo.create(create_test_document("Doc 2")).unwrap();

    let all = repo.find_all();

    assert!(all.is_ok());
    assert_eq!(all.unwrap().len(), 2);
}

#[test]
fn test_update_document() {
    let repo = MockDocumentRepository::new();
    let doc = create_test_document("Original");
    let mut created = repo.create(doc).unwrap();

    created.title = "Updated".to_string();
    let result = repo.update(created);

    assert!(result.is_ok());
    assert_eq!(result.unwrap().title, "Updated");
}

#[test]
fn test_update_not_found() {
    let repo = MockDocumentRepository::new();
    let mut doc = create_test_document("Test");
    doc.id = 999;

    let result = repo.update(doc);

    assert!(result.is_err());
}

#[test]
fn test_delete_document() {
    let repo = MockDocumentRepository::new();
    let doc = create_test_document("Test");
    let created = repo.create(doc).unwrap();

    let result = repo.delete(created.id);

    assert!(result.is_ok());
    assert!(repo.find_by_id(created.id).is_err());
}

#[test]
fn test_delete_not_found() {
    let repo = MockDocumentRepository::new();

    let result = repo.delete(999);

    assert!(result.is_err());
}
