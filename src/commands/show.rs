use anyhow::Result;
use rusqlite::Connection;

use crate::cache;
use crate::config::Config;
use crate::db;
use crate::model::Kind;
use crate::ui;

pub fn run(conn: &Connection, cfg: &Config, query: &str) -> Result<()> {
    let Some(doc) = ui::resolve(conn, query)? else {
        return Ok(());
    };

    let mut out = Fields::default();
    out.add("title", Some(&doc.title));
    out.add("kind", Some(doc.kind.as_str()));
    out.add("authors", doc.authors.as_deref());
    out.add_owned("year", doc.year.map(|y| y.to_string()));

    match doc.kind {
        Kind::Book => {
            out.add("publisher", doc.publisher.as_deref());
            out.add("edition", doc.edition.as_deref());
            out.add("isbn", doc.isbn.as_deref());
        }
        Kind::Article => {
            out.add("journal", doc.journal.as_deref());
            out.add("volume", doc.volume.as_deref());
            out.add("issue", doc.issue.as_deref());
            out.add("pages", doc.pages.as_deref());
            out.add("doi", doc.doi.as_deref());
            out.add("publisher", doc.publisher.as_deref());
        }
    }

    let tags = db::tags_for(conn, &doc.id)?;
    out.add_owned(
        "tags",
        if tags.is_empty() {
            None
        } else {
            Some(tags.join(", "))
        },
    );

    let local = cfg.cache_path(&doc.content_hash, &doc.ext);
    let cached = local.exists();
    out.add_owned("size", Some(cache::human_bytes(doc.size as u64)));
    out.add_owned(
        "cached",
        Some(if cached {
            local.display().to_string()
        } else {
            "no (fetched on open)".to_string()
        }),
    );
    out.add_owned(
        "remote",
        Some(format!("{}/{}", cfg.remote, doc.remote_path)),
    );
    out.add("added", Some(&doc.added_at));
    out.add(
        "last opened",
        Some(doc.last_opened.as_deref().unwrap_or("never")),
    );
    out.add("hash", Some(&doc.content_hash));
    out.add("id", Some(&doc.id));

    out.print();
    Ok(())
}

/// Collects label/value pairs so the labels can be aligned to the widest one
/// only after every field is known.
#[derive(Default)]
struct Fields {
    rows: Vec<(&'static str, String)>,
}

impl Fields {
    fn add(&mut self, label: &'static str, value: Option<&str>) {
        if let Some(value) = value {
            let value = value.trim();
            if !value.is_empty() {
                self.rows.push((label, value.to_string()));
            }
        }
    }

    fn add_owned(&mut self, label: &'static str, value: Option<String>) {
        self.add(label, value.as_deref());
    }

    fn print(&self) {
        let width = self.rows.iter().map(|(l, _)| l.len()).max().unwrap_or(0);
        for (label, value) in &self.rows {
            println!("{label:<width$}  {value}");
        }
    }
}
