#![allow(dead_code)]

use crate::application::dto::{CreateDocumentInput, CreateDocumentOutput};
use crate::application::repositories::DocumentRepository;
use crate::application::utils::now_iso8601;
use crate::domain::entities::Document;

pub struct CreateDocumentUseCase<R: DocumentRepository> {
    repository: R,
}

impl<R: DocumentRepository> CreateDocumentUseCase<R> {
    pub fn new(repository: R) -> Self {
        Self { repository }
    }

    pub fn execute(
        &self,
        input: CreateDocumentInput,
    ) -> Result<CreateDocumentOutput, anyhow::Error> {
        let timestamp = now_iso8601();

        let document = Document::new(
            0,
            input.title,
            input.doc_type,
            input.year,
            input.source,
            input.url,
            input.tags,
            input.notes,
            timestamp.clone(),
            timestamp,
        )?;

        let created = self.repository.create(document)?;

        Ok(created.into())
    }
}
