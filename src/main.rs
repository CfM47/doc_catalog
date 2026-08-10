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

const BANNER: &str = r#"
=========================================================================

    ██████╗  ██████╗  ██████╗██╗     ██╗██████╗     ( (
    ██╔══██╗██╔═══██╗██╔════╝██║     ██║██╔══██╗      ) )
    ██║  ██║██║   ██║██║     ██║     ██║██████╔╝   ********
    ██║  ██║██║   ██║██║     ██║     ██║██╔══██╗   ████████▀▌
    ██████╔╝╚██████╔╝╚██████╗███████╗██║██████╔╝   ▀██████▀▀
    ╚═════╝  ╚═════╝  ╚═════╝╚══════╝╚═╝╚═════╝     ▀▀▀▀▀▀

               A blazingly-fast ⚡ memory-safe 💾
Tag-indexed shelf for books 📚 and articles 📄, built in Rust 🦀
                              - CfM47 2026 -
=========================================================================
"#;

#[derive(Parser)]
#[command(
    name = "doclib",
    version,
    about = "Tag-indexed library for books and articles",
    before_help = BANNER
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
    /// Write the catalog to JSON: the tags and corrections exist nowhere else
    Export {
        /// File to write; omit to print to stdout
        file: Option<PathBuf>,
    },
    /// Merge a JSON backup back into the catalog
    Restore {
        /// Backup file written by `doclib export`
        file: PathBuf,
    },
    /// Summarise the catalog and show where metadata is missing
    Stats,
    /// Verify every store holds every catalogued document
    Sync,
    /// Copy files between stores until they all hold the same set
    Update {
        /// Show what would be copied without copying anything
        #[arg(long)]
        dry_run: bool,
    },
    /// Delete stored files that no catalog entry points at
    Purge {
        /// Skip the confirmation prompt
        #[arg(short = 'y', long = "yes")]
        assume_yes: bool,
    },
    /// Permanently delete the whole library
    Destroy {
        /// Also delete the files from every store
        #[arg(long)]
        stores: bool,
        /// Skip the typed confirmation
        #[arg(short = 'y', long = "yes")]
        assume_yes: bool,
    },
    /// Inspect or prune the local cache
    Cache {
        #[command(subcommand)]
        action: CacheAction,
    },
    /// Open the config in $EDITOR, or change one setting directly
    Config {
        /// Set the store folders and exit, without opening an editor
        #[arg(long = "store", value_name = "PATH", num_args = 1..)]
        stores: Vec<PathBuf>,
        /// Print the current settings instead of editing
        #[arg(long)]
        show: bool,
        /// Print only the config file path, for scripting
        #[arg(long)]
        path: bool,
        /// Replace the config with the defaults, keeping the old one as .bak
        #[arg(long)]
        reset: bool,
    },
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

/// Rust sets SIGPIPE to be ignored at startup, so `doclib list | head` turns
/// every later `println!` into a panic instead of ending the process quietly.
/// Restore the default: a reader that stops listening should end the program,
/// not crash it.
#[cfg(unix)]
fn restore_sigpipe() {
    // SAFETY: resetting a signal to its default disposition is
    // async-signal-safe, and this runs before any thread exists.
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }
}

#[cfg(not(unix))]
fn restore_sigpipe() {}

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
    restore_sigpipe();
    let cli = Cli::parse();

    // The config command runs before the config is read: a file that fails to
    // parse must not disable the only command that can repair it.
    if let Command::Config {
        stores,
        show,
        path,
        reset,
    } = &cli.command
    {
        return commands::config_cmd::run(stores, *show, *path, *reset);
    }

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
        Command::Export { file } => commands::export::export(&conn, file.as_deref()),
        Command::Restore { file } => commands::export::restore(&conn, &cfg, file),
        Command::Stats => commands::stats::run(&conn, &cfg),
        Command::Sync => commands::sync::run(&conn, &cfg),
        Command::Update { dry_run } => commands::update::run(&conn, &cfg, *dry_run),
        Command::Purge { assume_yes } => commands::purge::run(&conn, &cfg, *assume_yes),
        Command::Destroy { stores, assume_yes } => {
            commands::destroy::run(conn, &cfg, *stores, *assume_yes)
        }
        Command::Cache { action } => match action {
            CacheAction::Status => commands::cache_cmd::status(&cfg),
            CacheAction::Prune { max } => commands::cache_cmd::prune(&conn, &cfg, *max),
        },
        // Handled above, before the config was loaded.
        Command::Config { .. } => unreachable!(),
    }
}
