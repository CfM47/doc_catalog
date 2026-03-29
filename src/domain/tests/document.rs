use crate::domain::entities::{BookMetadata, Document, DocumentType, NotesMetadata};
use crate::domain::errors::DomainError;

fn create_valid_doc() -> Document {
    Document::new(
        1,
        "Test Document".to_string(),
        DocumentType::Book(BookMetadata {
            authors: Some("John Doe".to_string()),
            edition: None,
            publisher: None,
            isbn: None,
        }),
        Some(2024),
        Some("Personal Library".to_string()),
        Some("https://example.com".to_string()),
        vec!["rust".to_string(), "programming".to_string()],
        None,
        "2024-01-15T10:00:00Z".to_string(),
        "2024-01-15T10:00:00Z".to_string(),
    )
    .unwrap()
}

#[test]
fn test_document_creation_valid() {
    let doc = create_valid_doc();
    assert_eq!(doc.title, "Test Document");
    assert_eq!(doc.id, 1);
}

#[test]
fn test_document_creation_empty_title() {
    let result = Document::new(
        1,
        "".to_string(),
        DocumentType::Notes(NotesMetadata {}),
        None,
        None,
        None,
        Vec::new(),
        None,
        "2024-01-15T10:00:00Z".to_string(),
        "2024-01-15T10:00:00Z".to_string(),
    );
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), DomainError::EmptyTitle);
}

#[test]
fn test_document_invalid_year_too_low() {
    let result = Document::new(
        1,
        "Title".to_string(),
        DocumentType::Notes(NotesMetadata {}),
        Some(999),
        None,
        None,
        Vec::new(),
        None,
        "2024-01-15T10:00:00Z".to_string(),
        "2024-01-15T10:00:00Z".to_string(),
    );
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), DomainError::InvalidYear(999));
}

#[test]
fn test_document_invalid_year_too_high() {
    let result = Document::new(
        1,
        "Title".to_string(),
        DocumentType::Notes(NotesMetadata {}),
        Some(3000),
        None,
        None,
        Vec::new(),
        None,
        "2024-01-15T10:00:00Z".to_string(),
        "2024-01-15T10:00:00Z".to_string(),
    );
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), DomainError::InvalidYear(3000));
}

#[test]
fn test_document_valid_year() {
    let result = Document::new(
        1,
        "Title".to_string(),
        DocumentType::Notes(NotesMetadata {}),
        Some(2024),
        None,
        None,
        Vec::new(),
        None,
        "2024-01-15T10:00:00Z".to_string(),
        "2024-01-15T10:00:00Z".to_string(),
    );
    assert!(result.is_ok());
}

#[test]
fn test_document_tags_max_limit() {
    let tags: Vec<String> = (0..51).map(|i| format!("tag{}", i)).collect();
    let result = Document::new(
        1,
        "Title".to_string(),
        DocumentType::Notes(NotesMetadata {}),
        None,
        None,
        None,
        tags,
        None,
        "2024-01-15T10:00:00Z".to_string(),
        "2024-01-15T10:00:00Z".to_string(),
    );
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), DomainError::TooManyTags(51));
}

#[test]
fn test_document_tag_too_long() {
    let tag = "a".repeat(31);
    let tags = vec![tag.clone(); 1];
    let result = Document::new(
        1,
        "Title".to_string(),
        DocumentType::Notes(NotesMetadata {}),
        None,
        None,
        None,
        tags,
        None,
        "2024-01-15T10:00:00Z".to_string(),
        "2024-01-15T10:00:00Z".to_string(),
    );
    assert!(result.is_err());
}

#[test]
fn test_metadata_key_validation_book() {
    let doc = create_valid_doc();
    assert!(doc.validate_metadata_key("authors"));
    assert!(doc.validate_metadata_key("isbn"));
    assert!(!doc.validate_metadata_key("journal"));
}

#[test]
fn test_metadata_key_validation_notes() {
    let doc = Document::new(
        1,
        "Notes".to_string(),
        DocumentType::Notes(NotesMetadata {}),
        None,
        None,
        None,
        Vec::new(),
        None,
        "2024-01-15T10:00:00Z".to_string(),
        "2024-01-15T10:00:00Z".to_string(),
    )
    .unwrap();
    assert!(!doc.validate_metadata_key("authors"));
}
