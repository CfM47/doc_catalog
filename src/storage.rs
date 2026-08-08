//! Everything that knows rclone exists lives here. Swapping in a native S3
//! client later means rewriting this file and nothing else.

use anyhow::{Context, Result, bail};
use std::path::Path;
use std::process::{Command, Stdio};

use crate::config::Config;

/// Verify the whole setup before any work begins: the binary is installed, the
/// configured remote is shaped like a remote, and rclone can actually resolve
/// it. The last check is the one that catches `local:doclib` when no `[local]`
/// section exists — a string that looks perfectly valid until it is used.
pub fn check_ready(cfg: &Config) -> Result<()> {
    check_available()?;
    cfg.validate_remote()?;
    check_remote(cfg)
}

/// Names of the remotes in the user's rclone config, without trailing colons.
pub fn list_remotes() -> Result<Vec<String>> {
    let out = run(&["listremotes"])?;
    if !out.status.success() {
        return Ok(Vec::new());
    }
    Ok(String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|line| line.trim().trim_end_matches(':').to_string())
        .filter(|name| !name.is_empty())
        .collect())
}

/// Ask rclone to resolve the remote. A missing *path* is fine — a freshly
/// configured remote holds nothing yet — but a missing *backend* is fatal.
pub fn check_remote(cfg: &Config) -> Result<()> {
    let target = cfg.remote.trim_end_matches('/');
    let out = run(&["lsjson", "--stat", target])?;
    if out.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&out.stderr);
    if describes_missing_path(&stderr) {
        return Ok(());
    }

    if let Some(name) = unknown_remote_name(&stderr) {
        let configured = list_remotes().unwrap_or_default();
        let known = if configured.is_empty() {
            "none are configured — run `rclone config` to add one".to_string()
        } else {
            format!(
                "configured remotes: {}",
                configured
                    .iter()
                    .map(|r| format!("{r}:"))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        };
        bail!(
            "rclone has no remote named {name:?}, so {target:?} cannot be reached.\n\
             {known}\n\
             set it with `doclib config --remote <remote>`, or use an absolute path."
        );
    }

    bail!("rclone cannot reach {target:?}: {}", stderr.trim());
}

/// The path is absent but the backend resolved.
fn describes_missing_path(stderr: &str) -> bool {
    let lower = stderr.to_ascii_lowercase();
    lower.contains("directory not found")
        || lower.contains("path not found")
        || lower.contains("object not found")
        || lower.contains("no such file or directory")
}

/// rclone reports an unconfigured remote as:
///   didn't find section in config file ("local")
fn unknown_remote_name(stderr: &str) -> Option<String> {
    let lower = stderr.to_ascii_lowercase();
    if !lower.contains("didn't find section in config file") && !lower.contains("unknown backend") {
        return None;
    }
    // Prefer the quoted name rclone gives; fall back to a generic message.
    let start = stderr.rfind('(')?;
    let quoted = stderr[start..].trim_start_matches('(');
    let name = quoted
        .trim_start_matches('"')
        .split('"')
        .next()?
        .trim()
        .to_string();
    (!name.is_empty()).then_some(name)
}

/// Fail loudly at startup rather than halfway through a 400-file import.
pub fn check_available() -> Result<()> {
    let status = Command::new("rclone")
        .arg("version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    match status {
        Ok(s) if s.success() => Ok(()),
        Ok(s) => bail!("`rclone version` exited with {s}"),
        Err(_) => bail!(
            "rclone not found on PATH.\n\
             install it (https://rclone.org/install/) and configure a remote with `rclone config`."
        ),
    }
}

fn remote_url(cfg: &Config, remote_rel: &str) -> String {
    format!("{}/{}", cfg.remote.trim_end_matches('/'), remote_rel)
}

fn run(args: &[&str]) -> Result<std::process::Output> {
    let out = Command::new("rclone")
        .args(args)
        .output()
        .context("spawning rclone")?;
    Ok(out)
}

pub fn upload(cfg: &Config, local: &Path, remote_rel: &str) -> Result<()> {
    let dest = remote_url(cfg, remote_rel);
    let local = local.to_string_lossy();
    let out = run(&["copyto", &local, &dest])?;
    if !out.status.success() {
        bail!(
            "rclone copyto {local} -> {dest} failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(())
}

pub fn download(cfg: &Config, remote_rel: &str, dest: &Path) -> Result<()> {
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let src = remote_url(cfg, remote_rel);
    let dest_s = dest.to_string_lossy();
    let out = run(&["copyto", &src, &dest_s])?;
    if !out.status.success() {
        bail!(
            "rclone copyto {src} -> {dest_s} failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(())
}

/// Every stored file, as (path relative to the remote root, size in bytes).
pub fn list(cfg: &Config) -> Result<Vec<(String, u64)>> {
    let root = cfg.remote.trim_end_matches('/').to_string();
    let out = run(&[
        "lsf",
        "-R",
        "--files-only",
        "--format",
        "ps",
        "--separator",
        "\t",
        &root,
    ])?;
    if !out.status.success() {
        bail!(
            "rclone lsf {root} failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }

    let text = String::from_utf8_lossy(&out.stdout);
    Ok(text
        .lines()
        .filter_map(|line| {
            let (path, size) = line.split_once('\t')?;
            Some((path.to_string(), size.trim().parse().unwrap_or(0)))
        })
        .collect())
}

pub fn delete(cfg: &Config, remote_rel: &str) -> Result<()> {
    let target = remote_url(cfg, remote_rel);
    let out = run(&["deletefile", &target])?;
    if !out.status.success() {
        bail!(
            "rclone deletefile {target} failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(())
}

pub fn exists(cfg: &Config, remote_rel: &str) -> Result<bool> {
    let target = remote_url(cfg, remote_rel);
    let out = run(&["lsjson", "--stat", &target])?;
    Ok(out.status.success())
}

#[cfg(test)]
mod tests {
    use super::*;

    const UNKNOWN: &str = r#"2026/08/08 13:01:10 CRITICAL: Failed to create file system for "local:doclib": didn't find section in config file ("local")"#;

    #[test]
    fn extracts_the_name_of_an_unconfigured_remote() {
        assert_eq!(unknown_remote_name(UNKNOWN).as_deref(), Some("local"));
    }

    #[test]
    fn an_absent_path_is_not_an_unknown_remote() {
        // A correctly configured remote that simply holds nothing yet must not
        // be reported as broken.
        let absent = "2026/08/08 13:01:10 ERROR : : error listing: directory not found";
        assert!(describes_missing_path(absent));
        assert!(unknown_remote_name(absent).is_none());
    }

    #[test]
    fn an_unknown_remote_is_not_an_absent_path() {
        assert!(!describes_missing_path(UNKNOWN));
    }
}
