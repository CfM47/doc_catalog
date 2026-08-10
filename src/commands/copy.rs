//! Export a copy of a document under a name a human can read.
//!
//! The stores are content-addressed, which is right for a library and useless
//! on an e-reader: `92/920c6224….pdf` tells you nothing while you are looking
//! at a device's file list. This writes `Knuth - The Art of Computer
//! Programming (1997).pdf` instead, and never touches the catalog.

use anyhow::{Context, Result, bail};
use rusqlite::Connection;
use std::path::{Path, PathBuf};

use crate::cache;
use crate::config::{self, Config};
use crate::db;
use crate::model::Document;
use crate::ui;

/// Long enough to stay readable, short enough for the 255-byte limit that
/// most filesystems — including the FAT variants e-readers use — impose.
const MAX_STEM: usize = 120;

pub fn run(
    conn: &Connection,
    cfg: &Config,
    query: &str,
    destination: &Path,
    tag: Option<&str>,
) -> Result<()> {
    let destination = config::expand_home(destination);
    if !destination.exists() {
        bail!(
            "{} does not exist — if it is a device, is it plugged in and mounted?",
            destination.display()
        );
    }
    if !destination.is_dir() {
        bail!("{} is not a directory", destination.display());
    }

    let documents = match tag {
        Some(tag) => {
            let documents = db::list(conn, Some(tag), None)?;
            if documents.is_empty() {
                println!("no documents tagged {tag:?}.");
                return Ok(());
            }
            documents
        }
        None => match ui::resolve(conn, query)? {
            Some(doc) => vec![doc],
            None => return Ok(()),
        },
    };

    let mut copied = 0;
    let mut skipped = 0;

    for doc in &documents {
        // Pull it back from a store if the cache no longer holds it.
        let source = match cache::ensure_cached(conn, cfg, doc) {
            Ok(path) => path,
            Err(e) => {
                eprintln!("  {}: {e:#}", doc.title);
                continue;
            }
        };

        let target = match free_name(&destination, doc, &source)? {
            Some(target) => target,
            None => {
                println!("  already there: {}", filename_for(doc));
                skipped += 1;
                continue;
            }
        };

        std::fs::copy(&source, &target)
            .with_context(|| format!("copying to {}", target.display()))?;
        println!(
            "  {}",
            target
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default()
        );
        copied += 1;
    }

    println!(
        "\ncopied {copied} file(s) to {}{}",
        destination.display(),
        if skipped > 0 {
            format!(", {skipped} already present")
        } else {
            String::new()
        }
    );
    Ok(())
}

/// Where to write, or `None` when an identical copy is already there.
/// Same name but different bytes gets a counter rather than an overwrite.
fn free_name(destination: &Path, doc: &Document, source: &Path) -> Result<Option<PathBuf>> {
    let name = filename_for(doc);
    let stem = name
        .rsplit_once('.')
        .map(|(stem, _)| stem.to_string())
        .unwrap_or_else(|| name.clone());

    let mut candidate = destination.join(&name);
    let mut counter = 1;

    while candidate.exists() {
        if same_bytes(&candidate, source, doc)? {
            return Ok(None);
        }
        counter += 1;
        candidate = destination.join(format!("{stem} ({counter}).{}", doc.ext));
    }
    Ok(Some(candidate))
}

/// Size first, since it settles almost every case without reading anything.
fn same_bytes(existing: &Path, source: &Path, doc: &Document) -> Result<bool> {
    let size = std::fs::metadata(existing)?.len();
    if size != doc.size.max(0) as u64 {
        return Ok(false);
    }
    Ok(cache::hash_file(existing)? == cache::hash_file(source)?)
}

/// `Author - Title (Year).ext`, dropping whichever parts are unknown.
pub fn filename_for(doc: &Document) -> String {
    let title = sanitize(&doc.title);
    let title = if title.is_empty() {
        "untitled".to_string()
    } else {
        title
    };

    let author = doc.authors.as_deref().and_then(short_author);
    let year = doc.year.map(|y| format!(" ({y})")).unwrap_or_default();

    let prefix = author.map(|a| format!("{a} - ")).unwrap_or_default();
    // Trim the title rather than the author or year: those are what make two
    // similar entries tellable apart in a device's file list.
    let room = MAX_STEM.saturating_sub(prefix.chars().count() + year.chars().count());
    let title = trim_to(&title, room.max(8));

    format!("{prefix}{title}{year}.{}", doc.ext)
}

