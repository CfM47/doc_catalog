//! Stores are plain directories: a folder on this disk, on a mounted USB
//! stick, on an external drive, on anything the operating system has already
//! made look like a filesystem.
//!
//! Every function takes an explicit store. The library may be replicated
//! across several, and no function should have an opinion about which one it
//! is working on.

use anyhow::{Context, Result, bail};
use std::fs;
use std::path::{Path, PathBuf};

use crate::config::Config;

/// Written into the root of every store. Its presence is what distinguishes a
/// real store from an empty directory — which is exactly what an unmounted
/// mount point looks like. Without this, unplugging a USB disk would read as
/// "the store is empty", and the library would happily replicate into the
/// mount point on the internal disk.
pub const MARKER: &str = ".doclib-store";

const MARKER_BODY: &str = "doclib store\n\
                           this file marks the directory as a document store.\n\
                           deleting it makes doclib refuse to touch this folder.\n";

/// Which stores can actually be reached right now.
pub struct Availability {
    pub usable: Vec<PathBuf>,
    pub unusable: Vec<(PathBuf, String)>,
}

/// Check every configured store before any work begins. A single disconnected
/// disk must not block the tool, so this fails only when *none* is available.
pub fn check_ready(cfg: &Config) -> Result<Availability> {
    cfg.validate_stores()?;

    let mut usable = Vec::new();
    let mut unusable = Vec::new();

    for store in &cfg.stores {
        match check_store(store) {
            Ok(()) => usable.push(store.clone()),
            Err(e) => unusable.push((store.clone(), format!("{e:#}"))),
        }
    }

    if usable.is_empty() {
        let detail = unusable
            .iter()
            .map(|(store, why)| format!("  {}: {why}", store.display()))
            .collect::<Vec<_>>()
            .join("\n");
        bail!("no store is available:\n{detail}");
    }

    for (store, why) in &unusable {
        eprintln!("warning: {} is unavailable: {why}", store.display());
    }
    Ok(Availability { usable, unusable })
}

pub fn check_store(store: &Path) -> Result<()> {
    if !store.exists() {
        bail!(
            "{} does not exist — if it is on a removable disk, is it mounted?",
            store.display()
        );
    }
    if !store.is_dir() {
        bail!("{} is not a directory", store.display());
    }
    if !store.join(MARKER).exists() {
        bail!(
            "{} is not a doclib store (no {MARKER}).\n\
             an unmounted disk looks like an empty folder, so doclib will not write here \
             until the store is initialised with `doclib config --store {}`",
            store.display(),
            store.display()
        );
    }
    Ok(())
}

/// Create the directory and drop the marker in. The parent must already exist:
/// if `/mnt/usb` is missing, the disk is not mounted, and creating the whole
/// path would silently build the store on the internal disk instead.
pub fn init_store(store: &Path) -> Result<()> {
    if store.join(MARKER).exists() {
        return Ok(());
    }
    if !store.exists() {
        let parent = store.parent().unwrap_or(Path::new("/"));
        if !parent.exists() {
            bail!(
                "{} does not exist — if this store lives on a removable disk, mount it first",
                parent.display()
            );
        }
        fs::create_dir_all(store).with_context(|| format!("creating {}", store.display()))?;
    }
    fs::write(store.join(MARKER), MARKER_BODY)
        .with_context(|| format!("writing {}", store.join(MARKER).display()))?;
    Ok(())
}

fn full_path(store: &Path, rel: &str) -> PathBuf {
    store.join(rel)
}

/// Copy through a temporary name and rename into place. A file named after its
/// own hash must never be half-written: an interrupted copy would leave bytes
/// that every later check treats as verified.
fn copy_atomic(source: &Path, destination: &Path) -> Result<()> {
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    }
    let temporary = destination.with_extension(format!("part-{}", std::process::id()));

    fs::copy(source, &temporary)
        .with_context(|| format!("copying {} -> {}", source.display(), destination.display()))?;
    // Same directory, so this is a rename within one filesystem: atomic.
    fs::rename(&temporary, destination).with_context(|| {
        let _ = fs::remove_file(&temporary);
        format!("moving into place: {}", destination.display())
    })?;
    Ok(())
}

pub fn put(store: &Path, local: &Path, rel: &str) -> Result<()> {
    copy_atomic(local, &full_path(store, rel))
}

pub fn get(store: &Path, rel: &str, destination: &Path) -> Result<()> {
    let source = full_path(store, rel);
    if !source.exists() {
        bail!("{} does not hold {rel}", store.display());
    }
    copy_atomic(&source, destination)
}

/// Copy one stored file straight from one store to another.
pub fn copy_between(source_store: &Path, destination_store: &Path, rel: &str) -> Result<()> {
    let source = full_path(source_store, rel);
    if !source.exists() {
        bail!("{} does not hold {rel}", source_store.display());
    }
    copy_atomic(&source, &full_path(destination_store, rel))
}

