use anyhow::{Context, Result};
use rusqlite::Connection;
use std::path::Path;
use std::process::{Command, Stdio};

use crate::cache;
use crate::config::Config;
use crate::db;
use crate::ui;

pub fn run(conn: &Connection, cfg: &Config, query: &str) -> Result<()> {
    let Some(doc) = ui::resolve(conn, query)? else {
        return Ok(());
    };

    let path = cache::ensure_cached(conn, cfg, &doc)?;
    let opener = opener_for(cfg, &doc.ext);
    launch(&opener, &path)?;
    db::touch_opened(conn, &doc.id, &crate::now())?;

    println!("opened {:?} with {}", doc.title, opener);
    Ok(())
}

fn opener_for(cfg: &Config, ext: &str) -> String {
    if let Some(command) = cfg.openers.get(ext) {
        return command.clone();
    }
    if let Ok(command) = std::env::var("DOCLIB_OPENER")
        && !command.trim().is_empty()
    {
        return command;
    }
    "xdg-open".to_string()
}

/// Spawn detached: the reader outlives this process, so the shell returns
/// immediately instead of blocking until okular is closed.
fn launch(opener: &str, path: &Path) -> Result<()> {
    let mut parts = opener.split_whitespace();
    let program = parts.next().unwrap_or("xdg-open");
    let args: Vec<&str> = parts.collect();

    Command::new(program)
        .args(&args)
        .arg(path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .with_context(|| format!("launching {program}"))?;
    Ok(())
}
