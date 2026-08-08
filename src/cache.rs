use anyhow::{Context, Result};
use rusqlite::Connection;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use crate::config::Config;
use crate::db;
use crate::model::Document;

pub fn hash_file(path: &Path) -> Result<String> {
    let mut file = fs::File::open(path).with_context(|| format!("reading {}", path.display()))?;
    let mut hasher = blake3::Hasher::new();
    let mut buf = vec![0u8; 128 * 1024];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hasher.finalize().to_hex().to_string())
}

pub fn store(cfg: &Config, source: &Path, hash: &str, ext: &str) -> Result<PathBuf> {
    let dest = cfg.cache_path(hash, ext);
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }
    if !dest.exists() {
        fs::copy(source, &dest)
            .with_context(|| format!("copying {} -> {}", source.display(), dest.display()))?;
    }
    Ok(dest)
}

/// Local path for a document, pulling it back from the remote if it was evicted.
pub fn ensure_cached(conn: &Connection, cfg: &Config, doc: &Document) -> Result<PathBuf> {
    let path = cfg.cache_path(&doc.content_hash, &doc.ext);
    if path.exists() {
        return Ok(path);
    }
    println!("fetching {} from {}...", doc.title, cfg.remote);
    crate::storage::download(cfg, &doc.remote_path, &path)?;
    db::mark_cached(conn, &doc.id, Some(&crate::now()))?;
    Ok(path)
}

pub struct CacheStats {
    pub files: usize,
    pub bytes: u64,
}

pub fn stats(cfg: &Config) -> Result<CacheStats> {
    let mut files = 0;
    let mut bytes = 0;
    for entry in walkdir::WalkDir::new(cfg.cache_dir())
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() {
            files += 1;
            bytes += entry.metadata().map(|m| m.len()).unwrap_or(0);
        }
    }
    Ok(CacheStats { files, bytes })
}

/// Evict least-recently-opened cached files until under the configured ceiling.
/// Files only exist remotely afterwards; `ensure_cached` pulls them back.
pub fn prune(conn: &Connection, cfg: &Config, max_bytes: u64) -> Result<(usize, u64)> {
    if max_bytes == 0 {
        return Ok((0, 0));
    }
    let stats = stats(cfg)?;
    if stats.bytes <= max_bytes {
        return Ok((0, 0));
    }

    let mut docs = db::all(conn)?;
    // NULL last_opened sorts first: never-read files are evicted before read ones.
    docs.sort_by(|a, b| a.last_opened.cmp(&b.last_opened));

    let mut freed = 0u64;
    let mut evicted = 0usize;
    let mut current = stats.bytes;

    for doc in docs {
        if current <= max_bytes {
            break;
        }
        let path = cfg.cache_path(&doc.content_hash, &doc.ext);
        if !path.exists() {
            continue;
        }
        let size = fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        fs::remove_file(&path)?;
        db::mark_cached(conn, &doc.id, None)?;
        current = current.saturating_sub(size);
        freed += size;
        evicted += 1;
    }
    Ok((evicted, freed))
}

pub fn human_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}
