use crate::application::repositories::DocumentRepository;
use crate::domain::entities::{BookMetadata, Document, DocumentType, NotesMetadata};
use crate::infrastructure::database::{Database, initialize};
use crate::infrastructure::repositories::SqliteDocumentRepository;

fn create_test_repo() -> SqliteDocumentRepository {
    let db = Database::in_memory().unwrap();
    initialize(&db.conn).unwrap();
    SqliteDocumentRepository::new(db.conn)
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
    let repo = create_test_repo();
    let doc = create_test_document("Test Document");

    let result = repo.create(doc);

    assert!(result.is_ok());
    assert!(result.unwrap().id > 0);
}

#[test]
fn test_find_by_id() {
    let repo = create_test_repo();
    let doc = create_test_document("Test Document");
    let created = repo.create(doc).unwrap();

    let found = repo.find_by_id(created.id);

    assert!(found.is_ok());
    assert_eq!(found.unwrap().title, "Test Document");
}

#[test]
fn test_find_by_id_not_found() {
    let repo = create_test_repo();

    let result = repo.find_by_id(999);

    assert!(result.is_err());
}

#[test]
fn test_find_all() {
    let repo = create_test_repo();
    repo.create(create_test_document("Doc 1")).unwrap();
    repo.create(create_test_document("Doc 2")).unwrap();

    let all = repo.find_all();

    assert!(all.is_ok());
    assert_eq!(all.unwrap().len(), 2);
}

#[test]
fn test_update_document() {
    let repo = create_test_repo();
    let doc = create_test_document("Original");
    let mut created = repo.create(doc).unwrap();

    created.title = "Updated".to_string();
    let result = repo.update(created);

    assert!(result.is_ok());
    assert_eq!(result.unwrap().title, "Updated");
}

#[test]
fn test_update_not_found() {
    let repo = create_test_repo();
    let mut doc = create_test_document("Test");
    doc.id = 999;

    let result = repo.update(doc);

    assert!(result.is_err());
}

#[test]
fn test_delete_document() {
    let repo = create_test_repo();
    let doc = create_test_document("Test");
    let created = repo.create(doc).unwrap();

    let result = repo.delete(created.id);

    assert!(result.is_ok());
    assert!(repo.find_by_id(created.id).is_err());
}

#[test]
fn test_delete_not_found() {
    let repo = create_test_repo();

    let result = repo.delete(999);

    assert!(result.is_err());
}

#[test]
fn test_document_with_metadata() {
    let repo = create_test_repo();
    let doc = Document::new(
        0,
        "Rust Book".to_string(),
        DocumentType::Book(BookMetadata {
            authors: Some(vec!["John Doe".to_string()]),
            edition: Some("2nd".to_string()),
            publisher: Some("O'Reilly".to_string()),
            isbn: Some("978-1491950357".to_string()),
        }),
        Some(2024),
        None,
        None,
        vec!["rust".to_string()],
        None,
        "2024-01-15T10:00:00Z".to_string(),
        "2024-01-15T10:00:00Z".to_string(),
    )
    .unwrap();

    let created = repo.create(doc).unwrap();
    let found = repo.find_by_id(created.id).unwrap();

    match found.doc_type {
        DocumentType::Book(m) => {
            assert_eq!(m.authors, Some(vec!["John Doe".to_string()]));
            assert_eq!(m.edition, Some("2nd".to_string()));
            assert_eq!(m.publisher, Some("O'Reilly".to_string()));
            assert_eq!(m.isbn, Some("978-1491950357".to_string()));
        }
        _ => panic!("Expected Book document type"),
    }
}

#[test]
fn test_tags_stored_as_json() {
    let repo = create_test_repo();
    let doc = create_test_document("Test");
    let created = repo.create(doc).unwrap();

    let found = repo.find_by_id(created.id).unwrap();

    assert_eq!(found.tags, vec!["test"]);
}
