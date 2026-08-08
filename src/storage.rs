//! Everything that knows rclone exists lives here. Swapping in a native S3
//! client later means rewriting this file and nothing else.

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
