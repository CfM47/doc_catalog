//! Catalog backup. The stored files are already on the remote and can be
//! re-downloaded; the tags and corrected metadata exist nowhere else, so this
//! writes them somewhere a human can read and version-control.

use anyhow::{Context, Result};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::Path;
use uuid::Uuid;

use crate::config::Config;
use crate::db;
use crate::model::Document;

const FORMAT_VERSION: u32 = 1;

#[derive(Serialize, Deserialize)]
pub struct Catalog {
    pub format_version: u32,
    pub exported_at: String,
    pub documents: Vec<Entry>,
}

#[derive(Serialize, Deserialize)]
pub struct Entry {
    #[serde(flatten)]
    pub document: Document,
    pub tags: Vec<String>,
}

pub fn export(conn: &Connection, destination: Option<&Path>) -> Result<()> {
    let mut documents = Vec::new();
    for document in db::all(conn)? {
        let tags = db::tags_for(conn, &document.id)?;
        documents.push(Entry { document, tags });
    }

    let catalog = Catalog {
        format_version: FORMAT_VERSION,
        exported_at: crate::now(),
        documents,
    };
    // Pretty-printed so a diff between two backups is readable.
    let json = serde_json::to_string_pretty(&catalog)?;

    match destination {
        Some(path) => {
            std::fs::write(path, format!("{json}\n"))
                .with_context(|| format!("writing {}", path.display()))?;
            eprintln!(
                "exported {} document(s) to {}",
                catalog.documents.len(),
                path.display()
            );
        }
        None => {
            let mut out = std::io::stdout().lock();
            out.write_all(json.as_bytes())?;
            out.write_all(b"\n")?;
        }
    }
    Ok(())
}

/// Rebuild catalog entries from a backup. Documents already present are left
/// alone, so restoring into a live catalog merges rather than overwrites.
pub fn restore(conn: &Connection, cfg: &Config, source: &Path) -> Result<()> {
    let text =
        std::fs::read_to_string(source).with_context(|| format!("reading {}", source.display()))?;
    let catalog: Catalog =
        serde_json::from_str(&text).with_context(|| format!("parsing {}", source.display()))?;

    if catalog.format_version > FORMAT_VERSION {
        eprintln!(
            "warning: backup is format version {} and this build understands {FORMAT_VERSION}",
            catalog.format_version
        );
    }

    let mut restored = 0;
    let mut present = 0;

    for entry in catalog.documents {
        let mut document = entry.document;

        if db::find_by_hash(conn, &document.content_hash)?.is_some() {
            present += 1;
            continue;
        }
        // The id is only unique within one catalog; a merge can collide.
        if db::find_by_id(conn, &document.id)?.is_some() {
            document.id = Uuid::new_v4().to_string();
        }
        // Whether the bytes are cached is a fact about this machine, not
        // something a backup can assert.
        document.cached_at = cfg
            .cache_path(&document.content_hash, &document.ext)
            .exists()
            .then(crate::now);

        db::insert(conn, &document)?;
        if !entry.tags.is_empty() {
            db::set_tags(conn, &document.id, &entry.tags)?;
        }
        restored += 1;
    }

    println!("restored {restored}, already present {present}.");
    if restored > 0 {
        println!("run `doclib sync` to check the remote still holds the files.");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Kind;

    fn sample(id: &str, title: &str) -> Document {
        Document {
            id: id.to_string(),
            kind: Kind::Book,
            title: title.to_string(),
            authors: Some("Donald E. Knuth".to_string()),
            year: Some(1997),
            publisher: Some("Addison-Wesley".to_string()),
            edition: Some("3rd".to_string()),
            isbn: Some("9780201896831".to_string()),
            journal: None,
            volume: None,
            issue: None,
            pages: None,
            doi: None,
            ext: "pdf".to_string(),
            size: 1024,
            content_hash: format!("hash-{id}"),
            remote_path: format!("ha/hash-{id}.pdf"),
            cached_at: Some("2026-01-01T00:00:00Z".to_string()),
            last_opened: None,
            added_at: "2026-01-01T00:00:00Z".to_string(),
            extra: None,
        }
    }

    fn round_trip(entries: Vec<Entry>) -> Catalog {
        let catalog = Catalog {
            format_version: FORMAT_VERSION,
            exported_at: "2026-01-01T00:00:00Z".to_string(),
            documents: entries,
        };
        let json = serde_json::to_string_pretty(&catalog).unwrap();
        serde_json::from_str(&json).unwrap()
    }

    #[test]
    fn a_document_survives_the_round_trip_intact() {
        let original = sample("a", "The Art of Computer Programming");
        let parsed = round_trip(vec![Entry {
            document: original.clone(),
            tags: vec!["algorithms".into(), "reference".into()],
        }]);

        let entry = &parsed.documents[0];
        assert_eq!(entry.document.title, original.title);
        assert_eq!(entry.document.kind, Kind::Book);
        assert_eq!(entry.document.isbn, original.isbn);
        assert_eq!(entry.document.edition, original.edition);
        assert_eq!(entry.document.content_hash, original.content_hash);
        assert_eq!(entry.tags, vec!["algorithms", "reference"]);
    }

    #[test]
    fn kind_is_written_as_a_readable_string() {
        // The backup is meant to be read and diffed by a human, so the enum
        // must not serialise as {"Book": null} or an integer.
        let json = serde_json::to_string(&sample("a", "Book")).unwrap();
        assert!(json.contains("\"kind\":\"book\""), "got {json}");
    }

    #[test]
    fn empty_fields_survive_as_null() {
        let mut doc = sample("a", "Scan With No Metadata");
        doc.authors = None;
        doc.year = None;
        let parsed = round_trip(vec![Entry {
            document: doc,
            tags: vec![],
        }]);
        assert!(parsed.documents[0].document.authors.is_none());
        assert!(parsed.documents[0].document.year.is_none());
        assert!(parsed.documents[0].tags.is_empty());
    }
}
