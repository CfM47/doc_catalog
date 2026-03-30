#![allow(dead_code)]

use crate::application::dto::DeleteDocumentInput;
use crate::application::repositories::DocumentRepository;

pub struct DeleteDocumentUseCase<R: DocumentRepository> {
    repository: R,
}

impl<R: DocumentRepository> DeleteDocumentUseCase<R> {
    pub fn new(repository: R) -> Self {
        Self { repository }
    }

    pub fn execute(&self, input: DeleteDocumentInput) -> Result<(), anyhow::Error> {
        self.repository.delete(input.id)?;
        Ok(())
    }
}
