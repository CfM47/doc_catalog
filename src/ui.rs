use anyhow::Result;
use inquire::Select;
use rusqlite::Connection;
use std::fmt;

use crate::db;
use crate::model::Document;

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
