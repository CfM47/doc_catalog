use anyhow::Result;
use rusqlite::Connection;

use crate::db;
use crate::lookup;
use crate::metadata::Extracted;
use crate::model::Document;
use crate::ui;

pub fn run(conn: &Connection, query: &str, relookup: bool) -> Result<()> {
    let Some(doc) = ui::resolve(conn, query)? else {
        return Ok(());
    };

    println!("editing {:?}\n", doc.title);

    let kind = ui::ask_kind(doc.kind)?;
    let mut fields = to_extracted(&doc);

    if relookup {
        fetch(&mut fields, kind)?;
    }

    let edited = ui::prompt_fields(&fields, kind, "  ")?;
    let updated = apply(&doc, edited, kind);
    db::update(conn, &updated)?;

    let current = db::tags_for(conn, &doc.id)?;
    let tags = ui::ask_tags(conn, &current, "  ")?;
    db::set_tags(conn, &doc.id, &tags)?;

    println!("\nupdated {:?}", updated.title);
    Ok(())
}

/// Re-fetch from OpenLibrary or Crossref, overwriting the stored fields with
/// whatever the registry returns. Unlike import, the user asked for this, so
/// remote values replace local ones outright.
fn fetch(fields: &mut Extracted, kind: crate::model::Kind) -> Result<()> {
    use crate::model::Kind;

    let label = match kind {
        Kind::Article => "  DOI",
        Kind::Book => "  ISBN",
    };
    let current = match kind {
        Kind::Article => fields.doi.clone(),
        Kind::Book => fields.isbn.clone(),
    };

    let identifier = ui::ask(label, current.as_deref())?;
    if identifier.is_empty() {
        println!("  no identifier — skipping lookup");
        return Ok(());
    }

    println!("  looking up {identifier}...");
    let result = match kind {
        Kind::Article => lookup::by_doi(&identifier),
        Kind::Book => lookup::by_isbn(&identifier),
    };

    match result {
        Ok(Some(found)) => {
            println!(
                "  found: {}",
                found.title.as_deref().unwrap_or("(untitled)")
            );
            let mut merged = found;
            merged.merge_from(fields.clone());
            *fields = merged;
        }
        Ok(None) => println!("  no record for {identifier}"),
        Err(e) => println!("  lookup failed ({e}) — continuing"),
    }
    Ok(())
}

fn to_extracted(doc: &Document) -> Extracted {
    Extracted {
        title: Some(doc.title.clone()),
        authors: doc.authors.clone(),
        publisher: doc.publisher.clone(),
        edition: doc.edition.clone(),
        year: doc.year,
        isbn: doc.isbn.clone(),
        doi: doc.doi.clone(),
        journal: doc.journal.clone(),
        volume: doc.volume.clone(),
        issue: doc.issue.clone(),
        pages: doc.pages.clone(),
    }
}

fn apply(doc: &Document, fields: Extracted, kind: crate::model::Kind) -> Document {
    Document {
        kind,
        title: fields
            .title
            .filter(|t| !t.trim().is_empty())
            .unwrap_or_else(|| doc.title.clone()),
        authors: fields.authors,
        year: fields.year,
        publisher: fields.publisher,
        edition: fields.edition,
        isbn: fields.isbn,
        journal: fields.journal,
        volume: fields.volume,
        issue: fields.issue,
        pages: fields.pages,
        doi: fields.doi,
        ..doc.clone()
    }
}
