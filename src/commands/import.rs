use anyhow::Result;
use rusqlite::Connection;
use std::path::{Path, PathBuf};
use uuid::Uuid;
use walkdir::WalkDir;

use crate::cache;
use crate::config::Config;
use crate::db;
use crate::lookup;
use crate::metadata::{self, Extracted};
use crate::model::{Document, Kind};
use crate::storage;
use crate::ui;

const EXTENSIONS: [&str; 6] = ["pdf", "epub", "djvu", "mobi", "azw3", "chm"];

pub fn run(
    conn: &Connection,
    cfg: &Config,
    path: &Path,
    auto: bool,
    forced_kind: Option<Kind>,
) -> Result<()> {
    let available = storage::check_ready(cfg)?;

    let files = collect(path);
    if files.is_empty() {
        println!("no supported documents under {}", path.display());
        return Ok(());
    }
    println!("found {} file(s).\n", files.len());

    let mut added = 0;
    let mut skipped = 0;

    for (index, file) in files.iter().enumerate() {
        // One book per screen: with a few hundred to get through, the previous
        // book's prompts scrolling above the current one is just clutter. Left
        // alone under --auto, where the point is to watch the log go by.
        if !auto {
            ui::clear_screen();
            println!(
                "importing {}  ·  added {added}, skipped {skipped}\n",
                path.display()
            );
        }
        println!("[{}/{}] {}", index + 1, files.len(), file.display());
        match import_one(conn, cfg, &available.usable, file, auto, forced_kind) {
            Ok(true) => added += 1,
            Ok(false) => skipped += 1,
            // Ctrl-C means stop, not "fail this one and carry on through the
            // remaining three hundred files".
            Err(e) if ui::was_interrupted(&e) => {
                println!(
                    "\ninterrupted — imported {added}, {} left.",
                    files.len() - index
                );
                return Ok(());
            }
            Err(e) => {
                eprintln!("  failed: {e:#}");
                skipped += 1;
            }
        }
        println!();
    }

    println!("imported {added}, skipped {skipped}.");
    Ok(())
}

fn collect(path: &Path) -> Vec<PathBuf> {
    if path.is_file() {
        return vec![path.to_path_buf()];
    }
    WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .map(|e| e.into_path())
        .filter(|p| {
            p.extension()
                .and_then(|e| e.to_str())
                .map(|e| EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()))
                .unwrap_or(false)
        })
        .collect()
}

/// Returns whether a new document was catalogued.
fn import_one(
    conn: &Connection,
    cfg: &Config,
    stores: &[PathBuf],
    file: &Path,
    auto: bool,
    forced_kind: Option<Kind>,
) -> Result<bool> {
    let hash = cache::hash_file(file)?;
    if let Some(existing) = db::find_by_hash(conn, &hash)? {
        println!("  already catalogued as {:?}", existing.title);
        return Ok(false);
    }

    let ext = file
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("bin")
        .to_ascii_lowercase();
    let size = std::fs::metadata(file)?.len() as i64;

    let mut found = metadata::extract(file);
    let kind = match forced_kind {
        Some(k) => k,
        None if auto => guess_kind(&found),
        None => match ui::ask_kind_or_skip(guess_kind(&found))? {
            Some(kind) => kind,
            None => {
                println!("  skipped.");
                return Ok(false);
            }
        },
    };

    // An identifier is worth more than every embedded field combined: one
    // lookup returns a complete, correct record.
    enrich(&mut found, kind, auto)?;

    let doc = if auto {
        build(&found, kind, &ext, size, &hash, cfg)
    } else {
        let filled = ui::prompt_fields(&found, kind, "  ")?;
        build(&filled, kind, &ext, size, &hash, cfg)
    };

    let cached = cache::store(cfg, file, &hash, &ext)?;
    copy_into_all(stores, &cached, &doc.remote_path)?;

    db::insert(conn, &doc)?;
    // These bytes are catalogued again, so any pending purge no longer applies.
    db::forget_tombstone(conn, &hash)?;

    if !auto {
        let tags = ui::ask_tags(conn, &[], "  ")?;
        if !tags.is_empty() {
            db::set_tags(conn, &doc.id, &tags)?;
        }
    }

    println!("  added {:?}", doc.title);
    Ok(true)
}

/// Copy into every store. A single failure is not fatal — `doclib update`
/// will copy the file across later — but landing nowhere would leave the
/// catalog pointing at bytes that only exist in the cache, which
/// `cache prune` deletes.
fn copy_into_all(stores: &[PathBuf], local: &Path, rel: &str) -> Result<()> {
    print!("  storing...");
    let mut stored = 0;
    let mut failures = Vec::new();

    for store in stores {
        match storage::put(store, local, rel) {
            Ok(()) => {
                print!(" {} ok", store.display());
                stored += 1;
            }
            Err(e) => {
                print!(" {} FAILED", store.display());
                failures.push(format!("    {}: {e:#}", store.display()));
            }
        }
    }
    println!();
    for failure in &failures {
        println!("{failure}");
    }

    if stored == 0 {
        anyhow::bail!("no store accepted the file");
    }
    if !failures.is_empty() {
        println!(
            "    stored in {stored} of {} stores — `doclib update` will finish it",
            stores.len()
        );
    }
    Ok(())
}

fn guess_kind(found: &Extracted) -> Kind {
    if found.doi.is_some() || found.journal.is_some() {
        Kind::Article
    } else {
        Kind::Book
    }
}

fn enrich(found: &mut Extracted, kind: Kind, auto: bool) -> Result<()> {
    let identifier = match kind {
        Kind::Article => found.doi.clone(),
        Kind::Book => found.isbn.clone(),
    };

    let identifier = match identifier {
        Some(id) => Some(id),
        None if auto => None,
        None => {
            let label = match kind {
                Kind::Article => "  DOI (blank to skip lookup)",
                Kind::Book => "  ISBN (blank to skip lookup)",
            };
            ui::ask_opt(label, None)?
        }
    };

    let Some(identifier) = identifier else {
        return Ok(());
    };

    println!("  looking up {identifier}...");
    let result = match kind {
        Kind::Article => lookup::by_doi(&identifier),
        Kind::Book => lookup::by_isbn(&identifier),
    };

    match result {
        // Remote data wins over embedded fields, which are frequently wrong.
        Ok(Some(remote)) => {
            let mut merged = remote;
            merged.merge_from(found.clone());
            *found = merged;
            println!(
                "  found: {}",
                found.title.as_deref().unwrap_or("(untitled)")
            );
        }
        Ok(None) => println!("  no record for {identifier}"),
        Err(e) => println!("  lookup failed ({e}) — continuing"),
    }
    Ok(())
}

fn build(
    found: &Extracted,
    kind: Kind,
    ext: &str,
    size: i64,
    hash: &str,
    cfg: &Config,
) -> Document {
    Document {
        id: Uuid::new_v4().to_string(),
        kind,
        title: found
            .title
            .clone()
            .filter(|t| !t.trim().is_empty())
            .unwrap_or_else(|| "(untitled)".to_string()),
        authors: found.authors.clone(),
        year: found.year,
        publisher: found.publisher.clone(),
        edition: found.edition.clone(),
        isbn: found.isbn.clone(),
        journal: found.journal.clone(),
        volume: found.volume.clone(),
        issue: found.issue.clone(),
        pages: found.pages.clone(),
        doi: found.doi.clone(),
        ext: ext.to_string(),
        size,
        content_hash: hash.to_string(),
        remote_path: cfg.stored_path(hash, ext),
        cached_at: Some(crate::now()),
        last_opened: None,
        added_at: crate::now(),
        extra: None,
    }
}
