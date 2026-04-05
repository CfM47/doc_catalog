use crate::application::dto::SearchDocumentsInput;
use crate::application::tests::utils::MockRepository;
use crate::application::use_cases::SearchDocumentsUseCase;
use crate::domain::entities::{Document, DocumentType, NotesMetadata};

fn create_doc_with_title(title: &str) -> Document {
    Document::new(
        0,
        title.to_string(),
        DocumentType::Notes(NotesMetadata {}),
        None,
        None,
        None,
        vec![],
        None,
        "2024-01-15T10:00:00Z".to_string(),
        "2024-01-15T10:00:00Z".to_string(),
    )
    .unwrap()
}

#[test]
fn test_search_by_title() {
    let repo = MockRepository::new();
    repo.add_document(create_doc_with_title("Rust Programming Guide"));
    repo.add_document(create_doc_with_title("Python Basics"));

    let use_case = SearchDocumentsUseCase::new(repo);
    let input = SearchDocumentsInput {
        query: "rust".to_string(),
    };

    let result = use_case.execute(input);

    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 1);
}

#[test]
fn test_search_multiple_results() {
    let repo = MockRepository::new();
    repo.add_document(create_doc_with_title("Rust Programming"));
    repo.add_document(create_doc_with_title("Rust in Action"));
    repo.add_document(create_doc_with_title("Python Guide"));

    let use_case = SearchDocumentsUseCase::new(repo);
    let input = SearchDocumentsInput {
        query: "rust".to_string(),
    };

    let result = use_case.execute(input);

    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 2);
}

#[test]
fn test_search_no_matches() {
    let repo = MockRepository::new();
    repo.add_document(create_doc_with_title("Python Guide"));
    repo.add_document(create_doc_with_title("Java Basics"));

    let use_case = SearchDocumentsUseCase::new(repo);
    let input = SearchDocumentsInput {
        query: "rust".to_string(),
    };

    let result = use_case.execute(input);

    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 0);
}

#[test]
fn test_search_case_insensitive() {
    let repo = MockRepository::new();
    repo.add_document(create_doc_with_title("RUST Programming"));
    repo.add_document(create_doc_with_title("Python Guide"));

    let use_case = SearchDocumentsUseCase::new(repo);
    let input = SearchDocumentsInput {
        query: "rust".to_string(),
    };

    let result = use_case.execute(input);

    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 1);
}
