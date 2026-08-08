use anyhow::Result;
use inquire::{Confirm, Select, Text};
use rusqlite::Connection;
use std::fmt;

use crate::db;
use crate::metadata::Extracted;
use crate::model::{Document, Kind};

pub fn truncate(text: &str, width: usize) -> String {
    let count = text.chars().count();
    if count <= width {
        format!("{text:<width$}")
    } else if width <= 1 {
        text.chars().take(width).collect()
    } else {
        let kept: String = text.chars().take(width - 1).collect();
        format!("{kept}…")
    }
}

pub fn print_table(docs: &[Document]) {
    if docs.is_empty() {
        println!("no documents.");
        return;
    }
    println!(
        "{}  {}  {}  YEAR",
        truncate("TITLE", 44),
        truncate("AUTHORS", 26),
        truncate("KIND", 7)
    );
    for doc in docs {
        println!(
            "{}  {}  {}  {}",
            truncate(&doc.title, 44),
            truncate(doc.byline(), 26),
            truncate(doc.kind.as_str(), 7),
            doc.year.map(|y| y.to_string()).unwrap_or_default()
        );
    }
    println!("\n{} document(s)", docs.len());
}

/// Wrapper so `inquire` renders a document as one readable line.
struct Choice(Document);

impl fmt::Display for Choice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let doc = &self.0;
        write!(f, "{}", truncate(&doc.title, 50))?;
        write!(f, "  {}", truncate(doc.byline(), 24))?;
        if let Some(year) = doc.year {
            write!(f, "  {year}")?;
        }
        Ok(())
    }
}

pub fn pick(docs: Vec<Document>, prompt: &str) -> Result<Option<Document>> {
    if docs.is_empty() {
        println!("catalog is empty.");
        return Ok(None);
    }
    let choices: Vec<Choice> = docs.into_iter().map(Choice).collect();
    match Select::new(prompt, choices).with_page_size(15).prompt() {
        Ok(choice) => Ok(Some(choice.0)),
        Err(inquire::InquireError::OperationCanceled)
        | Err(inquire::InquireError::OperationInterrupted) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

pub fn ask(label: &str, default: Option<&str>) -> Result<String> {
    let mut prompt = Text::new(label);
    if let Some(default) = default {
        prompt = prompt.with_initial_value(default);
    }
    Ok(prompt.prompt()?.trim().to_string())
}

pub fn ask_opt(label: &str, default: Option<&str>) -> Result<Option<String>> {
    let answer = ask(label, default)?;
    Ok(if answer.is_empty() {
        None
    } else {
        Some(answer)
    })
}

pub fn confirm(label: &str) -> Result<bool> {
    match Confirm::new(label).with_default(false).prompt() {
        Ok(answer) => Ok(answer),
        Err(inquire::InquireError::OperationCanceled)
        | Err(inquire::InquireError::OperationInterrupted) => Ok(false),
        Err(e) => Err(e.into()),
    }
}

fn kind_options(default: Kind) -> Vec<&'static str> {
    match default {
        Kind::Book => vec!["book", "article"],
        Kind::Article => vec!["article", "book"],
    }
}

pub fn ask_kind(default: Kind) -> Result<Kind> {
    Kind::parse(Select::new("  kind", kind_options(default)).prompt()?)
}

const SKIP: &str = "skip this file";

/// Like `ask_kind`, plus a way out. Typing `s` filters the list down to the
/// skip entry, and Escape skips as well — abandoning one file partway through
/// a long import should not abandon the import.
pub fn ask_kind_or_skip(default: Kind) -> Result<Option<Kind>> {
    let mut options = kind_options(default);
    options.push(SKIP);

    match Select::new("  kind", options).prompt() {
        Ok(choice) if choice == SKIP => Ok(None),
        Ok(choice) => Ok(Some(Kind::parse(choice)?)),
        Err(inquire::InquireError::OperationCanceled) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// True when the user hit Ctrl-C. Escape means "not this one"; Ctrl-C means
/// "stop", and a per-item loop has to tell them apart.
pub fn was_interrupted(error: &anyhow::Error) -> bool {
    matches!(
        error.downcast_ref::<inquire::InquireError>(),
        Some(inquire::InquireError::OperationInterrupted)
    )
}

/// Prompt for every field this kind uses, pre-filled with what is already
/// known. Shared by `import` and `edit` so both ask the same questions.
pub fn prompt_fields(found: &Extracted, kind: Kind, indent: &str) -> Result<Extracted> {
    let label = |name: &str| format!("{indent}{name}");
    let mut out = found.clone();

    out.title = Some(ask(&label("title"), found.title.as_deref())?);
    out.authors = ask_opt(&label("authors"), found.authors.as_deref())?;
    out.year = ask_opt(&label("year"), found.year.map(|y| y.to_string()).as_deref())?
        .and_then(|y| y.parse::<i64>().ok());

    match kind {
        Kind::Book => {
            out.publisher = ask_opt(&label("publisher"), found.publisher.as_deref())?;
            out.edition = ask_opt(&label("edition"), found.edition.as_deref())?;
            out.isbn = ask_opt(&label("isbn"), found.isbn.as_deref())?;
        }
        Kind::Article => {
            out.journal = ask_opt(&label("journal"), found.journal.as_deref())?;
            out.volume = ask_opt(&label("volume"), found.volume.as_deref())?;
            out.issue = ask_opt(&label("issue"), found.issue.as_deref())?;
            out.pages = ask_opt(&label("pages"), found.pages.as_deref())?;
            out.doi = ask_opt(&label("doi"), found.doi.as_deref())?;
        }
    }
    Ok(out)
}

pub fn ask_tags(conn: &Connection, current: &[String], label: &str) -> Result<Vec<String>> {
    let known = db::all_tags(conn)?;
    if !known.is_empty() {
        println!("{label}known tags: {}", known.join(", "));
    }
    let answer = ask(
        &format!("{label}tags (comma separated)"),
        Some(&current.join(", ")),
    )?;
    Ok(answer
        .split(',')
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .collect())
}

/// Search, then fall through to an interactive pick: no matches means picking
/// from the whole catalog, several matches means picking among them.
pub fn resolve(conn: &Connection, query: &str) -> Result<Option<Document>> {
    let matches = db::search(conn, query)?;
    match matches.len() {
        1 => Ok(matches.into_iter().next()),
        0 => {
            if !query.trim().is_empty() {
                println!("no match for {query:?} — showing everything.");
            }
            pick(db::all(conn)?, "document")
        }
        _ => pick(matches, "document"),
    }
}
