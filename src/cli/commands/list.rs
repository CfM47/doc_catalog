use crate::application::dto::ListDocumentsInput;
use crate::application::repositories::DocumentRepository;
use crate::cli::dependencies::CliDependencies;
use crate::cli::utils::truncate;

pub fn run<R: DocumentRepository + Clone>(deps: CliDependencies<R>) -> anyhow::Result<()> {
    let input = ListDocumentsInput {
        doc_types: None,
        tags: None,
        authors: None,
    };

    let documents = deps.list_documents.execute(input)?;

    if documents.is_empty() {
        println!("No documents found.");
        return Ok(());
    }

    println!("{:<50} {:<10} {:<50}", "Title", "Type", "Tags");
    println!("{:-<50} {:-^10} {:-<50}", "", "", "");

    for doc in documents {
        let tags_str = if doc.tags.is_empty() {
            "-".to_string()
        } else {
            doc.tags.join(", ")
        };
        println!(
            "{:<50} {:<10} {:<50}",
            truncate(&doc.title, 50),
            doc.doc_type,
            truncate(&tags_str, 50)
        );
    }

    Ok(())
}
