use anyhow::Result;
use rusqlite::Connection;

use crate::cache;
use crate::config::Config;
use crate::db;
use crate::storage;

/// Verify every available store holds every catalogued document, copying from
/// the cache to fill gaps. Gaps the cache cannot fill are reported;
/// `doclib update` closes those by copying between stores.
pub fn run(conn: &Connection, cfg: &Config) -> Result<()> {
    let available = storage::check_ready(cfg)?;

    let docs = db::all(conn)?;
    let mut copied = 0;
    let mut unfillable = Vec::new();

    for store in &available.usable {
        let mut missing = 0;
        for doc in &docs {
            if storage::exists(store, &doc.remote_path) {
                continue;
            }
            missing += 1;
            let cached = cfg.cache_path(&doc.content_hash, &doc.ext);
            if cached.exists() {
                print!("copying {} into {}... ", doc.title, store.display());
                storage::put(store, &cached, &doc.remote_path)?;
                println!("done");
                copied += 1;
            } else {
                unfillable.push((store.clone(), doc));
            }
        }
        println!(
            "{}: {} document(s), {missing} missing",
            store.display(),
            docs.len()
        );
    }

    let stats = cache::stats(cfg)?;
    println!(
        "\n{} document(s), {} cached ({}), {copied} copied",
        docs.len(),
        stats.files,
        cache::human_bytes(stats.bytes)
    );

    if !unfillable.is_empty() {
        println!("\n{} gap(s) the cache cannot fill:", unfillable.len());
        for (store, doc) in &unfillable {
            println!("  {}  {}", store.display(), doc.title);
        }
        if cfg.stores.len() > 1 {
            println!("run `doclib update` to copy these between stores.");
        }
    }
    Ok(())
}
