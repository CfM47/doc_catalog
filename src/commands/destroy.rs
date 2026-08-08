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

    let stored = if remote {
        storage::check_ready(cfg)?;
        storage::list(cfg)?
            .into_iter()
            .filter(|(path, _)| is_doclib_file(path))
            .collect()
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
        let bytes: u64 = stored.iter().map(|(_, size)| size).sum();
        println!(
            "  {} stored file(s), {}, from {}",
            stored.len(),
            cache::human_bytes(bytes),
            cfg.remote
        );
    } else {
        println!(
            "\nthe stored files at {} are kept. re-run with --remote to delete those too.",
            cfg.remote
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
    for (path, _) in &stored {
        match storage::delete(cfg, path) {
            Ok(()) => deleted_remote += 1,
            Err(e) => eprintln!("  failed to delete {path}: {e:#}"),
        }
    }
    if remote {
        println!("deleted {deleted_remote} file(s) from {}", cfg.remote);
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

/// Only touch files this tool wrote: `<two hex>/<64 hex>.<ext>`. A remote
/// pointed at a shared folder may hold things that are none of our business.
fn is_doclib_file(path: &str) -> bool {
    let Some((shard, name)) = path.split_once('/') else {
        return false;
    };
    let Some((hash, ext)) = name.rsplit_once('.') else {
        return false;
    };
    let hex = |s: &str| s.chars().all(|c| c.is_ascii_hexdigit());

    shard.len() == 2
        && hex(shard)
        && hash.len() == 64
        && hex(hash)
        && hash.starts_with(shard)
        && !ext.is_empty()
        && ext.chars().all(|c| c.is_ascii_alphanumeric())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognises_files_this_tool_wrote() {
        let hash = "920c622478f05bf785b0549899ce6e670dbc1c9ba5d3699a718f07d31d4a6105";
        assert!(is_doclib_file(&format!("92/{hash}.pdf")));
        assert!(is_doclib_file(&format!("92/{hash}.epub")));
    }

    #[test]
    fn leaves_everything_else_alone() {
        let hash = "920c622478f05bf785b0549899ce6e670dbc1c9ba5d3699a718f07d31d4a6105";
        // Someone else's files sharing the remote must survive a --remote wipe.
        assert!(!is_doclib_file("holiday-photos/beach.jpg"));
        assert!(!is_doclib_file("notes.txt"));
        assert!(!is_doclib_file("92/not-a-hash.pdf"));
        assert!(!is_doclib_file(&format!("{hash}.pdf")));
        // Shard must match the hash it claims to shard.
        assert!(!is_doclib_file(&format!("ab/{hash}.pdf")));
    }
}
