mod cache;
mod commands;
mod config;
mod db;
mod lookup;
mod metadata;
mod model;
mod storage;
mod ui;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

use config::Config;
use model::Kind;

#[derive(Parser)]
#[command(
    name = "doclib",
    version,
    about = "Tag-indexed library for books and articles"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Copy files into the catalog and upload them to the remote
    Import {
        /// File or directory to scan
        path: PathBuf,
        /// Accept extracted metadata without prompting
        #[arg(long)]
        auto: bool,
        /// Treat everything as this kind instead of guessing
        #[arg(long, value_parser = ["book", "article"])]
        kind: Option<String>,
    },
    /// Full-text search over titles, authors, publishers and journals
    Search { query: Option<String> },
    /// Search, pick, and open in the configured reader
    Open {
        #[arg(default_value = "")]
        query: String,
    },
    /// Print every stored field for one document
    Show {
        #[arg(default_value = "")]
        query: String,
    },
    /// Correct the metadata on one document
    Edit {
        #[arg(default_value = "")]
        query: String,
        /// Re-fetch from OpenLibrary or Crossref before prompting
        #[arg(long)]
        lookup: bool,
    },
    /// Remove a document from the catalog
    Delete {
        #[arg(default_value = "")]
        query: String,
        /// Also delete the stored copy from the remote
        #[arg(long)]
        purge: bool,
        /// Skip the confirmation prompt
        #[arg(short = 'y', long = "yes")]
        assume_yes: bool,
    },
    /// List the catalog, optionally filtered
    List {
        #[arg(long)]
        tag: Option<String>,
        #[arg(long, value_parser = ["book", "article"])]
        kind: Option<String>,
    },
    /// Show every tag with its document count
    Tags,
    /// Edit the tags on one document
    Tag {
        #[arg(default_value = "")]
        query: String,
    },
    /// Verify the remote holds every catalogued document
    Sync,
    /// Delete stored files that no catalog entry points at
    Purge {
        /// Skip the confirmation prompt
        #[arg(short = 'y', long = "yes")]
        assume_yes: bool,
    },
    /// Inspect or prune the local cache
    Cache {
        #[command(subcommand)]
        action: CacheAction,
    },
    /// Print the config file path
    Config,
}

#[derive(Subcommand)]
enum CacheAction {
    Status,
    Prune {
        /// Ceiling in bytes; defaults to cache_max_bytes from the config
        #[arg(long)]
        max: Option<u64>,
    },
}

/// Single timestamp format across the schema.
pub fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}

fn parse_kind(value: &Option<String>) -> Result<Option<Kind>> {
    match value {
        Some(v) => Ok(Some(Kind::parse(v)?)),
        None => Ok(None),
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let cfg = Config::load()?;
    let conn = db::open(&cfg.db_path())?;

    match &cli.command {
        Command::Import { path, auto, kind } => {
            commands::import::run(&conn, &cfg, path, *auto, parse_kind(kind)?)
        }
        Command::Search { query } => commands::search::run(&conn, query.as_deref()),
        Command::Open { query } => commands::open::run(&conn, &cfg, query),
        Command::Show { query } => commands::show::run(&conn, &cfg, query),
        Command::Edit { query, lookup } => commands::edit::run(&conn, query, *lookup),
        Command::Delete {
            query,
            purge,
            assume_yes,
        } => commands::delete::run(&conn, &cfg, query, *purge, *assume_yes),
        Command::List { tag, kind } => {
            commands::list::run(&conn, tag.as_deref(), parse_kind(kind)?)
        }
        Command::Tags => commands::list::tags(&conn),
        Command::Tag { query } => commands::tag::run(&conn, query),
        Command::Sync => commands::sync::run(&conn, &cfg),
        Command::Purge { assume_yes } => commands::purge::run(&conn, &cfg, *assume_yes),
        Command::Cache { action } => match action {
            CacheAction::Status => commands::cache_cmd::status(&cfg),
            CacheAction::Prune { max } => commands::cache_cmd::prune(&conn, &cfg, *max),
        },
        Command::Config => {
            println!("{}", config::config_path()?.display());
            Ok(())
        }
    }
}
