use std::collections::HashMap;
use std::sync::Mutex;

use crate::application::dto::DeleteDocumentInput;
use crate::application::repositories::DocumentRepository;
use crate::application::use_cases::DeleteDocumentUseCase;
use crate::domain::entities::{Document, DocumentType, NotesMetadata};

struct MockRepository {
    documents: Mutex<HashMap<i64, Document>>,
    next_id: Mutex<i64>,
}

impl MockRepository {
    fn new() -> Self {
        Self {
            documents: Mutex::new(HashMap::new()),
            next_id: Mutex::new(1),
        }
    }

    fn add_test_doc(&self) -> i64 {
        let doc = Document::new(
            0,
            "Test Document".to_string(),
            DocumentType::Notes(NotesMetadata {}),
            None,
            None,
            None,
            vec![],
            None,
            "2024-01-15T10:00:00Z".to_string(),
            "2024-01-15T10:00:00Z".to_string(),
        )
        .unwrap();

        let id = *self.next_id.lock().unwrap();
        let mut d = doc;
        d.id = id;
        self.documents.lock().unwrap().insert(id, d);
        *self.next_id.lock().unwrap() += 1;
        id
    }
}

impl DocumentRepository for MockRepository {
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

    fn find_all_with_filter(
        &self,
        _filter: crate::application::dto::ListDocumentsFilter,
    ) -> Result<Vec<Document>, anyhow::Error> {
        unimplemented!()
    }

    fn search(&self, _query: &str) -> Result<Vec<Document>, anyhow::Error> {
        unimplemented!()
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

#[test]
fn test_delete_existing() {
    let repo = MockRepository::new();
    let doc_id = repo.add_test_doc();

    let use_case = DeleteDocumentUseCase::new(repo);
    let input = DeleteDocumentInput { id: doc_id };

    let result = use_case.execute(input);

    assert!(result.is_ok());
}

#[test]
fn test_delete_nonexistent() {
    let repo = MockRepository::new();
    repo.add_test_doc();

    let use_case = DeleteDocumentUseCase::new(repo);
    let input = DeleteDocumentInput { id: 999 };

    let result = use_case.execute(input);

    assert!(result.is_err());
}
