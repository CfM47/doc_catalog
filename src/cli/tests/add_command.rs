#![allow(dead_code)]

use crate::application::tests::utils::MockRepository;
use crate::cli::dependencies::CliDependencies;
use crate::cli::printer::CliPrinter;
use crate::domain::entities::{BookMetadata, DocumentType};

#[cfg(test)]
mod tests {
    use super::*;

    fn create_deps() -> CliDependencies<MockRepository> {
        CliDependencies::new(MockRepository::new(), CliPrinter::with_default_config())
    }

    #[test]
    fn test_cli_dependencies_have_create_use_case() {
        let deps = create_deps();
        let _ = deps.create_document;
    }

    #[test]
    fn test_cli_dependencies_have_list_use_case() {
        let deps = create_deps();
        let _ = deps.list_documents;
    }

    #[test]
    fn test_cli_dependencies_have_search_use_case() {
        let deps = create_deps();
        let _ = deps.search_documents;
    }

    #[test]
    fn test_cli_dependencies_have_update_use_case() {
        let deps = create_deps();
        let _ = deps.update_document;
    }

    #[test]
    fn test_cli_dependencies_have_delete_use_case() {
        let deps = create_deps();
        let _ = deps.delete_document;
    }

    #[test]
    fn test_document_type_book_metadata_keys() {
        let doc_type = DocumentType::Book(BookMetadata::default());
        let keys = doc_type.metadata_keys();
        assert!(keys.contains(&"authors"));
        assert!(keys.contains(&"edition"));
        assert!(keys.contains(&"publisher"));
        assert!(keys.contains(&"isbn"));
    }

    #[test]
    fn test_document_type_notes_has_no_metadata() {
        let doc_type = DocumentType::Notes(crate::domain::entities::NotesMetadata::default());
        let keys = doc_type.metadata_keys();
        assert!(keys.is_empty());
    }
}
