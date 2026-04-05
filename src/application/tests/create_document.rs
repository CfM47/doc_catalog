use crate::application::dto::CreateDocumentInput;
use crate::application::repositories::DocumentRepository;
use crate::application::tests::utils::MockRepository;
use crate::application::use_cases::CreateDocumentUseCase;
use crate::domain::entities::{DocumentType, NotesMetadata};

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
