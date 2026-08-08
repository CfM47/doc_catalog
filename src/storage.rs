//! Everything that knows rclone exists lives here. Swapping in a native S3
//! client later means rewriting this file and nothing else.
//!
//! Every function takes an explicit remote string. The library may be
//! replicated across several, and no function should have an opinion about
//! which one it is working on.

use anyhow::{Context, Result, bail};
use std::path::Path;
use std::process::{Command, Stdio};

use crate::config::Config;

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

/// Which remotes can actually be reached right now.
pub struct Reachability {
    pub usable: Vec<String>,
    pub unusable: Vec<(String, String)>,
}

/// Verify the whole setup before any work begins. With several remotes a
/// single outage must not block the tool, so this fails only when *none* is
/// reachable; degraded remotes are reported to the caller.
pub fn check_ready(cfg: &Config) -> Result<Reachability> {
    check_available()?;
    cfg.validate_remotes()?;

    let mut usable = Vec::new();
    let mut unusable = Vec::new();

    for remote in &cfg.remotes {
        match check_remote(remote) {
            Ok(()) => usable.push(remote.clone()),
            Err(e) => unusable.push((remote.clone(), format!("{e:#}"))),
        }
    }

    if usable.is_empty() {
        let detail = unusable
            .iter()
            .map(|(remote, why)| format!("  {remote}: {why}"))
            .collect::<Vec<_>>()
            .join("\n");
        bail!("no remote is reachable:\n{detail}");
    }

    for (remote, why) in &unusable {
        eprintln!("warning: {remote} is unreachable: {why}");
    }
    Ok(Reachability { usable, unusable })
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

/// Ask rclone to resolve a remote. A missing *path* is fine — a freshly
/// configured remote holds nothing yet — but a missing *backend* is fatal.
pub fn check_remote(remote: &str) -> Result<()> {
    let target = remote.trim_end_matches('/');
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
             {known}"
        );
    }

    bail!("rclone cannot reach {target:?}: {}", stderr.trim());
}

fn remote_url(remote: &str, remote_rel: &str) -> String {
    format!("{}/{}", remote.trim_end_matches('/'), remote_rel)
}

fn run(args: &[&str]) -> Result<std::process::Output> {
    Command::new("rclone")
        .args(args)
        .output()
        .context("spawning rclone")
}

pub fn upload(remote: &str, local: &Path, remote_rel: &str) -> Result<()> {
    let dest = remote_url(remote, remote_rel);
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

pub fn download(remote: &str, remote_rel: &str, dest: &Path) -> Result<()> {
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let src = remote_url(remote, remote_rel);
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

/// Copy one stored file straight from one remote to another. Server-side where
/// the backend supports it, so replication never round-trips through the disk.
pub fn copy_between(src_remote: &str, dst_remote: &str, remote_rel: &str) -> Result<()> {
    let src = remote_url(src_remote, remote_rel);
    let dst = remote_url(dst_remote, remote_rel);
    let out = run(&["copyto", &src, &dst])?;
    if !out.status.success() {
        bail!(
            "rclone copyto {src} -> {dst} failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(())
}

pub fn delete(remote: &str, remote_rel: &str) -> Result<()> {
    let target = remote_url(remote, remote_rel);
    let out = run(&["deletefile", "--retries", "1", &target])?;
    if !out.status.success() {
        bail!(
            "rclone deletefile {target} failed: {}",
            first_error(&out.stderr)
        );
    }
    Ok(())
}

/// Every stored file on one remote, as (path relative to its root, bytes).
/// Filtered to this tool's layout: a remote may hold other things.
pub fn list(remote: &str) -> Result<Vec<(String, u64)>> {
    let root = remote.trim_end_matches('/');
    let out = run(&[
        "lsf",
        "-R",
        "--files-only",
        "--format",
        "ps",
        "--separator",
        "\t",
        root,
    ])?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        if describes_missing_path(&stderr) {
            return Ok(Vec::new());
        }
        bail!("rclone lsf {root} failed: {}", stderr.trim());
    }

    let text = String::from_utf8_lossy(&out.stdout);
    Ok(text
        .lines()
        .filter_map(|line| {
            let (path, size) = line.split_once('\t')?;
            is_doclib_file(path).then(|| (path.to_string(), size.trim().parse().unwrap_or(0)))
        })
        .collect())
}

pub fn exists(remote: &str, remote_rel: &str) -> Result<bool> {
    let target = remote_url(remote, remote_rel);
    let out = run(&["lsjson", "--stat", &target])?;
    Ok(out.status.success())
}

/// Only touch files this tool wrote: `<two hex>/<64 hex>.<ext>`. A remote
/// pointed at a shared folder may hold things that are none of our business.
pub fn is_doclib_file(path: &str) -> bool {
    let Some((shard, name)) = path.split_once('/') else {
        return false;
    };
    let Some((hash, ext)) = name.rsplit_once('.') else {
        return false;
    };
    let hex = |s: &str| s.chars().all(|c| c.is_ascii_hexdigit());

    shard.len() == 2
        && hex(shard)
        && hash.len() == 64
        && hex(hash)
        && hash.starts_with(shard)
        && !ext.is_empty()
        && ext.chars().all(|c| c.is_ascii_alphanumeric())
}

/// rclone logs one line per retry and per error; the first is the decisive
/// one, and the rest just bury it.
fn first_error(stderr: &[u8]) -> String {
    let text = String::from_utf8_lossy(stderr);
    text.lines()
        .find(|line| line.contains("ERROR") || line.contains("CRITICAL"))
        .or_else(|| text.lines().next())
        .unwrap_or("")
        .trim()
        .to_string()
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

    #[test]
    fn recognises_files_this_tool_wrote() {
        let hash = "920c622478f05bf785b0549899ce6e670dbc1c9ba5d3699a718f07d31d4a6105";
        assert!(is_doclib_file(&format!("92/{hash}.pdf")));
        assert!(is_doclib_file(&format!("92/{hash}.epub")));
    }

    #[test]
    fn leaves_everything_else_alone() {
        let hash = "920c622478f05bf785b0549899ce6e670dbc1c9ba5d3699a718f07d31d4a6105";
        // Someone else's files sharing the remote must survive.
        assert!(!is_doclib_file("holiday-photos/beach.jpg"));
        assert!(!is_doclib_file("notes.txt"));
        assert!(!is_doclib_file("92/not-a-hash.pdf"));
        assert!(!is_doclib_file(&format!("{hash}.pdf")));
        // Shard must match the hash it claims to shard.
        assert!(!is_doclib_file(&format!("ab/{hash}.pdf")));
    }
}
