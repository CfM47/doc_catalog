use std::collections::HashMap;
use std::sync::Mutex;

use crate::application::dto::{ListDocumentsFilter, ListDocumentsInput};
use crate::application::repositories::DocumentRepository;
use crate::application::use_cases::ListDocumentsUseCase;
use crate::domain::entities::{BookMetadata, Document, DocumentType, NotesMetadata};

struct MockRepository {
    documents: Mutex<HashMap<i64, Document>>,
    next_id: Mutex<i64>,
}

impl MockRepository {
    fn new() -> Self {
        Self {
            documents: Mutex::new(HashMap::new()),
            next_id: Mutex::new(1),
        }
    }

    fn add_test_doc(&self, doc: Document) {
        let id = *self.next_id.lock().unwrap();
        let mut d = doc;
        d.id = id;
        self.documents.lock().unwrap().insert(id, d);
        *self.next_id.lock().unwrap() += 1;
    }
}

impl DocumentRepository for MockRepository {
    fn create(&self, mut document: Document) -> Result<Document, anyhow::Error> {
        let id = *self.next_id.lock().unwrap();
        *self.next_id.lock().unwrap() += 1;
        document.id = id;
        self.documents.lock().unwrap().insert(id, document.clone());
        Ok(document)
    }

    fn find_by_id(&self, _id: i64) -> Result<Document, anyhow::Error> {
        unimplemented!()
    }

    fn find_all(&self) -> Result<Vec<Document>, anyhow::Error> {
        Ok(self.documents.lock().unwrap().values().cloned().collect())
    }

    fn find_all_with_filter(
        &self,
        filter: ListDocumentsFilter,
    ) -> Result<Vec<Document>, anyhow::Error> {
        if filter.is_empty() {
            return self.find_all();
        }

        let mut docs: Vec<Document> = self.documents.lock().unwrap().values().cloned().collect();

        if let Some(doc_types) = &filter.doc_types {
            if !doc_types.is_empty() {
                docs.retain(|doc| doc_types.contains(&doc.doc_type.as_str().to_string()));
            }
        }

        if let Some(tags) = &filter.tags {
            if !tags.is_empty() {
                docs.retain(|doc| doc.tags.iter().any(|t| tags.contains(t)));
            }
        }

        if let Some(authors) = &filter.authors {
            if !authors.is_empty() {
                docs.retain(|doc| match &doc.doc_type {
                    DocumentType::Book(m) => {
                        if let Some(doc_authors) = &m.authors {
                            doc_authors.iter().any(|a| {
                                authors
                                    .iter()
                                    .any(|f| a.to_lowercase().contains(&f.to_lowercase()))
                            })
                        } else {
                            false
                        }
                    }
                    DocumentType::Paper(m) => {
                        if let Some(doc_authors) = &m.authors {
                            doc_authors.iter().any(|a| {
                                authors
                                    .iter()
                                    .any(|f| a.to_lowercase().contains(&f.to_lowercase()))
                            })
                        } else {
                            false
                        }
                    }
                    _ => false,
                });
            }
        }

        Ok(docs)
    }

    fn search(&self, query: &str) -> Result<Vec<Document>, anyhow::Error> {
        let query_lower = query.to_lowercase();
        let docs: Vec<Document> = self
            .documents
            .lock()
            .unwrap()
            .values()
            .filter(|doc| {
                doc.title.to_lowercase().contains(&query_lower)
                    || doc
                        .tags
                        .iter()
                        .any(|t| t.to_lowercase().contains(&query_lower))
                    || doc
                        .notes
                        .as_ref()
                        .map_or(false, |n| n.to_lowercase().contains(&query_lower))
            })
            .cloned()
            .collect();
        Ok(docs)
    }

    fn update(&self, _document: Document) -> Result<Document, anyhow::Error> {
        unimplemented!()
    }

    fn delete(&self, _id: i64) -> Result<(), anyhow::Error> {
        unimplemented!()
    }
}

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
    repo.add_test_doc(create_book(
        "Book 1",
        vec!["John".to_string()],
        vec!["rust".to_string()],
    ));
    repo.add_test_doc(create_notes("Notes 1", vec!["todo".to_string()]));

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
    repo.add_test_doc(create_book(
        "Book 1",
        vec!["John".to_string()],
        vec!["rust".to_string()],
    ));
    repo.add_test_doc(create_notes("Notes 1", vec!["todo".to_string()]));

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
    repo.add_test_doc(create_book(
        "Book 1",
        vec!["John".to_string()],
        vec!["rust".to_string()],
    ));
    repo.add_test_doc(create_notes("Notes 1", vec!["todo".to_string()]));

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
    repo.add_test_doc(create_book(
        "Book 1",
        vec!["John Doe".to_string()],
        vec!["rust".to_string()],
    ));
    repo.add_test_doc(create_book(
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
    repo.add_test_doc(create_book(
        "Book 1",
        vec!["John".to_string()],
        vec!["rust".to_string()],
    ));
    repo.add_test_doc(create_notes("Notes 1", vec!["todo".to_string()]));
    repo.add_test_doc(
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
    repo.add_test_doc(create_book(
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
