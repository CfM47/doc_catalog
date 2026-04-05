#![allow(dead_code)]

use crate::application::dto::{DocumentSummaryOutput, SearchDocumentsInput};
use crate::application::repositories::DocumentRepository;

pub struct SearchDocumentsUseCase<R: DocumentRepository> {
    repository: R,
}

impl<R: DocumentRepository> SearchDocumentsUseCase<R> {
    pub fn new(repository: R) -> Self {
        Self { repository }
    }

    pub fn execute(
        &self,
        input: SearchDocumentsInput,
    ) -> Result<Vec<DocumentSummaryOutput>, anyhow::Error> {
        let mut documents = self.repository.search(&input.query)?;
        documents.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
        let results = documents
            .into_iter()
            .map(DocumentSummaryOutput::from)
            .collect();
        Ok(results)
    }
}
