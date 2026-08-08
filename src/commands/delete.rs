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
        for store in &cfg.stores {
            println!(
                "  - the stored copy at {}/{}",
                store.display(),
                doc.remote_path
            );
        }
        println!("  (disconnected stores are cleaned up by a later `doclib purge`)");
    } else {
        println!(
            "
the stored copies of {} are kept, and `doclib purge` can delete them later.",
            doc.remote_path
        );
    }

    if !assume_yes && !ui::confirm("\ndelete this document?")? {
        println!("cancelled.");
        return Ok(());
    }

    // Purge first: if it fails, the catalog entry survives and still points at
    // the file, so nothing is orphaned.
    if purge {
        let available = storage::check_ready(cfg)?;
        let mut survives = available.unusable.len();

        for store in &available.usable {
            match storage::remove(store, &doc.remote_path) {
                Ok(()) => println!("deleted from {}", store.display()),
                Err(e) => {
                    eprintln!("  failed to delete from {}: {e:#}", store.display());
                    survives += 1;
                }
            }
        }

        if survives > 0 {
            // A copy is still out there — on the disconnected USB disk, most
            // likely. Record the deletion so `update` treats the file as gone
            // rather than copying it back, and let `purge` finish the job when
            // that store is next available.
            db::tombstone(conn, &doc)?;
            println!(
                "
{survives} store(s) are unavailable and may still hold this file. it will \
                 not be copied back, and `doclib purge` removes it there once they return."
            );
        } else {
            db::forget_tombstone(conn, &doc.content_hash)?;
        }
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
