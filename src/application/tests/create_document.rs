use std::collections::HashMap;
use std::sync::Mutex;

use crate::application::dto::CreateDocumentInput;
use crate::application::repositories::DocumentRepository;
use crate::application::use_cases::CreateDocumentUseCase;
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
}

impl DocumentRepository for MockRepository {
    fn create(&self, mut document: Document) -> Result<Document, anyhow::Error> {
        let id = *self.next_id.lock().unwrap();
        *self.next_id.lock().unwrap() += 1;
        document.id = id;
        self.documents.lock().unwrap().insert(id, document.clone());
        Ok(document)
    }

    fn find_by_id(&self, _id: i64) -> Result<Document, anyhow::Error> {
        unimplemented!()
    }

    fn find_all(&self) -> Result<Vec<Document>, anyhow::Error> {
        unimplemented!()
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

    fn update(&self, _document: Document) -> Result<Document, anyhow::Error> {
        unimplemented!()
    }

    fn delete(&self, _id: i64) -> Result<(), anyhow::Error> {
        unimplemented!()
    }
}

#[test]
fn test_create_document_success() {
    let repo = MockRepository::new();
    let use_case = CreateDocumentUseCase::new(repo);

    let input = CreateDocumentInput {
        title: "Test Document".to_string(),
        doc_type: DocumentType::Notes(NotesMetadata {}),
        year: Some(2024),
        source: Some("Test Source".to_string()),
        url: Some("https://example.com".to_string()),
        tags: vec!["test".to_string()],
        notes: Some("Test notes".to_string()),
    };

    let result = use_case.execute(input);

    assert!(result.is_ok());
    let output = result.unwrap();
    assert_eq!(output.title, "Test Document");
    assert!(output.id > 0);
    assert!(!output.created_at.is_empty());
}

#[test]
fn test_create_document_empty_title() {
    let repo = MockRepository::new();
    let use_case = CreateDocumentUseCase::new(repo);

    let input = CreateDocumentInput {
        title: "".to_string(),
        doc_type: DocumentType::Notes(NotesMetadata {}),
        year: None,
        source: None,
        url: None,
        tags: vec![],
        notes: None,
    };

    let result = use_case.execute(input);

    assert!(result.is_err());
}

#[test]
fn test_create_document_invalid_tags() {
    let repo = MockRepository::new();
    let use_case = CreateDocumentUseCase::new(repo);

    let input = CreateDocumentInput {
        title: "Test".to_string(),
        doc_type: DocumentType::Notes(NotesMetadata {}),
        year: None,
        source: None,
        url: None,
        tags: vec!["Invalid Tag".to_string()], // Not kebab-case
        notes: None,
    };

    let result = use_case.execute(input);

    assert!(result.is_err());
}

#[test]
fn test_create_document_with_book_metadata() {
    let repo = MockRepository::new();
    let use_case = CreateDocumentUseCase::new(repo);

    let input = CreateDocumentInput {
        title: "Rust Programming".to_string(),
        doc_type: DocumentType::Book(crate::domain::entities::BookMetadata {
            authors: Some(vec!["John Doe".to_string()]),
            edition: Some("2nd".to_string()),
            publisher: Some("O'Reilly".to_string()),
            isbn: Some("978-1491950357".to_string()),
        }),
        year: Some(2024),
        source: None,
        url: None,
        tags: vec!["rust".to_string(), "programming".to_string()],
        notes: None,
    };

    let result = use_case.execute(input);

    assert!(result.is_ok());
    let output = result.unwrap();
    assert_eq!(output.doc_type, "book");
    assert_eq!(output.tags, vec!["rust", "programming"]);
}
