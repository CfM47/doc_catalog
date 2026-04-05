use crate::application::dto::ListDocumentsInput;
use crate::application::repositories::DocumentRepository;
use crate::cli::dependencies::CliDependencies;

pub fn run<R: DocumentRepository + Clone>(deps: CliDependencies<R>) -> anyhow::Result<()> {
    let input = ListDocumentsInput {
        doc_types: None,
        tags: None,
        authors: None,
    };

    let documents = deps.list_documents.execute(input)?;

    if documents.is_empty() {
        deps.printer.print_no_documents();
        return Ok(());
    }

    deps.printer.print_list_header();

    for doc in documents {
        deps.printer
            .print_list_row(&doc.title, &doc.doc_type, &doc.tags);
    }

    Ok(())
}
