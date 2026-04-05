#![allow(dead_code)]

use crate::application::dto::ListDocumentsInput;
use crate::application::tests::utils::MockRepository;
use crate::cli::dependencies::CliDependencies;
use crate::domain::entities::{BookMetadata, Document, DocumentType};

#[cfg(test)]
mod tests {
    use super::*;

    fn create_deps_with_docs(docs: Vec<Document>) -> CliDependencies<MockRepository> {
        CliDependencies::new(MockRepository::with_documents(docs))
    }

    #[test]
    fn test_list_documents_returns_documents() {
        let docs = vec![Document {
            id: 1,
            title: "Test Book".to_string(),
            doc_type: DocumentType::Book(BookMetadata::default()),
            year: Some(2024),
            source: None,
            url: None,
            tags: vec!["rust".to_string()],
            notes: None,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        }];

        let deps = create_deps_with_docs(docs);

        let input = ListDocumentsInput {
            doc_types: None,
            tags: None,
            authors: None,
        };

        let result = deps.list_documents.execute(input);
        assert!(result.is_ok());
        let docs = result.unwrap();
        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0].title, "Test Book");
        assert_eq!(docs[0].doc_type, "book");
        assert_eq!(docs[0].tags, vec!["rust"]);
    }

    #[test]
    fn test_list_empty_returns_empty() {
        let deps = create_deps_with_docs(vec![]);

        let input = ListDocumentsInput {
            doc_types: None,
            tags: None,
            authors: None,
        };

        let result = deps.list_documents.execute(input);
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }
}