/// "Alfred V. Aho, Monica S. Lam, …" becomes "Aho et al." — a file list sorts
/// by surname usefully, and by given name not at all.
fn short_author(authors: &str) -> Option<String> {
    let mut names = authors
        .split(',')
        .map(str::trim)
        .filter(|name| !name.is_empty());

    let first = names.next()?;
    let others = names.next().is_some();

    let surname = first.split_whitespace().last().unwrap_or(first);
    let surname = sanitize(surname);
    if surname.is_empty() {
        return None;
    }

    Some(if others {
        format!("{surname} et al.")
    } else {
        surname
    })
}

/// Strip what filesystems reject. FAT is the strictest thing this is likely to
/// meet, so its rules are the ones worth following.
fn sanitize(text: &str) -> String {
    let replaced: String = text
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => ' ',
            c if c.is_control() => ' ',
            c => c,
        })
        .collect();

    // A trailing dot or space is legal to create on Linux and unopenable on
    // Windows, which is what an e-reader is likely to be read from.
    replaced
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim_end_matches(['.', ' '])
        .to_string()
}

fn trim_to(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_string();
    }
    let kept: String = text.chars().take(max).collect();
    kept.trim_end().trim_end_matches(['.', ' ']).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Kind;

    fn doc(title: &str, authors: Option<&str>, year: Option<i64>) -> Document {
        Document {
            id: "id".into(),
            kind: Kind::Book,
            title: title.into(),
            authors: authors.map(str::to_string),
            year,
            publisher: None,
            edition: None,
            isbn: None,
            journal: None,
            volume: None,
            issue: None,
            pages: None,
            doi: None,
            ext: "pdf".into(),
            size: 10,
            content_hash: "hash".into(),
            remote_path: "ha/hash.pdf".into(),
            cached_at: None,
            last_opened: None,
            added_at: "2026-01-01".into(),
            extra: None,
        }
    }

    #[test]
    fn builds_a_readable_name() {
        assert_eq!(
            filename_for(&doc(
                "The Art of Computer Programming",
                Some("Donald E. Knuth"),
                Some(1997)
            )),
            "Knuth - The Art of Computer Programming (1997).pdf"
        );
    }

    #[test]
    fn several_authors_become_et_al() {
        assert_eq!(
            filename_for(&doc(
                "Compilers",
                Some("Alfred V. Aho, Monica S. Lam, Ravi Sethi"),
                Some(2006)
            )),
            "Aho et al. - Compilers (2006).pdf"
        );
    }

    #[test]
    fn missing_parts_are_left_out_rather_than_faked() {
        assert_eq!(filename_for(&doc("Scan", None, None)), "Scan.pdf");
        assert_eq!(
            filename_for(&doc("Scan", Some("Meduna"), None)),
            "Meduna - Scan.pdf"
        );
        assert_eq!(
            filename_for(&doc("Scan", None, Some(2014))),
            "Scan (2014).pdf"
        );
    }

    #[test]
    fn strips_characters_filesystems_reject() {
        let name = filename_for(&doc(
            "Where/When: A Study?",
            Some("A. B. Smith"),
            Some(2020),
        ));
        assert_eq!(name, "Smith - Where When A Study (2020).pdf");
        assert!(!name.contains('/'));
        assert!(!name.contains(':'));
    }

    #[test]
    fn never_ends_a_name_with_a_dot_or_space() {
        // Windows cannot open such a file, and an e-reader is often read there.
        let name = filename_for(&doc("Vol. 1 .", None, None));
        assert_eq!(name, "Vol. 1.pdf");
    }

    #[test]
    fn long_titles_are_trimmed_but_author_and_year_survive() {
        let name = filename_for(&doc(&"A".repeat(400), Some("Knuth"), Some(1997)));
        let stem = name.strip_suffix(".pdf").unwrap();
        assert!(
            stem.chars().count() <= MAX_STEM,
            "{} chars",
            stem.chars().count()
        );
        assert!(stem.starts_with("Knuth - "));
        assert!(stem.ends_with("(1997)"));
    }

    #[test]
    fn an_untitled_document_still_gets_a_name() {
        assert_eq!(filename_for(&doc("///", None, None)), "untitled.pdf");
    }
}
