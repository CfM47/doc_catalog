use anyhow::{Context, Result};
use rusqlite::Connection;

use crate::cache;
use crate::config::Config;
use crate::db;
use crate::storage;
use crate::ui;

/// Typing `y` is too easy for something this final.
const CONFIRMATION: &str = "destroy";

pub fn run(conn: Connection, cfg: &Config, remote: bool, assume_yes: bool) -> Result<()> {
    let documents = db::all(&conn)?;
    let tags = db::tag_counts(&conn)?;
    let cached = cache::stats(cfg)?;

    let stored: Vec<(String, String, u64)> = if remote {
        let reachable = storage::check_ready(cfg)?;
        let mut all = Vec::new();
        for r in &reachable.usable {
            for (path, size) in storage::list(r)? {
                all.push((r.clone(), path, size));
            }
        }
        all
    } else {
        Vec::new()
    };

    println!("this will permanently delete:\n");
    println!(
        "  {} document(s) and {} tag(s) from {}",
        documents.len(),
        tags.len(),
        cfg.db_path().display()
    );
    println!(
        "  {} cached file(s), {}, from {}",
        cached.files,
        cache::human_bytes(cached.bytes),
        cfg.cache_dir().display()
    );

    if remote {
        for r in &cfg.remotes {
            let files: Vec<_> = stored.iter().filter(|(from, _, _)| from == r).collect();
            let bytes: u64 = files.iter().map(|(_, _, size)| size).sum();
            println!(
                "  {} stored file(s), {}, from {r}",
                files.len(),
                cache::human_bytes(bytes)
            );
        }
    } else {
        println!(
            "\nthe stored files on {} remote(s) are kept. re-run with --remote to delete those too.",
            cfg.remotes.len()
        );
    }

    println!("\nthere is no undo. `doclib export <file>` first if you want the tags back.");

    if !assume_yes {
        let answer = ui::ask(&format!("\ntype {CONFIRMATION} to confirm"), None)?;
        if answer != CONFIRMATION {
            println!("cancelled.");
            return Ok(());
        }
    }

    // Remote first: if it fails, the catalog still describes what is out there.
    let mut deleted_remote = 0;
    for (from, path, _) in &stored {
        match storage::delete(from, path) {
            Ok(()) => deleted_remote += 1,
            Err(e) => eprintln!("  failed to delete {path} from {from}: {e:#}"),
        }
    }
    if remote {
        println!(
            "deleted {deleted_remote} file(s) across {} remote(s)",
            cfg.remotes.len()
        );
    }

    // Close the database before unlinking it, so SQLite does not write the
    // WAL back out over the removal.
    drop(conn);
    remove_database(cfg)?;

    let cache_dir = cfg.cache_dir();
    if cache_dir.exists() {
        std::fs::remove_dir_all(&cache_dir)
            .with_context(|| format!("removing {}", cache_dir.display()))?;
    }

    println!("the library is gone. `doclib import` starts a new one.");
    Ok(())
}

fn remove_database(cfg: &Config) -> Result<()> {
    let db = cfg.db_path();
    for suffix in ["", "-wal", "-shm"] {
        let path = if suffix.is_empty() {
            db.clone()
        } else {
            let mut name = db.clone().into_os_string();
            name.push(suffix);
            name.into()
        };
        if path.exists() {
            std::fs::remove_file(&path).with_context(|| format!("removing {}", path.display()))?;
        }
    }
    Ok(())
}
