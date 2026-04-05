use dialoguer::Input;

use crate::application::dto::SearchDocumentsInput;
use crate::application::repositories::DocumentRepository;
use crate::cli::dependencies::CliDependencies;
use crate::cli::utils::truncate;

pub fn run<R: DocumentRepository + Clone>(deps: CliDependencies<R>) -> anyhow::Result<()> {
    let query: String = Input::new().with_prompt("Search query").interact()?;

    let input = SearchDocumentsInput { query };
    let results = deps.search_documents.execute(input)?;

    if results.is_empty() {
        println!("No matching documents found.");
        return Ok(());
    }

    println!("{:<4} {:<30} {:<10}", "ID", "Title", "Type");
    println!("{:-<4} {:-^30} {:-<10}", "", "", "");

    for doc in results {
        println!(
            "{:<4} {:<30} {:<10}",
            doc.id,
            truncate(&doc.title, 30),
            doc.doc_type
        );
    }

    Ok(())
}
