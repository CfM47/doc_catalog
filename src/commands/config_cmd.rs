use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::config::{self, Config};
use crate::storage;

pub fn run(cfg: &Config, stores: &[PathBuf], show: bool, path_only: bool) -> Result<()> {
    let file = config::config_path()?;

    if path_only {
        println!("{}", file.display());
        return Ok(());
    }
    if !stores.is_empty() {
        return set_stores(&file, stores);
    }
    if show {
        return status(cfg);
    }

    edit(&file)?;
    // Re-read rather than trusting the in-memory copy: the point of opening an
    // editor is that the file on disk changed.
    status(&Config::load()?)
}

fn status(cfg: &Config) -> Result<()> {
    println!("config  {}", config::config_path()?.display());
    println!("data    {}", cfg.data_dir.display());
    if cfg.stores.is_empty() {
        println!("stores  (none)");
    } else {
        println!("stores");
    }
    for store in &cfg.stores {
        // Being a valid path is only half the answer: the disk also has to be
        // there, carrying the marker that says it really is our store.
        let state = match config::validate_store(store) {
            Err(e) => first_line(&e),
            Ok(()) => match storage::check_store(store) {
                Ok(()) => "available".to_string(),
                Err(e) => first_line(&e),
            },
        };
        println!("  {}  —  {state}", store.display());
    }

    if let Err(e) = cfg.validate_stores() {
        println!(
            "
{e:#}"
        );
    }
    Ok(())
}

fn first_line(error: &anyhow::Error) -> String {
    format!("{error:#}")
        .lines()
        .next()
        .unwrap_or("unavailable")
        .to_string()
}

/// Validate before writing, so a typo is rejected while the old value is still
/// in place rather than after it has been overwritten.
fn set_stores(file: &Path, stores: &[PathBuf]) -> Result<()> {
    let stores: Vec<PathBuf> = stores.iter().map(|s| config::expand_home(s)).collect();
    for store in &stores {
        config::validate_store(store)?;
    }

    let text =
        std::fs::read_to_string(file).with_context(|| format!("reading {}", file.display()))?;
    let quoted = toml::Value::Array(
        stores
            .iter()
            .map(|s| toml::Value::String(s.display().to_string()))
            .collect(),
    )
    .to_string();

    // Rewrite the one line in place: serializing the whole document back would
    // discard the explanatory comments and anything the user added.
    let mut replaced = false;
    let mut out: Vec<String> = Vec::new();
    for line in text.lines() {
        if !replaced && is_remote_assignment(line) {
            out.push(format!("stores = {quoted}"));
            replaced = true;
        } else {
            out.push(line.to_string());
        }
    }
    if !replaced {
        out.push(format!("stores = {quoted}"));
    }

    std::fs::write(file, format!("{}\n", out.join("\n")))
        .with_context(|| format!("writing {}", file.display()))?;

    println!("stores = {quoted}");

    // Create each store now so the marker exists and the folder is usable. A
    // disconnected disk is a warning, not a failure — configuring it before
    // plugging it in is a reasonable order to do things in.
    for store in &stores {
        match storage::init_store(store) {
            Ok(()) => println!("  {} ready", store.display()),
            Err(e) => eprintln!("  warning: {e:#}"),
        }
    }
    Ok(())
}

/// Matches an actual `remote = ...` assignment, not the commented examples in
/// the file header.
fn is_remote_assignment(line: &str) -> bool {
    let trimmed = line.trim_start();
    if trimmed.starts_with('#') {
        return false;
    }
    // Both the current `remotes` key and the legacy `remote` one, so setting a
    // value replaces whichever the file happens to use.
    for key in ["stores", "remotes", "remote"] {
        if let Some(rest) = trimmed.strip_prefix(key)
            && rest.trim_start().starts_with('=')
        {
            return true;
        }
    }
    false
}

fn edit(file: &Path) -> Result<()> {
    let editor = std::env::var("VISUAL")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .or_else(|| {
            std::env::var("EDITOR")
                .ok()
                .filter(|v| !v.trim().is_empty())
        })
        .unwrap_or_else(|| "vi".to_string());

    // $EDITOR may carry arguments, e.g. "code --wait" or "emacs -nw".
    let mut parts = editor.split_whitespace();
    let program = parts.next().unwrap_or("vi");
    let args: Vec<&str> = parts.collect();

    let status = Command::new(program)
        .args(&args)
        .arg(file)
        .status()
        .with_context(|| {
            format!("launching editor {program:?} — set $EDITOR or use `doclib config --remote`")
        })?;

    if !status.success() {
        bail!("editor {program:?} exited with {status}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognises_a_real_assignment() {
        assert!(is_remote_assignment("stores = [\"/home/you/library\"]"));
        assert!(is_remote_assignment("remotes = [\"gdrive:doclib\"]"));
        assert!(is_remote_assignment("remote = \"gdrive:doclib\""));
        assert!(is_remote_assignment("remote=\"x\""));
        assert!(is_remote_assignment("  remote  =  \"x\""));
    }

    #[test]
    fn ignores_commented_examples_and_other_keys() {
        // The header documents `remote = "gdrive:doclib"` inside a comment;
        // rewriting that line instead of the real one would silently do nothing.
        assert!(!is_remote_assignment("#     remote = \"gdrive:doclib\""));
        assert!(!is_remote_assignment("# remote"));
        assert!(!is_remote_assignment("remote_path = \"x\""));
        assert!(!is_remote_assignment("cache_max_bytes = 1"));
    }
}
