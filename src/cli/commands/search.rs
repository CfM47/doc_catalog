#![allow(dead_code)]

use crate::application::repositories::DocumentRepository;
use crate::cli::dependencies::CliDependencies;

pub fn run<R: DocumentRepository + Clone>(_deps: CliDependencies<R>) -> anyhow::Result<()> {
    println!("search command not implemented yet");
    Ok(())
}
