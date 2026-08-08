//! Converge every remote on the union of what they all hold.
//!
//! Remotes are treated as append-only replicas: if a file exists on one and
//! not another, the answer is always to copy it, never to delete it. Deletion
//! is the job of `delete --purge`, `purge` and `destroy`, which act on every
//! remote at once precisely so that this command cannot resurrect the file.

use anyhow::{Result, bail};
use std::collections::{BTreeMap, HashMap, HashSet};

use crate::cache;
use crate::config::Config;
use crate::db;
use crate::storage;
use crate::ui;

pub fn run(conn: &rusqlite::Connection, cfg: &Config, dry_run: bool) -> Result<()> {
    let reachable = storage::check_ready(cfg)?;

    if cfg.remotes.len() < 2 {
        bail!(
            "update copies files between remotes, and only one is configured.\n\
             add another to `remotes` in the config, then run this again."
        );
    }
    if reachable.usable.len() < 2 {
        bail!(
            "only {} of {} remotes are reachable, so there is nothing to converge.",
            reachable.usable.len(),
            cfg.remotes.len()
        );
    }

    // Listing is the expensive part; do it once per remote.
    let mut holdings: Vec<(String, HashMap<String, u64>)> = Vec::new();
    for remote in &reachable.usable {
        let files = storage::list(remote)?;
        println!("{remote}: {} file(s)", files.len());
        holdings.push((remote.clone(), files.into_iter().collect()));
    }

    // The union is the target state for every remote. BTreeMap so the report
    // and the copy order are stable between runs.
    //
    // Deleted files are excluded: a remote that was offline during the delete
    // still holds a copy, and without this the union would faithfully copy it
    // back to everywhere it was just removed from.
    let deleted = db::tombstoned_paths(conn)?;
    let mut union: BTreeMap<String, u64> = BTreeMap::new();
    let mut resurrections_avoided = 0;
    for (_, files) in &holdings {
        for (path, size) in files {
            if deleted.contains(path) {
                resurrections_avoided += 1;
                continue;
            }
            union.insert(path.clone(), *size);
        }
    }
    if resurrections_avoided > 0 {
        println!(
            "\nignoring {} deleted file(s) still present on some remote — \
             `doclib purge` removes them.",
            deleted.len()
        );
    }

    let plan: Vec<Transfer> = holdings
        .iter()
        .flat_map(|(remote, files)| {
            union
                .iter()
                .filter(|(path, _)| !files.contains_key(*path))
                .filter_map(|(path, size)| {
                    // Any remote that already has it will do as the source.
                    let source = holdings.iter().find(|(other, other_files)| {
                        other != remote && other_files.contains_key(path)
                    })?;
                    Some(Transfer {
                        source: source.0.clone(),
                        destination: remote.clone(),
                        path: path.clone(),
                        size: *size,
                    })
                })
                .collect::<Vec<_>>()
        })
        .collect();

    if plan.is_empty() {
        println!(
            "\nall {} remote(s) already hold the same {} file(s).",
            holdings.len(),
            union.len()
        );
        return report_unreachable(&reachable);
    }

    print_plan(&plan, dry_run);
    if dry_run {
        return report_unreachable(&reachable);
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
                    transfer.path, transfer.destination
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
    report_unreachable(&reachable)
}

struct Transfer {
    source: String,
    destination: String,
    path: String,
    size: u64,
}

fn print_plan(plan: &[Transfer], dry_run: bool) {
    // Grouped by destination: "what does this remote still need" is the
    // question a reader actually has.
    let mut by_destination: BTreeMap<&str, (usize, u64)> = BTreeMap::new();
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
            ui::truncate(destination, 40),
            cache::human_bytes(*bytes)
        );
    }

    if dry_run {
        for transfer in plan {
            println!(
                "    {} -> {}  ({})",
                transfer.path,
                transfer.destination,
                cache::human_bytes(transfer.size)
            );
        }
    }
}

fn report_unreachable(reachable: &storage::Reachability) -> Result<()> {
    if !reachable.unusable.is_empty() {
        let names: HashSet<&str> = reachable.unusable.iter().map(|(r, _)| r.as_str()).collect();
        println!(
            "\n{} remote(s) were skipped and may still be missing files: {}",
            names.len(),
            names.into_iter().collect::<Vec<_>>().join(", ")
        );
    }
    Ok(())
}
