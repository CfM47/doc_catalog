pub mod commands;
pub mod dependencies;
pub mod printer;
pub mod utils;

#[cfg(test)]
mod tests;

use crate::application::repositories::DocumentRepository;
use clap::{Parser, Subcommand};
use dependencies::CliDependencies;

#[derive(Parser)]
#[command(name = "doclib")]
#[command(about = "Personal document catalog CLI", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    Add,
    List,
    Search,
}

pub fn run<R: DocumentRepository + Clone>(
    deps: CliDependencies<R>,
    cli: Cli,
) -> anyhow::Result<()> {
    match cli.command {
        Commands::Add => commands::add::run(deps),
        Commands::List => commands::list::run(deps),
        Commands::Search => commands::search::run(deps),
    }
}
