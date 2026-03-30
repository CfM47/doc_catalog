#![allow(dead_code)]

use crate::application::repositories::DocumentRepository;

pub fn run<R: DocumentRepository>(_repo: &R) -> anyhow::Result<()> {
    println!("search command not implemented yet");
    Ok(())
}
