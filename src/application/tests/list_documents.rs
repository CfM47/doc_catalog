use crate::application::dto::ListDocumentsInput;
use crate::application::tests::utils::MockRepository;
use crate::application::use_cases::ListDocumentsUseCase;
use crate::domain::entities::{BookMetadata, Document, DocumentType, NotesMetadata};

fn create_book(title: &str, authors: Vec<String>, tags: Vec<String>) -> Document {
    Document::new(
        0,
        title.to_string(),
        DocumentType::Book(BookMetadata {
            authors: Some(authors),
            edition: None,
            publisher: Some("O'Reilly".to_string()),
            isbn: None,
        }),
        Some(2024),
        None,
        None,
        tags,
        None,
        "2024-01-15T10:00:00Z".to_string(),
        "2024-01-15T10:00:00Z".to_string(),
    )
    .unwrap()
}

fn create_notes(title: &str, tags: Vec<String>) -> Document {
    Document::new(
        0,
        title.to_string(),
        DocumentType::Notes(NotesMetadata {}),
        None,
        None,
        None,
        tags,
        None,
        "2024-01-15T10:00:00Z".to_string(),
        "2024-01-15T10:00:00Z".to_string(),
    )
    .unwrap()
}

#[test]
fn test_list_all_no_filters() {
    let repo = MockRepository::new();
    repo.add_document(create_book(
        "Book 1",
        vec!["John".to_string()],
        vec!["rust".to_string()],
    ));
    repo.add_document(create_notes("Notes 1", vec!["todo".to_string()]));

    let use_case = ListDocumentsUseCase::new(repo);
    let input = ListDocumentsInput {
        doc_types: None,
        tags: None,
        authors: None,
    };

    let result = use_case.execute(input);

    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 2);
}

#[test]
fn test_filter_by_doc_types() {
    let repo = MockRepository::new();
    repo.add_document(create_book(
        "Book 1",
        vec!["John".to_string()],
        vec!["rust".to_string()],
    ));
    repo.add_document(create_notes("Notes 1", vec!["todo".to_string()]));

    let use_case = ListDocumentsUseCase::new(repo);
    let input = ListDocumentsInput {
        doc_types: Some(vec!["book".to_string()]),
        tags: None,
        authors: None,
    };

    let result = use_case.execute(input);

    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 1);
}

#[test]
fn test_filter_by_tags() {
    let repo = MockRepository::new();
    repo.add_document(create_book(
        "Book 1",
        vec!["John".to_string()],
        vec!["rust".to_string()],
    ));
    repo.add_document(create_notes("Notes 1", vec!["todo".to_string()]));

    let use_case = ListDocumentsUseCase::new(repo);
    let input = ListDocumentsInput {
        doc_types: None,
        tags: Some(vec!["rust".to_string()]),
        authors: None,
    };

    let result = use_case.execute(input);

    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 1);
}

#[test]
fn test_filter_by_authors() {
    let repo = MockRepository::new();
    repo.add_document(create_book(
        "Book 1",
        vec!["John Doe".to_string()],
        vec!["rust".to_string()],
    ));
    repo.add_document(create_book(
        "Book 2",
        vec!["Jane Smith".to_string()],
        vec!["python".to_string()],
    ));

    let use_case = ListDocumentsUseCase::new(repo);
    let input = ListDocumentsInput {
        doc_types: None,
        tags: None,
        authors: Some(vec!["John".to_string()]),
    };

    let result = use_case.execute(input);

    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 1);
}

#[test]
fn test_filter_multiple_doc_types() {
    let repo = MockRepository::new();
    repo.add_document(create_book(
        "Book 1",
        vec!["John".to_string()],
        vec!["rust".to_string()],
    ));
    repo.add_document(create_notes("Notes 1", vec!["todo".to_string()]));
    repo.add_document(
        Document::new(
            0,
            "Paper 1".to_string(),
            DocumentType::Paper(crate::domain::entities::PaperMetadata {
                authors: Some(vec!["Author".to_string()]),
                journal: None,
                volume: None,
                issue: None,
                doi: None,
            }),
            Some(2024),
            None,
            None,
            vec!["research".to_string()],
            None,
            "2024-01-15T10:00:00Z".to_string(),
            "2024-01-15T10:00:00Z".to_string(),
        )
        .unwrap(),
    );

    let use_case = ListDocumentsUseCase::new(repo);
    let input = ListDocumentsInput {
        doc_types: Some(vec!["book".to_string(), "paper".to_string()]),
        tags: None,
        authors: None,
    };

    let result = use_case.execute(input);

    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 2);
}

#[test]
fn test_metadata_included() {
    use crate::application::dto::DocumentMetadata;

    let repo = MockRepository::new();
    repo.add_document(create_book(
        "Book 1",
        vec!["John Doe".to_string(), "Jane Smith".to_string()],
        vec![],
    ));

    let use_case = ListDocumentsUseCase::new(repo);
    let input = ListDocumentsInput {
        doc_types: None,
        tags: None,
        authors: None,
    };

    let result = use_case.execute(input);

    assert!(result.is_ok());
    let docs = result.unwrap();
    assert_eq!(docs.len(), 1);

    match &docs[0].metadata {
        DocumentMetadata::Book { authors, .. } => {
            assert!(authors.is_some());
            assert!(authors.as_ref().unwrap().contains(&"John Doe".to_string()));
        }
        _ => panic!("Expected Book metadata"),
    }
}
