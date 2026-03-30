#![allow(dead_code)]

use crate::application::dto::UpdateDocumentInput;
use crate::application::repositories::DocumentRepository;
use crate::application::utils::now_iso8601;
use crate::domain::entities::Document;

pub struct UpdateDocumentUseCase<R: DocumentRepository> {
    repository: R,
}

impl<R: DocumentRepository> UpdateDocumentUseCase<R> {
    pub fn new(repository: R) -> Self {
        Self { repository }
    }

    pub fn execute(&self, input: UpdateDocumentInput) -> Result<Document, anyhow::Error> {
        let mut document = self.repository.find_by_id(input.id)?;

        if let Some(title) = input.title {
            document.title = title;
        }

        if let Some(doc_type) = input.doc_type {
            document.doc_type = doc_type;
        }

        if let Some(year) = input.year {
            document.year = Some(year);
        }

        if let Some(source) = input.source {
            document.source = Some(source);
        }

        if let Some(url) = input.url {
            document.url = Some(url);
        }

        if let Some(tags) = input.tags {
            document.tags = tags;
        }

        if let Some(notes) = input.notes {
            document.notes = Some(notes);
        }

        document.updated_at = now_iso8601();

        document.validate()?;

        let updated = self.repository.update(document)?;

        Ok(updated)
    }
}
