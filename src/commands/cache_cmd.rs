use anyhow::Result;
use rusqlite::Connection;

use crate::cache;
use crate::config::Config;

pub fn status(cfg: &Config) -> Result<()> {
    let stats = cache::stats(cfg)?;
    println!("cache dir : {}", cfg.cache_dir().display());
    println!("files     : {}", stats.files);
    println!("size      : {}", cache::human_bytes(stats.bytes));
    println!(
        "ceiling   : {}",
        if cfg.cache_max_bytes == 0 {
            "unlimited".to_string()
        } else {
            cache::human_bytes(cfg.cache_max_bytes)
        }
    );
    Ok(())
}

pub fn prune(conn: &Connection, cfg: &Config, max: Option<u64>) -> Result<()> {
    let ceiling = max.unwrap_or(cfg.cache_max_bytes);
    let (evicted, freed) = cache::prune(conn, cfg, ceiling)?;
    if evicted == 0 {
        println!("nothing to evict.");
    } else {
        println!(
            "evicted {evicted} file(s), freed {}",
            cache::human_bytes(freed)
        );
    }
    Ok(())
}
