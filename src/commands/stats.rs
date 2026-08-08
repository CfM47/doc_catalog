use anyhow::Result;
use rusqlite::Connection;

use crate::cache;
use crate::config::Config;
use crate::db;
use crate::model::Kind;
use crate::ui;

const TOP_TAGS: usize = 10;

pub fn run(conn: &Connection, cfg: &Config) -> Result<()> {
    let documents = db::all(conn)?;
    if documents.is_empty() {
        println!("catalog is empty.");
        return Ok(());
    }

    let books = documents.iter().filter(|d| d.kind == Kind::Book).count();
    let articles = documents.len() - books;
    let total_bytes: u64 = documents.iter().map(|d| d.size.max(0) as u64).sum();

    println!(
        "documents  {}  ({books} book(s), {articles} article(s))",
        documents.len()
    );
    println!("total size {}", cache::human_bytes(total_bytes));

    let cached = cache::stats(cfg)?;
    println!(
        "cached     {} file(s), {}{}",
        cached.files,
        cache::human_bytes(cached.bytes),
        if cfg.cache_max_bytes > 0 {
            format!(" of {}", cache::human_bytes(cfg.cache_max_bytes))
        } else {
            String::new()
        }
    );

    println!("stores     {}", cfg.stores.len());

    let years: Vec<i64> = documents.iter().filter_map(|d| d.year).collect();
    if let (Some(min), Some(max)) = (years.iter().min(), years.iter().max()) {
        println!("years      {min}–{max}");
    }

    let read = documents.iter().filter(|d| d.last_opened.is_some()).count();
    println!("opened     {read} of {}", documents.len());

    // Gaps are the point of this command: they say what to fix next.
    let no_authors = documents.iter().filter(|d| d.authors.is_none()).count();
    let no_year = documents.iter().filter(|d| d.year.is_none()).count();
    let untagged = db::untagged_count(conn)?;
    let counts = db::tag_counts(conn)?;

    println!("\nincomplete");
    println!("  no authors  {no_authors}");
    println!("  no year     {no_year}");
    println!("  no tags     {untagged}");

    if counts.is_empty() {
        println!("\nno tags yet.");
        return Ok(());
    }

    println!("\ntags  ({} total)", counts.len());
    for (name, uses) in counts.iter().take(TOP_TAGS) {
        println!("  {}  {uses}", ui::truncate(name, 24));
    }
    if counts.len() > TOP_TAGS {
        println!("  … {} more, see `doclib tags`", counts.len() - TOP_TAGS);
    }
    Ok(())
}
