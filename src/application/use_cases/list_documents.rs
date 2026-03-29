use crate::application::dto::{DocumentSummaryOutput, ListDocumentsFilter, ListDocumentsInput};
use crate::application::repositories::DocumentRepository;

pub struct ListDocumentsUseCase<R: DocumentRepository> {
    repository: R,
}

impl<R: DocumentRepository> ListDocumentsUseCase<R> {
    pub fn new(repository: R) -> Self {
        Self { repository }
    }

    pub fn execute(
        &self,
        input: ListDocumentsInput,
    ) -> Result<Vec<DocumentSummaryOutput>, anyhow::Error> {
        let filter = ListDocumentsFilter::from(input);
        let documents = self.repository.find_all_with_filter(filter)?;
        let summaries = documents
            .into_iter()
            .map(DocumentSummaryOutput::from)
            .collect();
        Ok(summaries)
    }
}
