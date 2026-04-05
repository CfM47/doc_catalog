use dialoguer::Input;

use crate::application::dto::SearchDocumentsInput;
use crate::application::repositories::DocumentRepository;
use crate::cli::dependencies::CliDependencies;

pub fn run<R: DocumentRepository + Clone>(deps: CliDependencies<R>) -> anyhow::Result<()> {
    let query: String = Input::new().with_prompt("Search query").interact()?;

    let input = SearchDocumentsInput { query };
    let results = deps.search_documents.execute(input)?;

    if results.is_empty() {
        deps.printer.print_no_matches();
        return Ok(());
    }

    deps.printer.print_search_header();

    for doc in results {
        deps.printer
            .print_search_row(doc.id, &doc.title, &doc.doc_type);
    }

    Ok(())
}
