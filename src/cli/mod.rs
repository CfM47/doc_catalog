pub mod commands;

#[cfg(test)]
mod tests;

use crate::application::repositories::DocumentRepository;
use clap::{Parser, Subcommand};

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

pub fn run<R: DocumentRepository>(repo: &R, cli: Cli) -> anyhow::Result<()> {
    match cli.command {
        Commands::Add => commands::add::run(repo),
        Commands::List => commands::list::run(repo),
        Commands::Search => commands::search::run(repo),
    }
}
