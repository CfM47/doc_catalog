#![allow(dead_code)]

use crate::application::dto::SearchDocumentsInput;
use crate::application::tests::utils::MockRepository;
use crate::cli::dependencies::CliDependencies;
use crate::cli::printer::CliPrinter;
use crate::domain::entities::{BookMetadata, Document, DocumentType};

#[cfg(test)]
mod tests {
    use super::*;

    fn create_deps_with_docs(docs: Vec<Document>) -> CliDependencies<MockRepository> {
        CliDependencies::new(
            MockRepository::with_documents(docs),
            CliPrinter::with_default_config(),
        )
    }

    #[test]
    fn test_search_returns_matching_documents() {
        let docs = vec![Document {
            id: 1,
            title: "Rust Book".to_string(),
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

        let input = SearchDocumentsInput {
            query: "rust".to_string(),
        };

        let result = deps.search_documents.execute(input);
        assert!(result.is_ok());
        let docs = result.unwrap();
        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0].title, "Rust Book");
        assert_eq!(docs[0].doc_type, "book");
    }

    #[test]
    fn test_search_empty_returns_empty() {
        let deps = create_deps_with_docs(vec![]);

        let input = SearchDocumentsInput {
            query: "rust".to_string(),
        };

        let result = deps.search_documents.execute(input);
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_search_sorts_by_title() {
        let docs = vec![
            Document {
                id: 1,
                title: "Zebra Book".to_string(),
                doc_type: DocumentType::Book(BookMetadata::default()),
                year: None,
                source: None,
                url: None,
                tags: vec![],
                notes: None,
                created_at: "".to_string(),
                updated_at: "".to_string(),
            },
            Document {
                id: 2,
                title: "Alpha Book".to_string(),
                doc_type: DocumentType::Book(BookMetadata::default()),
                year: None,
                source: None,
                url: None,
                tags: vec![],
                notes: None,
                created_at: "".to_string(),
                updated_at: "".to_string(),
            },
        ];

        let deps = create_deps_with_docs(docs);

        let input = SearchDocumentsInput {
            query: "book".to_string(),
        };

        let result = deps.search_documents.execute(input).unwrap();
        assert_eq!(result[0].title, "Alpha Book");
        assert_eq!(result[1].title, "Zebra Book");
    }
}
