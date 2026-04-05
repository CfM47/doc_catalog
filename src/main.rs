mod application;
pub mod cli;
mod domain;
mod infrastructure;

use anyhow::Context;
use clap::Parser;
use std::path::PathBuf;

use cli::Cli;
use cli::dependencies::CliDependencies;
use cli::printer::CliPrinter;
use infrastructure::database::Database;
use infrastructure::repositories::SqliteDocumentRepository;

fn get_db_path() -> PathBuf {
    let home = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
    let app_dir = home.join("doclib");
    std::fs::create_dir_all(&app_dir).ok();
    app_dir.join("documents.db")
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let db_path = get_db_path();
    let db = Database::new(&db_path).context("Failed to open database")?;
    infrastructure::database::initialize(&db.conn).context("Failed to initialize database")?;

    let repo = SqliteDocumentRepository::new(db.conn);
    let printer = CliPrinter::with_default_config();
    let deps = CliDependencies::new(repo, printer);

    cli::run(deps, cli)?;

    Ok(())
}