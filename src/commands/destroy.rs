use anyhow::{Context, Result};
use rusqlite::Connection;

use crate::cache;
use crate::config::Config;
use crate::db;
use crate::storage;
use crate::ui;

/// Typing `y` is too easy for something this final.
const CONFIRMATION: &str = "destroy";

pub fn run(conn: Connection, cfg: &Config, stores: bool, assume_yes: bool) -> Result<()> {
    let documents = db::all(&conn)?;
    let tags = db::tag_counts(&conn)?;
    let cached = cache::stats(cfg)?;

    let stored: Vec<(std::path::PathBuf, String, u64)> = if stores {
        let available = storage::check_ready(cfg)?;
        let mut all = Vec::new();
        for store in &available.usable {
            for (path, size) in storage::list(store)? {
                all.push((store.clone(), path, size));
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

    if stores {
        for store in &cfg.stores {
            let files: Vec<_> = stored.iter().filter(|(from, _, _)| from == store).collect();
            let bytes: u64 = files.iter().map(|(_, _, size)| size).sum();
            println!(
                "  {} stored file(s), {}, from {}",
                files.len(),
                cache::human_bytes(bytes),
                store.display()
            );
        }
    } else {
        println!(
            "\nthe files in {} store(s) are kept. re-run with --stores to delete those too.",
            cfg.stores.len()
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
    let mut deleted = 0;
    for (from, path, _) in &stored {
        match storage::remove(from, path) {
            Ok(()) => deleted += 1,
            Err(e) => eprintln!("  failed to delete {path} from {}: {e:#}", from.display()),
        }
    }
    if stores {
        println!(
            "deleted {deleted} file(s) across {} store(s)",
            cfg.stores.len()
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
