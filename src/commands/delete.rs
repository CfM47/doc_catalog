use anyhow::Result;
use rusqlite::Connection;

use crate::config::Config;
use crate::db;
use crate::storage;
use crate::ui;

/// Remove a document from the catalog. The stored copy on the remote survives
/// unless `purge` is set, so a mistaken delete costs metadata, not the file.
pub fn run(
    conn: &Connection,
    cfg: &Config,
    query: &str,
    purge: bool,
    assume_yes: bool,
) -> Result<()> {
    let Some(doc) = ui::resolve(conn, query)? else {
        return Ok(());
    };

    let cached = cfg.cache_path(&doc.content_hash, &doc.ext);
    let tags = db::tags_for(conn, &doc.id)?;

    println!("\n{}", doc.title);
    if let Some(authors) = &doc.authors {
        println!("{authors}");
    }
    if !tags.is_empty() {
        println!("tags: {}", tags.join(", "));
    }

    println!("\nthis will remove:");
    println!("  - the catalog entry and its tags");
    if cached.exists() {
        println!("  - the cached file at {}", cached.display());
    }
    if purge {
        println!("  - the stored copy at {}/{}", cfg.remote, doc.remote_path);
    } else {
        println!(
            "\nthe stored copy at {}/{} is kept, and `doclib purge` can delete it later.",
            cfg.remote, doc.remote_path
        );
    }

    if !assume_yes && !ui::confirm("\ndelete this document?")? {
        println!("cancelled.");
        return Ok(());
    }

    // Purge first: if it fails, the catalog entry survives and still points at
    // the file, so nothing is orphaned.
    if purge {
        storage::check_available()?;
        storage::delete(cfg, &doc.remote_path)?;
        db::forget_tombstone(conn, &doc.content_hash)?;
        println!("deleted from {}", cfg.remote);
    } else {
        // The file outlives the catalog entry, so remember what it was —
        // otherwise `doclib purge` could only report it as a bare hash.
        db::tombstone(conn, &doc)?;
    }

    if cached.exists() {
        std::fs::remove_file(&cached)?;
    }
    db::delete(conn, &doc.id)?;

    println!("removed {:?} from the catalog.", doc.title);
    Ok(())
}
