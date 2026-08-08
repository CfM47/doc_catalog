use anyhow::{Context, Result, bail};
use std::path::Path;
use std::process::Command;

use crate::config::{self, Config};

pub fn run(cfg: &Config, remote: Option<&str>, show: bool, path_only: bool) -> Result<()> {
    let file = config::config_path()?;

    if path_only {
        println!("{}", file.display());
        return Ok(());
    }
    if let Some(remote) = remote {
        return set_remote(&file, remote);
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
    println!(
        "remote  {}",
        if cfg.remote.is_empty() {
            "(unset)"
        } else {
            &cfg.remote
        }
    );
    match cfg.validate_remote() {
        Ok(()) => println!("\nremote looks usable."),
        Err(e) => println!("\n{e:#}"),
    }
    Ok(())
}

/// Validate before writing, so a typo is rejected while the old value is still
/// in place rather than after it has been overwritten.
fn set_remote(file: &Path, remote: &str) -> Result<()> {
    let candidate = Config {
        remote: remote.to_string(),
        ..Default::default()
    };
    candidate.validate_remote()?;

    let text =
        std::fs::read_to_string(file).with_context(|| format!("reading {}", file.display()))?;
    let quoted = toml::Value::String(remote.to_string()).to_string();

    // Rewrite the one line in place: serializing the whole document back would
    // discard the explanatory comments and anything the user added.
    let mut replaced = false;
    let mut out: Vec<String> = Vec::new();
    for line in text.lines() {
        if !replaced && is_remote_assignment(line) {
            out.push(format!("remote = {quoted}"));
            replaced = true;
        } else {
            out.push(line.to_string());
        }
    }
    if !replaced {
        out.push(format!("remote = {quoted}"));
    }

    std::fs::write(file, format!("{}\n", out.join("\n")))
        .with_context(|| format!("writing {}", file.display()))?;

    println!("remote = {remote}");
    Ok(())
}

/// Matches an actual `remote = ...` assignment, not the commented examples in
/// the file header.
fn is_remote_assignment(line: &str) -> bool {
    let trimmed = line.trim_start();
    if trimmed.starts_with('#') {
        return false;
    }
    match trimmed.strip_prefix("remote") {
        Some(rest) => rest.trim_start().starts_with('='),
        None => false,
    }
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
