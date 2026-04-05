use crate::application::dto::UpdateDocumentInput;
use crate::application::tests::utils::MockRepository;
use crate::application::use_cases::UpdateDocumentUseCase;

#[test]
fn test_update_title() {
    let repo = MockRepository::new();
    let doc_id = repo.add_test_doc("Original Title");

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
    let doc_id = repo.add_test_doc("Original Title");

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
    let doc_id = repo.add_test_doc("Original Title");

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
fn test_preserve_unmodified_fields() {
    let repo = MockRepository::new();
    let doc_id = repo.add_test_doc("Original Title");

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

    assert_eq!(updated.year, Some(2023));
    assert_eq!(updated.source, Some("Original Source".to_string()));
    assert_eq!(updated.url, Some("https://original.com".to_string()));
    assert_eq!(updated.tags, vec!["original-tag"]);
    assert_eq!(updated.notes, Some("Original notes".to_string()));
}

#[test]
fn test_update_nonexistent_document() {
    let repo = MockRepository::new();
    repo.add_test_doc("Original Title");

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
    let doc_id = repo.add_test_doc("Original Title");

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
    let doc_id = repo.add_test_doc("Original Title");

    let use_case = UpdateDocumentUseCase::new(repo);
    let input = UpdateDocumentInput {
        id: doc_id,
        title: None,
        doc_type: None,
        year: None,
        source: None,
        url: None,
        tags: Some(vec!["Invalid Tag".to_string()]),
        notes: None,
    };

    let result = use_case.execute(input);

    assert!(result.is_err());
}
