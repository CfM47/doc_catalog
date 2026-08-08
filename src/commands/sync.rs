use anyhow::Result;
use rusqlite::Connection;

use crate::cache;
use crate::config::Config;
use crate::db;
use crate::storage;

/// Verify every reachable remote holds every catalogued document, uploading
/// from the cache to fill gaps. Gaps that the cache cannot fill are reported;
/// `doclib update` closes those by copying between remotes.
pub fn run(conn: &Connection, cfg: &Config) -> Result<()> {
    let reachable = storage::check_ready(cfg)?;

    let docs = db::all(conn)?;
    let mut uploaded = 0;
    let mut unfillable = Vec::new();

    for remote in &reachable.usable {
        let mut missing = 0;
        for doc in &docs {
            if storage::exists(remote, &doc.remote_path)? {
                continue;
            }
            missing += 1;
            let cached = cfg.cache_path(&doc.content_hash, &doc.ext);
            if cached.exists() {
                print!("uploading {} to {remote}... ", doc.title);
                storage::upload(remote, &cached, &doc.remote_path)?;
                println!("done");
                uploaded += 1;
            } else {
                unfillable.push((remote.clone(), doc));
            }
        }
        println!("{remote}: {} document(s), {missing} missing", docs.len());
    }

    let stats = cache::stats(cfg)?;
    println!(
        "\n{} document(s), {} cached ({}), {uploaded} uploaded",
        docs.len(),
        stats.files,
        cache::human_bytes(stats.bytes)
    );

    if !unfillable.is_empty() {
        println!("\n{} gap(s) the cache cannot fill:", unfillable.len());
        for (remote, doc) in &unfillable {
            println!("  {remote}  {}", doc.title);
        }
        if cfg.remotes.len() > 1 {
            println!("run `doclib update` to copy these between remotes.");
        }
    }
    Ok(())
}
