use crate::application::dto::DeleteDocumentInput;
use crate::application::tests::utils::MockRepository;
use crate::application::use_cases::DeleteDocumentUseCase;

#[test]
fn test_delete_existing() {
    let repo = MockRepository::new();
    let doc_id = repo.add_test_doc("Test Document");

    let use_case = DeleteDocumentUseCase::new(repo);
    let input = DeleteDocumentInput { id: doc_id };

    let result = use_case.execute(input);

    assert!(result.is_ok());
}

#[test]
fn test_delete_nonexistent() {
    let repo = MockRepository::new();
    repo.add_test_doc("Test Document");

    let use_case = DeleteDocumentUseCase::new(repo);
    let input = DeleteDocumentInput { id: 999 };

    let result = use_case.execute(input);

    assert!(result.is_err());
}
