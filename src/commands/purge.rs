use anyhow::Result;
use rusqlite::Connection;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use crate::cache;
use crate::config::Config;
use crate::db;
use crate::storage;
use crate::ui;

/// Delete stored files that no catalog entry points at. These come from
/// `doclib delete` without `--purge`, and from imports that failed after the
/// file was stored but before the database write.
///
/// Every available store is scanned, because a file left in one would be copied
/// back to the others by `doclib update`.
pub fn run(conn: &Connection, cfg: &Config, assume_yes: bool) -> Result<()> {
    let available = storage::check_ready(cfg)?;

    let live: HashSet<String> = db::all(conn)?
        .into_iter()
        .map(|doc| doc.remote_path)
        .collect();

    let known: HashMap<String, db::Tombstone> = db::tombstones(conn)?
        .into_iter()
        .map(|t| (t.remote_path.clone(), t))
        .collect();

    let mut orphans: Vec<(PathBuf, String, u64)> = Vec::new();
    for store in &available.usable {
        for (path, size) in storage::list(store)? {
            if !live.contains(&path) {
                orphans.push((store.clone(), path, size));
            }
        }
    }

    clear_stale_tombstones(conn, &known, &live)?;

    if orphans.is_empty() {
        println!(
            "no orphans: every stored file is referenced by the catalog ({} document(s)).",
            live.len()
        );
        return Ok(());
    }

    let total: u64 = orphans.iter().map(|(_, _, size)| size).sum();
    println!("orphaned files:\n");
    for (store, path, size) in &orphans {
        let label = match known.get(path) {
            Some(t) => format!(
                "{}  {}  deleted {}",
                ui::truncate(&t.title, 32),
                ui::truncate(t.authors.as_deref().unwrap_or("-"), 16),
                &t.deleted_at[..10.min(t.deleted_at.len())]
            ),
            // No tombstone: stored but never catalogued, so only the hash in
            // the filename identifies it.
            None => format!("{}  unknown origin", ui::truncate(path, 52)),
        };
        println!(
            "  {}  {label}  {}",
            ui::truncate(&store.display().to_string(), 24),
            cache::human_bytes(*size)
        );
    }
    println!(
        "\n{} file(s) across {} store(s), {} — permanently deleted",
        orphans.len(),
        available.usable.len(),
        cache::human_bytes(total)
    );

    if !assume_yes && !ui::confirm("\ndelete these files?")? {
        println!("cancelled.");
        return Ok(());
    }

    // A tombstone may only be forgotten once every store has been checked; a
    // disconnected one could still be holding the file.
    let all_stores_checked = available.unusable.is_empty();
    let mut deleted = 0;
    let mut freed = 0;
    let mut purged_hashes: HashSet<String> = HashSet::new();

    for (store, path, size) in &orphans {
        match storage::remove(store, path) {
            Ok(()) => {
                deleted += 1;
                freed += size;
                if all_stores_checked && let Some(t) = known.get(path) {
                    purged_hashes.insert(t.content_hash.clone());
                }
            }
            Err(e) => eprintln!("  failed to delete {path} from {}: {e:#}", store.display()),
        }
    }
    for hash in purged_hashes {
        db::forget_tombstone(conn, &hash)?;
    }

    println!(
        "deleted {deleted} file(s), freed {}",
        cache::human_bytes(freed)
    );
    Ok(())
}

/// A tombstone stops describing anything once its path is catalogued again.
///
/// Deliberately *not* cleared merely because no store listed the file: an
/// unmounted disk lists as empty, and forgetting the tombstone on that basis
/// would let `update` copy the file back the moment the disk reappears. A
/// tombstone with no matching file costs one row and blocks nothing — a
/// re-import clears it.
fn clear_stale_tombstones(
    conn: &Connection,
    known: &HashMap<String, db::Tombstone>,
    live: &HashSet<String>,
) -> Result<()> {
    for (path, t) in known {
        if live.contains(path) {
            db::forget_tombstone(conn, &t.content_hash)?;
        }
    }
    Ok(())
}
