//! Converge every store on the union of what they all hold.
//!
//! Stores are treated as append-only replicas: if a file exists in one and not
//! another, the answer is always to copy it, never to delete it. Deletion is
//! the job of `delete --purge`, `purge` and `destroy`, which record what they
//! removed so that this command cannot copy it back.

use anyhow::{Result, bail};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::PathBuf;

use crate::cache;
use crate::config::Config;
use crate::db;
use crate::storage;

pub fn run(conn: &rusqlite::Connection, cfg: &Config, dry_run: bool) -> Result<()> {
    let available = storage::check_ready(cfg)?;

    if cfg.stores.len() < 2 {
        bail!(
            "update copies files between stores, and only one is configured.\n\
             add another to `stores` in the config, then run this again."
        );
    }
    if available.usable.len() < 2 {
        bail!(
            "only {} of {} stores are available, so there is nothing to converge.",
            available.usable.len(),
            cfg.stores.len()
        );
    }

    let mut holdings: Vec<(PathBuf, HashMap<String, u64>)> = Vec::new();
    for store in &available.usable {
        let files = storage::list(store)?;
        println!("{}: {} file(s)", store.display(), files.len());
        holdings.push((store.clone(), files.into_iter().collect()));
    }

    // The union is the target state for every store. BTreeMap so the report and
    // the copy order are stable between runs.
    //
    // Deleted files are excluded: a store that was disconnected during the
    // delete still holds a copy, and without this the union would faithfully
    // copy it back to everywhere it was just removed from.
    let deleted = db::tombstoned_paths(conn)?;
    let mut union: BTreeMap<String, u64> = BTreeMap::new();
    let mut skipped = 0;
    for (_, files) in &holdings {
        for (path, size) in files {
            if deleted.contains(path) {
                skipped += 1;
                continue;
            }
            union.insert(path.clone(), *size);
        }
    }
    if skipped > 0 {
        println!(
            "\nignoring deleted file(s) still present in some store — \
             `doclib purge` removes them."
        );
    }

    let plan: Vec<Transfer> = holdings
        .iter()
        .flat_map(|(store, files)| {
            union
                .iter()
                .filter(|(path, _)| !files.contains_key(*path))
                .filter_map(|(path, size)| {
                    // Any store that already has it will do as the source.
                    let source = holdings.iter().find(|(other, other_files)| {
                        other != store && other_files.contains_key(path)
                    })?;
                    Some(Transfer {
                        source: source.0.clone(),
                        destination: store.clone(),
                        path: path.clone(),
                        size: *size,
                    })
                })
                .collect::<Vec<_>>()
        })
        .collect();

    if plan.is_empty() {
        println!(
            "\nall {} store(s) already hold the same {} file(s).",
            holdings.len(),
            union.len()
        );
        return report_unavailable(&available);
    }

    print_plan(&plan, dry_run);
    if dry_run {
        return report_unavailable(&available);
    }

    println!();
    let mut copied = 0;
    let mut failed = 0;
    for transfer in &plan {
        match storage::copy_between(&transfer.source, &transfer.destination, &transfer.path) {
            Ok(()) => copied += 1,
            Err(e) => {
                eprintln!(
                    "  failed: {} -> {}: {e:#}",
                    transfer.path,
                    transfer.destination.display()
                );
                failed += 1;
            }
        }
    }

    println!(
        "copied {copied} file(s){}.",
        if failed > 0 {
            format!(", {failed} failed")
        } else {
            String::new()
        }
    );
    report_unavailable(&available)
}

struct Transfer {
    source: PathBuf,
    destination: PathBuf,
    path: String,
    size: u64,
}

fn print_plan(plan: &[Transfer], dry_run: bool) {
    // Grouped by destination: "what does this store still need" is the question
    // a reader actually has.
    let mut by_destination: BTreeMap<&PathBuf, (usize, u64)> = BTreeMap::new();
    for transfer in plan {
        let entry = by_destination
            .entry(&transfer.destination)
            .or_insert((0, 0));
        entry.0 += 1;
        entry.1 += transfer.size;
    }

    println!("\n{}:", if dry_run { "would copy" } else { "copying" });
    for (destination, (count, bytes)) in &by_destination {
        println!(
            "  {}  +{count} file(s), {}",
            destination.display(),
            cache::human_bytes(*bytes)
        );
    }

    if dry_run {
        for transfer in plan {
            println!(
                "    {} -> {}  ({})",
                transfer.path,
                transfer.destination.display(),
                cache::human_bytes(transfer.size)
            );
        }
    }
}

fn report_unavailable(available: &storage::Availability) -> Result<()> {
    if !available.unusable.is_empty() {
        let names: HashSet<String> = available
            .unusable
            .iter()
            .map(|(s, _)| s.display().to_string())
            .collect();
        println!(
            "\n{} store(s) were skipped and may still be missing files: {}",
            names.len(),
            names.into_iter().collect::<Vec<_>>().join(", ")
        );
    }
    Ok(())
}
