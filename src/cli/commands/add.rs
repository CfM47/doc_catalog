use dialoguer::{Confirm, Input, Select};

use crate::application::dto::CreateDocumentInput;
use crate::application::repositories::DocumentRepository;
use crate::cli::dependencies::CliDependencies;
use crate::domain::entities::{
    BookMetadata, DocumentType, LectureMetadata, NotesMetadata, PaperMetadata,
};
use crate::domain::utils::validate_tags;

pub fn run<R: DocumentRepository + Clone>(deps: CliDependencies<R>) -> anyhow::Result<()> {
    let doc_type = prompt_doc_type()?;
    let metadata = prompt_metadata(&doc_type)?;
    let title = prompt_title()?;
    let year = prompt_year()?;
    let source = prompt_source()?;
    let url = prompt_url()?;
    let tags = prompt_tags()?;
    let notes = prompt_notes()?;

    println!();
    println!("┌──────────────────────────────────────────┐");
    println!("│  Add Document                            │");
    println!("├──────────────────────────────────────────┤"); 
    println!("│  Type:    {:30} │", doc_type.as_str());
    println!("│  Title:   {:30} │", truncate(&title, 30));
    if let Some(authors) = metadata.authors() {
        println!("│  Authors: {:30} │", truncate(&authors.join(", "), 30));
    }
    if let Some(y) = year {
        println!("│  Year:    {:30} │", y);
    } else {
        println!("│  Year:    {:30} │", "-");
    }
    if let Some(ref s) = source {
        println!("│  Source:  {:30} │", truncate(s, 30));
    } else {
        println!("│  Source:  {:30} │", "-");
    }
    if let Some(ref u) = url {
        println!("│  URL:     {:30} │", truncate(u, 30));
    } else {
        println!("│  URL:     {:30} │", "-");
    }
    if tags.is_empty() {
        println!("│  Tags:    {:30} │", "-");
    } else {
        println!("│  Tags:    {:30} │", truncate(&tags.join(", "), 30));
    }
    if let Some(ref n) = notes {
        println!("│  Notes:   {:30} │", truncate(n, 30));
    } else {
        println!("│  Notes:   {:30} │", "-");
    }
    println!("└──────────────────────────────────────────┘");
    println!();

    if !Confirm::with_theme(&dialoguer::theme::ColorfulTheme::default())
        .with_prompt("Save this document?")
        .default(true)
        .interact()?
    {
        println!("Aborted.");
        return Ok(());
    }

    let input = CreateDocumentInput {
        title,
        doc_type,
        year,
        source,
        url,
        tags,
        notes,
    };

    let result = deps.create_document.execute(input)?;
    println!("Document created with ID: {}", result.id);

    Ok(())
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

fn prompt_doc_type() -> anyhow::Result<DocumentType> {
    let choices = ["book", "paper", "lecture", "notes"];
    let selection = Select::with_theme(&dialoguer::theme::ColorfulTheme::default())
        .with_prompt("Document type")
        .default(0)
        .items(&choices)
        .interact()?;

    match selection {
        0 => Ok(DocumentType::Book(BookMetadata::default())),
        1 => Ok(DocumentType::Paper(PaperMetadata::default())),
        2 => Ok(DocumentType::Lecture(LectureMetadata::default())),
        3 => Ok(DocumentType::Notes(NotesMetadata::default())),
        _ => unreachable!(),
    }
}

fn prompt_metadata(doc_type: &DocumentType) -> anyhow::Result<DocumentType> {
    match doc_type {
        DocumentType::Book(_) => {
            let authors = prompt_authors()?;
            let edition: Option<String> = Input::new()
                .with_prompt("Edition (optional)")
                .allow_empty(true)
                .interact()
                .ok();
            let publisher: Option<String> = Input::new()
                .with_prompt("Publisher (optional)")
                .allow_empty(true)
                .interact()
                .ok();
            let isbn: Option<String> = Input::new()
                .with_prompt("ISBN (optional)")
                .allow_empty(true)
                .interact()
                .ok();
            Ok(DocumentType::Book(BookMetadata {
                authors,
                edition,
                publisher,
                isbn,
            }))
        }
        DocumentType::Paper(_) => {
            let authors = prompt_authors()?;
            let journal: Option<String> = Input::new()
                .with_prompt("Journal (optional)")
                .allow_empty(true)
                .interact()
                .ok();
            let volume: Option<String> = Input::new()
                .with_prompt("Volume (optional)")
                .allow_empty(true)
                .interact()
                .ok();
            let issue: Option<String> = Input::new()
                .with_prompt("Issue (optional)")
                .allow_empty(true)
                .interact()
                .ok();
            let doi: Option<String> = Input::new()
                .with_prompt("DOI (optional)")
                .allow_empty(true)
                .interact()
                .ok();
            Ok(DocumentType::Paper(PaperMetadata {
                authors,
                journal,
                volume,
                issue,
                doi,
            }))
        }
        DocumentType::Lecture(_) => {
            let event: Option<String> = Input::new()
                .with_prompt("Event (optional)")
                .allow_empty(true)
                .interact()
                .ok();
            let institution: Option<String> = Input::new()
                .with_prompt("Institution (optional)")
                .allow_empty(true)
                .interact()
                .ok();
            let location: Option<String> = Input::new()
                .with_prompt("Location (optional)")
                .allow_empty(true)
                .interact()
                .ok();
            let topic: Option<String> = Input::new()
                .with_prompt("Topic (optional)")
                .allow_empty(true)
                .interact()
                .ok();
            Ok(DocumentType::Lecture(LectureMetadata {
                event,
                institution,
                location,
                topic,
            }))
        }
        DocumentType::Notes(_) => Ok(DocumentType::Notes(NotesMetadata::default())),
    }
}

fn prompt_authors() -> anyhow::Result<Option<Vec<String>>> {
    let input: String = Input::new()
        .with_prompt("Authors (comma-separated, optional)")
        .allow_empty(true)
        .interact()?;
    if input.trim().is_empty() {
        Ok(None)
    } else {
        let authors: Vec<String> = input
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        Ok(if authors.is_empty() {
            None
        } else {
            Some(authors)
        })
    }
}

fn prompt_title() -> anyhow::Result<String> {
    loop {
        let title: String = Input::new().with_prompt("Title").interact()?;
        if !title.trim().is_empty() {
            return Ok(title);
        }
        println!("Title cannot be empty. Please try again.");
    }
}

fn prompt_year() -> anyhow::Result<Option<i32>> {
    loop {
        let input: String = Input::new()
            .with_prompt("Year (optional)")
            .allow_empty(true)
            .interact()?;
        if input.trim().is_empty() {
            return Ok(None);
        }
        match input.trim().parse::<i32>() {
            Ok(year) if (1000..=2100).contains(&year) => return Ok(Some(year)),
            _ => println!("Invalid year. Please enter a year between 1000 and 2100."),
        }
    }
}

fn prompt_source() -> anyhow::Result<Option<String>> {
    let input: String = Input::new()
        .with_prompt("Source (optional, e.g., Amazon, library)")
        .allow_empty(true)
        .interact()?;
    Ok(if input.trim().is_empty() {
        None
    } else {
        Some(input)
    })
}

fn prompt_url() -> anyhow::Result<Option<String>> {
    let input: String = Input::new()
        .with_prompt("Remote URL (optional)")
        .allow_empty(true)
        .interact()?;
    Ok(if input.trim().is_empty() {
        None
    } else {
        Some(input)
    })
}

fn prompt_tags() -> anyhow::Result<Vec<String>> {
    loop {
        let input: String = Input::new()
            .with_prompt("Tags (comma-separated, optional)")
            .allow_empty(true)
            .interact()?;
        if input.trim().is_empty() {
            return Ok(Vec::new());
        }
        let tags: Vec<String> = input
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        if let Err(e) = validate_tags(&tags) {
            println!(
                "Invalid tags: {}. Please use kebab-case (e.g., rust, programming-lang).",
                e
            );
            continue;
        }
        return Ok(tags);
    }
}

fn prompt_notes() -> anyhow::Result<Option<String>> {
    let input: String = Input::new()
        .with_prompt("Notes (optional)")
        .allow_empty(true)
        .interact()?;
    Ok(if input.trim().is_empty() {
        None
    } else {
        Some(input)
    })
}
