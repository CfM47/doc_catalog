use std::marker::PhantomData;

use crate::application::repositories::DocumentRepository;
use crate::application::use_cases::{
    CreateDocumentUseCase, DeleteDocumentUseCase, ListDocumentsUseCase, SearchDocumentsUseCase,
    UpdateDocumentUseCase,
};
use crate::cli::printer::CliPrinter;

pub struct CliDependencies<R: DocumentRepository + Clone> {
    pub create_document: CreateDocumentUseCase<R>,
    pub list_documents: ListDocumentsUseCase<R>,
    pub search_documents: SearchDocumentsUseCase<R>,
    pub update_document: UpdateDocumentUseCase<R>,
    pub delete_document: DeleteDocumentUseCase<R>,
    pub printer: CliPrinter,
    _marker: PhantomData<R>,
}

impl<R: DocumentRepository + Clone> CliDependencies<R> {
    pub fn new(repository: R, printer: CliPrinter) -> Self {
        Self {
            create_document: CreateDocumentUseCase::new(repository.clone()),
            list_documents: ListDocumentsUseCase::new(repository.clone()),
            search_documents: SearchDocumentsUseCase::new(repository.clone()),
            update_document: UpdateDocumentUseCase::new(repository.clone()),
            delete_document: DeleteDocumentUseCase::new(repository),
            printer,
            _marker: PhantomData,
        }
    }
}
