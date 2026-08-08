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
///
/// Every remote is scanned, because a file left on one would be copied back to
/// the others by `doclib update`.
pub fn run(conn: &Connection, cfg: &Config, assume_yes: bool) -> Result<()> {
    let reachable = storage::check_ready(cfg)?;

    let live: HashSet<String> = db::all(conn)?
        .into_iter()
        .map(|doc| doc.remote_path)
        .collect();

    let known: HashMap<String, db::Tombstone> = db::tombstones(conn)?
        .into_iter()
        .map(|t| (t.remote_path.clone(), t))
        .collect();

    let mut orphans: Vec<(String, String, u64)> = Vec::new();
    for remote in &reachable.usable {
        for (path, size) in storage::list(remote)? {
            if !live.contains(&path) {
                orphans.push((remote.clone(), path, size));
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
    for (remote, path, size) in &orphans {
        let label = match known.get(path) {
            Some(t) => format!(
                "{}  {}  deleted {}",
                ui::truncate(&t.title, 36),
                ui::truncate(t.authors.as_deref().unwrap_or("-"), 18),
                &t.deleted_at[..10.min(t.deleted_at.len())]
            ),
            // No tombstone: uploaded but never catalogued, so only the hash in
            // the filename identifies it.
            None => format!("{}  unknown origin", ui::truncate(path, 56)),
        };
        println!(
            "  {}  {}  {}",
            ui::truncate(remote, 24),
            label,
            cache::human_bytes(*size)
        );
    }
    println!(
        "\n{} file(s) across {} remote(s), {} — permanently deleted",
        orphans.len(),
        reachable.usable.len(),
        cache::human_bytes(total)
    );

    if !assume_yes && !ui::confirm("\ndelete these files?")? {
        println!("cancelled.");
        return Ok(());
    }

    let mut deleted = 0;
    let mut freed = 0;
    let mut purged_hashes: HashSet<String> = HashSet::new();
    let all_remotes_checked = reachable.unusable.is_empty();
    for (remote, path, size) in &orphans {
        match storage::delete(remote, path) {
            Ok(()) => {
                deleted += 1;
                freed += size;
                if all_remotes_checked && let Some(t) = known.get(path) {
                    purged_hashes.insert(t.content_hash.clone());
                }
            }
            Err(e) => eprintln!("  failed to delete {path} from {remote}: {e:#}"),
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
/// Deliberately *not* cleared merely because no remote listed the file: an
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
