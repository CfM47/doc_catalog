use std::collections::HashMap;
use std::sync::Mutex;

use crate::application::dto::UpdateDocumentInput;
use crate::application::repositories::DocumentRepository;
use crate::application::use_cases::UpdateDocumentUseCase;
use crate::domain::entities::{BookMetadata, Document, DocumentType, NotesMetadata};

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

    fn add_test_doc(&self, doc: Document) -> i64 {
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

fn create_test_document() -> Document {
    Document::new(
        0,
        "Original Title".to_string(),
        DocumentType::Notes(NotesMetadata {}),
        Some(2023),
        Some("Original Source".to_string()),
        Some("https://original.com".to_string()),
        vec!["original-tag".to_string()],
        Some("Original notes".to_string()),
        "2024-01-15T10:00:00Z".to_string(),
        "2024-01-15T10:00:00Z".to_string(),
    )
    .unwrap()
}

#[test]
fn test_update_title() {
    let repo = MockRepository::new();
    let doc_id = repo.add_test_doc(create_test_document());

    let use_case = UpdateDocumentUseCase::new(repo);
    let input = UpdateDocumentInput {
        id: doc_id,
        title: Some("New Title".to_string()),
        doc_type: None,
        year: None,
        source: None,
        url: None,
        tags: None,
        notes: None,
    };

    let result = use_case.execute(input);

    assert!(result.is_ok());
    assert_eq!(result.unwrap().title, "New Title");
}

#[test]
fn test_update_year() {
    let repo = MockRepository::new();
    let doc_id = repo.add_test_doc(create_test_document());

    let use_case = UpdateDocumentUseCase::new(repo);
    let input = UpdateDocumentInput {
        id: doc_id,
        title: None,
        doc_type: None,
        year: Some(2024),
        source: None,
        url: None,
        tags: None,
        notes: None,
    };

    let result = use_case.execute(input);

    assert!(result.is_ok());
    assert_eq!(result.unwrap().year, Some(2024));
}

#[test]
fn test_update_tags() {
    let repo = MockRepository::new();
    let doc_id = repo.add_test_doc(create_test_document());

    let use_case = UpdateDocumentUseCase::new(repo);
    let input = UpdateDocumentInput {
        id: doc_id,
        title: None,
        doc_type: None,
        year: None,
        source: None,
        url: None,
        tags: Some(vec!["new-tag".to_string(), "rust".to_string()]),
        notes: None,
    };

    let result = use_case.execute(input);

    assert!(result.is_ok());
    assert_eq!(result.unwrap().tags, vec!["new-tag", "rust"]);
}

#[test]
fn test_update_metadata() {
    let repo = MockRepository::new();
    let doc = Document::new(
        0,
        "Book Title".to_string(),
        DocumentType::Book(BookMetadata {
            authors: Some(vec!["Original Author".to_string()]),
            edition: None,
            publisher: None,
            isbn: None,
        }),
        Some(2023),
        None,
        None,
        vec![],
        None,
        "2024-01-15T10:00:00Z".to_string(),
        "2024-01-15T10:00:00Z".to_string(),
    )
    .unwrap();
    let doc_id = repo.add_test_doc(doc);

    let use_case = UpdateDocumentUseCase::new(repo);
    let input = UpdateDocumentInput {
        id: doc_id,
        title: None,
        doc_type: Some(DocumentType::Book(BookMetadata {
            authors: Some(vec!["New Author".to_string()]),
            edition: Some("2nd".to_string()),
            publisher: Some("O'Reilly".to_string()),
            isbn: None,
        })),
        year: None,
        source: None,
        url: None,
        tags: None,
        notes: None,
    };

    let result = use_case.execute(input);

    assert!(result.is_ok());
    match result.unwrap().doc_type {
        DocumentType::Book(m) => {
            assert_eq!(m.authors, Some(vec!["New Author".to_string()]));
            assert_eq!(m.edition, Some("2nd".to_string()));
            assert_eq!(m.publisher, Some("O'Reilly".to_string()));
        }
        _ => panic!("Expected Book document type"),
    }
}

#[test]
fn test_preserve_unmodified_fields() {
    let repo = MockRepository::new();
    let doc_id = repo.add_test_doc(create_test_document());

    let use_case = UpdateDocumentUseCase::new(repo);
    let input = UpdateDocumentInput {
        id: doc_id,
        title: Some("New Title".to_string()),
        doc_type: None,
        year: None,
        source: None,
        url: None,
        tags: None,
        notes: None,
    };

    let result = use_case.execute(input);

    assert!(result.is_ok());
    let updated = result.unwrap();

    // Verify unmodified fields are preserved
    assert_eq!(updated.year, Some(2023));
    assert_eq!(updated.source, Some("Original Source".to_string()));
    assert_eq!(updated.url, Some("https://original.com".to_string()));
    assert_eq!(updated.tags, vec!["original-tag"]);
    assert_eq!(updated.notes, Some("Original notes".to_string()));
}

#[test]
fn test_update_nonexistent_document() {
    let repo = MockRepository::new();
    repo.add_test_doc(create_test_document());

    let use_case = UpdateDocumentUseCase::new(repo);
    let input = UpdateDocumentInput {
        id: 999,
        title: Some("New Title".to_string()),
        doc_type: None,
        year: None,
        source: None,
        url: None,
        tags: None,
        notes: None,
    };

    let result = use_case.execute(input);

    assert!(result.is_err());
}

#[test]
fn test_update_empty_title() {
    let repo = MockRepository::new();
    let doc_id = repo.add_test_doc(create_test_document());

    let use_case = UpdateDocumentUseCase::new(repo);
    let input = UpdateDocumentInput {
        id: doc_id,
        title: Some("".to_string()),
        doc_type: None,
        year: None,
        source: None,
        url: None,
        tags: None,
        notes: None,
    };

    let result = use_case.execute(input);

    assert!(result.is_err());
}

#[test]
fn test_update_invalid_tags() {
    let repo = MockRepository::new();
    let doc_id = repo.add_test_doc(create_test_document());

    let use_case = UpdateDocumentUseCase::new(repo);
    let input = UpdateDocumentInput {
        id: doc_id,
        title: None,
        doc_type: None,
        year: None,
        source: None,
        url: None,
        tags: Some(vec!["Invalid Tag".to_string()]), // Not kebab-case
        notes: None,
    };

    let result = use_case.execute(input);

    assert!(result.is_err());
}
