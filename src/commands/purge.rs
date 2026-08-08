use anyhow::Result;
use rusqlite::Connection;
use std::collections::{HashMap, HashSet};

use crate::cache;
use crate::config::Config;
use crate::db;
use crate::storage;
use crate::ui;

/// Delete stored files that no catalog entry points at. These come from
/// `doclib delete` without `--purge`, and from imports that failed after the
/// upload but before the database write.
pub fn run(conn: &Connection, cfg: &Config, assume_yes: bool) -> Result<()> {
    storage::check_ready(cfg)?;

    let stored = storage::list(cfg)?;
    if stored.is_empty() {
        println!("nothing stored at {}", cfg.remote);
        return Ok(());
    }

    let live: HashSet<String> = db::all(conn)?
        .into_iter()
        .map(|doc| doc.remote_path)
        .collect();

    let known: HashMap<String, db::Tombstone> = db::tombstones(conn)?
        .into_iter()
        .map(|t| (t.remote_path.clone(), t))
        .collect();

    let present: HashSet<&str> = stored.iter().map(|(path, _)| path.as_str()).collect();
    clear_stale_tombstones(conn, &known, &live, &present)?;

    let orphans: Vec<(String, u64)> = stored
        .iter()
        .filter(|(path, _)| !live.contains(path))
        .cloned()
        .collect();

    if orphans.is_empty() {
        println!(
            "no orphans: every stored file is referenced by the catalog ({} document(s)).",
            live.len()
        );
        return Ok(());
    }

    let total: u64 = orphans.iter().map(|(_, size)| size).sum();
    println!("orphaned files at {}:\n", cfg.remote);
    for (path, size) in &orphans {
        match known.get(path) {
            Some(t) => println!(
                "  {}  {}  {}  deleted {}",
                ui::truncate(&t.title, 40),
                ui::truncate(t.authors.as_deref().unwrap_or("-"), 20),
                cache::human_bytes(*size),
                &t.deleted_at[..10.min(t.deleted_at.len())]
            ),
            // No tombstone: uploaded but never catalogued, so only the hash
            // in the filename identifies it.
            None => println!(
                "  {}  {}  unknown origin",
                ui::truncate(path, 62),
                cache::human_bytes(*size)
            ),
        }
    }
    println!(
        "\n{} file(s), {} — permanently deleted from {}",
        orphans.len(),
        cache::human_bytes(total),
        cfg.remote
    );

    if !assume_yes && !ui::confirm("\ndelete these files?")? {
        println!("cancelled.");
        return Ok(());
    }

    let mut deleted = 0;
    let mut freed = 0;
    for (path, size) in &orphans {
        match storage::delete(cfg, path) {
            Ok(()) => {
                if let Some(t) = known.get(path) {
                    db::forget_tombstone(conn, &t.content_hash)?;
                }
                deleted += 1;
                freed += size;
            }
            Err(e) => eprintln!("  failed to delete {path}: {e:#}"),
        }
    }

    println!(
        "deleted {deleted} file(s), freed {}",
        cache::human_bytes(freed)
    );
    Ok(())
}

/// A tombstone stops describing anything once its file is gone from the remote
/// or its path has been re-imported into the catalog.
fn clear_stale_tombstones(
    conn: &Connection,
    known: &HashMap<String, db::Tombstone>,
    live: &HashSet<String>,
    present: &HashSet<&str>,
) -> Result<()> {
    for (path, t) in known {
        if live.contains(path) || !present.contains(path.as_str()) {
            db::forget_tombstone(conn, &t.content_hash)?;
        }
    }
    Ok(())
}
