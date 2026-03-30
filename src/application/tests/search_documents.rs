use std::collections::HashMap;
use std::sync::Mutex;

use crate::application::dto::SearchDocumentsInput;
use crate::application::repositories::DocumentRepository;
use crate::application::use_cases::SearchDocumentsUseCase;
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
        _filter: crate::application::dto::ListDocumentsFilter,
    ) -> Result<Vec<Document>, anyhow::Error> {
        unimplemented!()
    }

    fn search(&self, query: &str) -> Result<Vec<Document>, anyhow::Error> {
        let query_lower = query.to_lowercase();

        let check_metadata = |doc: &Document| -> bool {
            match &doc.doc_type {
                DocumentType::Book(m) => {
                    if let Some(authors) = &m.authors {
                        if authors
                            .iter()
                            .any(|a| a.to_lowercase().contains(&query_lower))
                        {
                            return true;
                        }
                    }
                    if let Some(publisher) = &m.publisher {
                        if publisher.to_lowercase().contains(&query_lower) {
                            return true;
                        }
                    }
                    if let Some(isbn) = &m.isbn {
                        if isbn.to_lowercase().contains(&query_lower) {
                            return true;
                        }
                    }
                    false
                }
                DocumentType::Paper(m) => {
                    if let Some(authors) = &m.authors {
                        if authors
                            .iter()
                            .any(|a| a.to_lowercase().contains(&query_lower))
                        {
                            return true;
                        }
                    }
                    if let Some(journal) = &m.journal {
                        if journal.to_lowercase().contains(&query_lower) {
                            return true;
                        }
                    }
                    if let Some(doi) = &m.doi {
                        if doi.to_lowercase().contains(&query_lower) {
                            return true;
                        }
                    }
                    false
                }
                DocumentType::Lecture(m) => {
                    if let Some(event) = &m.event {
                        if event.to_lowercase().contains(&query_lower) {
                            return true;
                        }
                    }
                    if let Some(institution) = &m.institution {
                        if institution.to_lowercase().contains(&query_lower) {
                            return true;
                        }
                    }
                    if let Some(topic) = &m.topic {
                        if topic.to_lowercase().contains(&query_lower) {
                            return true;
                        }
                    }
                    false
                }
                DocumentType::Notes(_) => false,
            }
        };

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
                    || check_metadata(doc)
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

fn create_doc_with_tags(tags: Vec<String>) -> Document {
    Document::new(
        0,
        "Test Document".to_string(),
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

fn create_doc_with_notes(notes: &str) -> Document {
    Document::new(
        0,
        "Test Document".to_string(),
        DocumentType::Notes(NotesMetadata {}),
        None,
        None,
        None,
        vec![],
        Some(notes.to_string()),
        "2024-01-15T10:00:00Z".to_string(),
        "2024-01-15T10:00:00Z".to_string(),
    )
    .unwrap()
}

fn create_doc_with_metadata() -> Document {
    Document::new(
        0,
        "Rust Book".to_string(),
        DocumentType::Book(BookMetadata {
            authors: Some(vec!["John Doe".to_string()]),
            edition: None,
            publisher: Some("O'Reilly".to_string()),
            isbn: None,
        }),
        Some(2024),
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
    repo.add_test_doc(create_doc_with_title("Rust Programming Guide"));
    repo.add_test_doc(create_doc_with_title("Python Basics"));

    let use_case = SearchDocumentsUseCase::new(repo);
    let input = SearchDocumentsInput {
        query: "rust".to_string(),
    };

    let result = use_case.execute(input);

    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 1);
}

#[test]
fn test_search_by_tags() {
    let repo = MockRepository::new();
    repo.add_test_doc(create_doc_with_tags(vec![
        "rust".to_string(),
        "programming".to_string(),
    ]));
    repo.add_test_doc(create_doc_with_tags(vec!["python".to_string()]));

    let use_case = SearchDocumentsUseCase::new(repo);
    let input = SearchDocumentsInput {
        query: "rust".to_string(),
    };

    let result = use_case.execute(input);

    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 1);
}

#[test]
fn test_search_by_notes() {
    let repo = MockRepository::new();
    repo.add_test_doc(create_doc_with_notes(
        "Important notes about Rust programming",
    ));
    repo.add_test_doc(create_doc_with_notes("Python learning materials"));

    let use_case = SearchDocumentsUseCase::new(repo);
    let input = SearchDocumentsInput {
        query: "rust".to_string(),
    };

    let result = use_case.execute(input);

    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 1);
}

#[test]
fn test_search_by_metadata() {
    let repo = MockRepository::new();
    repo.add_test_doc(create_doc_with_metadata());
    repo.add_test_doc(create_doc_with_title("Other Book"));

    let use_case = SearchDocumentsUseCase::new(repo);
    let input = SearchDocumentsInput {
        query: "john".to_string(),
    };

    let result = use_case.execute(input);

    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 1);
}

#[test]
fn test_search_multiple_results() {
    let repo = MockRepository::new();
    repo.add_test_doc(create_doc_with_title("Rust Programming"));
    repo.add_test_doc(create_doc_with_title("Rust in Action"));
    repo.add_test_doc(create_doc_with_title("Python Guide"));

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
    repo.add_test_doc(create_doc_with_title("Python Guide"));
    repo.add_test_doc(create_doc_with_title("Java Basics"));

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
    repo.add_test_doc(create_doc_with_title("RUST Programming"));
    repo.add_test_doc(create_doc_with_title("Python Guide"));

    let use_case = SearchDocumentsUseCase::new(repo);
    let input = SearchDocumentsInput {
        query: "rust".to_string(),
    };

    let result = use_case.execute(input);

    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 1);
}
