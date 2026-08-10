use anyhow::Result;
use inquire::{Confirm, MultiSelect, Select, Text};
use rusqlite::Connection;
use std::fmt;

use crate::db;
use crate::metadata::Extracted;
use crate::model::{Document, Kind};

/// Wipe the visible screen and home the cursor, leaving scrollback intact so
/// nothing already printed is actually lost. A no-op when stdout is not a
/// terminal, since escape codes in a redirected log are just noise.
pub fn clear_screen() {
    use std::io::{IsTerminal, Write};
    if !std::io::stdout().is_terminal() {
        return;
    }
    print!("\x1b[2J\x1b[1;1H");
    let _ = std::io::stdout().flush();
}

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

/// First entry of the tag picker. Selecting it opens a text prompt for tags
/// that do not exist yet, so creating one costs an extra keystroke only when
/// you actually want one.
const NEW_TAG: &str = "+ new tag…";

/// Pick from the tags already in use, with the current ones pre-selected.
/// Retyping a tag by hand is slow and silently creates near-duplicates —
/// "algorithms" and "algorithm" are different tags and nothing would say so.
pub fn ask_tags(conn: &Connection, current: &[String], label: &str) -> Result<Vec<String>> {
    let known = db::all_tags(conn)?;
    if known.is_empty() {
        // Nothing to choose from yet, so go straight to typing.
        return Ok(parse_tag_list(&ask(
            &format!("{label}tags (comma separated)"),
            Some(&current.join(", ")),
        )?));
    }

    let mut options = vec![NEW_TAG.to_string()];
    options.extend(known.iter().cloned());

    let preselected: Vec<usize> = current
        .iter()
        .filter_map(|tag| options.iter().position(|option| option == tag))
        .collect();

    let chosen = match MultiSelect::new(&format!("{label}tags"), options)
        .with_default(&preselected)
        .with_page_size(15)
        // Clear the filter after every toggle. Keeping it would hide the tags
        // that do not match what was typed, including ones just ticked — and
        // the checkmarks are the only running account of what has been chosen.
        .with_keep_filter(false)
        .with_help_message("type to filter · space to toggle · enter to confirm")
        .prompt()
    {
        Ok(chosen) => chosen,
        // Backing out leaves the tags as they were rather than clearing them.
        Err(inquire::InquireError::OperationCanceled)
        | Err(inquire::InquireError::OperationInterrupted) => return Ok(current.to_vec()),
        Err(e) => return Err(e.into()),
    };

    let wants_new = chosen.iter().any(|tag| tag == NEW_TAG);
    let mut tags: Vec<String> = chosen.into_iter().filter(|tag| tag != NEW_TAG).collect();

    if wants_new {
        let typed = ask(&format!("{label}new tags (comma separated)"), None)?;
        for tag in parse_tag_list(&typed) {
            if !tags.contains(&tag) {
                tags.push(tag);
            }
        }
    }

    tags.sort();
    Ok(tags)
}

fn parse_tag_list(answer: &str) -> Vec<String> {
    let mut tags: Vec<String> = Vec::new();
    for tag in answer.split(',') {
        let tag = tag.trim().to_string();
        if !tag.is_empty() && !tags.contains(&tag) {
            tags.push(tag);
        }
    }
    tags
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
