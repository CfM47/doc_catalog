use anyhow::Result;
use rusqlite::Connection;

use crate::cache;
use crate::config::Config;
use crate::db;
use crate::storage;

/// Verify every catalogued document exists remotely, re-uploading from the
/// cache when it does not. Documents that are neither remote nor cached are
/// reported, not silently ignored.
pub fn run(conn: &Connection, cfg: &Config) -> Result<()> {
    storage::check_ready(cfg)?;

    let docs = db::all(conn)?;
    let mut uploaded = 0;
    let mut missing = Vec::new();

    for doc in &docs {
        if storage::exists(cfg, &doc.remote_path)? {
            continue;
        }
        let cached = cfg.cache_path(&doc.content_hash, &doc.ext);
        if cached.exists() {
            print!("uploading {}... ", doc.title);
            storage::upload(cfg, &cached, &doc.remote_path)?;
            println!("done");
            uploaded += 1;
        } else {
            missing.push(doc);
        }
    }

    let stats = cache::stats(cfg)?;
    println!(
        "\n{} document(s), {} cached ({}), {uploaded} uploaded",
        docs.len(),
        stats.files,
        cache::human_bytes(stats.bytes)
    );

    if !missing.is_empty() {
        println!(
            "\n{} document(s) exist in neither the remote nor the cache:",
            missing.len()
        );
        for doc in missing {
            println!("  {} [{}]", doc.title, &doc.content_hash[..12]);
        }
    }
    Ok(())
}