pub fn remove(store: &Path, rel: &str) -> Result<()> {
    let target = full_path(store, rel);
    if !target.exists() {
        // Already absent. The caller wanted it gone; it is gone.
        return Ok(());
    }
    fs::remove_file(&target).with_context(|| format!("removing {}", target.display()))?;

    // Leave the shard directory tidy, but only if emptying it was our doing.
    if let Some(shard) = target.parent()
        && shard != store
        && fs::read_dir(shard)
            .map(|mut d| d.next().is_none())
            .unwrap_or(false)
    {
        let _ = fs::remove_dir(shard);
    }
    Ok(())
}

pub fn exists(store: &Path, rel: &str) -> bool {
    full_path(store, rel).exists()
}

/// Every stored file in one store, as (path relative to its root, bytes).
/// Filtered to this tool's layout: a store may sit alongside other things.
pub fn list(store: &Path) -> Result<Vec<(String, u64)>> {
    let mut found = Vec::new();
    for entry in walkdir::WalkDir::new(store)
        .max_depth(2)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if !entry.file_type().is_file() {
            continue;
        }
        let Ok(relative) = entry.path().strip_prefix(store) else {
            continue;
        };
        let Some(relative) = relative.to_str() else {
            continue;
        };
        if is_doclib_file(relative) {
            let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
            found.push((relative.to_string(), size));
        }
    }
    found.sort();
    Ok(found)
}

/// Only touch files this tool wrote: `<two hex>/<64 hex>.<ext>`. A store
/// sharing a folder with other files leaves those alone.
pub fn is_doclib_file(path: &str) -> bool {
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

    const HASH: &str = "920c622478f05bf785b0549899ce6e670dbc1c9ba5d3699a718f07d31d4a6105";

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("doclib-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn recognises_files_this_tool_wrote() {
        assert!(is_doclib_file(&format!("92/{HASH}.pdf")));
        assert!(is_doclib_file(&format!("92/{HASH}.epub")));
    }

    #[test]
    fn leaves_everything_else_alone() {
        assert!(!is_doclib_file("holiday-photos/beach.jpg"));
        assert!(!is_doclib_file("notes.txt"));
        assert!(!is_doclib_file(".doclib-store"));
        assert!(!is_doclib_file("92/not-a-hash.pdf"));
        assert!(!is_doclib_file(&format!("{HASH}.pdf")));
        // Shard must match the hash it claims to shard.
        assert!(!is_doclib_file(&format!("ab/{HASH}.pdf")));
    }

    #[test]
    fn an_unmarked_directory_is_not_a_store() {
        let dir = scratch("unmarked");
        // This is what an unmounted mount point looks like, and writing the
        // library into it would be silently wrong.
        assert!(check_store(&dir).is_err());

        init_store(&dir).unwrap();
        assert!(check_store(&dir).is_ok());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_missing_directory_reports_the_mount() {
        let dir = scratch("missing").join("not-mounted");
        let message = format!("{:#}", check_store(&dir).unwrap_err());
        assert!(message.contains("mounted"), "got {message}");
        fs::remove_dir_all(dir.parent().unwrap()).unwrap();
    }

    #[test]
    fn init_refuses_when_the_parent_is_absent() {
        // /mnt/usb missing means the disk is not mounted; creating the whole
        // path would build the store on the internal disk instead.
        let dir = scratch("noparent");
        let deep = dir.join("not-mounted").join("lib");
        assert!(init_store(&deep).is_err());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn round_trips_a_file_and_lists_it() {
        let store = scratch("roundtrip");
        init_store(&store).unwrap();

        let source = store.join("source.bin");
        fs::write(&source, b"hello").unwrap();
        let rel = format!("92/{HASH}.pdf");

        put(&store, &source, &rel).unwrap();
        assert!(exists(&store, &rel));
        assert_eq!(list(&store).unwrap(), vec![(rel.clone(), 5)]);

        let back = store.join("back.bin");
        get(&store, &rel, &back).unwrap();
        assert_eq!(fs::read(&back).unwrap(), b"hello");

        remove(&store, &rel).unwrap();
        assert!(!exists(&store, &rel));
        assert!(list(&store).unwrap().is_empty());
        // Removing something already gone is success, not an error.
        assert!(remove(&store, &rel).is_ok());

        fs::remove_dir_all(&store).unwrap();
    }

    #[test]
    fn listing_ignores_the_marker_and_foreign_files() {
        let store = scratch("foreign");
        init_store(&store).unwrap();
        fs::write(store.join("notes.txt"), b"mine").unwrap();
        fs::create_dir_all(store.join("photos")).unwrap();
        fs::write(store.join("photos").join("beach.jpg"), b"jpeg").unwrap();

        assert!(list(&store).unwrap().is_empty());
        fs::remove_dir_all(&store).unwrap();
    }
}
