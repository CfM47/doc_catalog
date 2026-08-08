use anyhow::{Context, Result, bail};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    /// Every remote the library is replicated to. Order is read preference.
    #[serde(default)]
    pub remotes: Vec<String>,

    /// Superseded by `remotes`, still read so older configs keep working.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    remote: Option<String>,

    /// Cache eviction ceiling in bytes. 0 disables pruning.
    pub cache_max_bytes: u64,
    /// Per-extension opener override. Falls back to $DOCLIB_OPENER, then xdg-open.
    pub openers: BTreeMap<String, String>,

    #[serde(skip)]
    pub data_dir: PathBuf,
}

impl Default for Config {
    fn default() -> Self {
        let mut openers = BTreeMap::new();
        openers.insert("pdf".to_string(), "okular".to_string());
        Config {
            // Deliberately empty. Any guess here would be wrong for most
            // people, and a wrong-looking value that happens to parse is worse
            // than none: `remotes = ["books"]` silently uploads to ./books.
            remotes: Vec::new(),
            remote: None,
            cache_max_bytes: 20 * 1024 * 1024 * 1024,
            openers,
            data_dir: PathBuf::new(),
        }
    }
}

fn project_dirs() -> Result<ProjectDirs> {
    ProjectDirs::from("", "", "doclib").context("cannot determine platform config directories")
}

pub fn config_path() -> Result<PathBuf> {
    Ok(project_dirs()?.config_dir().join("config.toml"))
}

impl Config {
    /// Read config, writing a commented default on first run.
    pub fn load() -> Result<Config> {
        let dirs = project_dirs()?;
        let config_file = dirs.config_dir().join("config.toml");
        let data_dir = dirs.data_dir().to_path_buf();

        fs::create_dir_all(dirs.config_dir())?;
        fs::create_dir_all(&data_dir)?;

        if !config_file.exists() {
            let default = Config::default();
            fs::write(&config_file, default.to_toml()?)?;
            eprintln!("wrote default config to {}", config_file.display());
            eprintln!("set `remotes` before importing — see the comments in that file.");
        }

        let text = fs::read_to_string(&config_file)
            .with_context(|| format!("reading {}", config_file.display()))?;
        let mut cfg: Config =
            toml::from_str(&text).with_context(|| format!("parsing {}", config_file.display()))?;
        cfg.data_dir = data_dir;
        cfg.absorb_legacy_remote();
        Ok(cfg)
    }

    /// A config written before multi-remote support has `remote = "..."`.
    /// Treat it as a one-element list rather than making the user edit.
    fn absorb_legacy_remote(&mut self) {
        if let Some(legacy) = self.remote.take()
            && self.remotes.is_empty()
            && !legacy.trim().is_empty()
        {
            self.remotes.push(legacy);
        }
    }

    fn to_toml(&self) -> Result<String> {
        let body = toml::to_string_pretty(self)?;
        Ok(format!("{CONFIG_HEADER}\n{body}"))
    }

    pub fn validate_remotes(&self) -> Result<()> {
        if self.remotes.is_empty() {
            bail!(
                "no remotes configured.\n\
                 set `remotes` in {}\n\
                 list your configured rclone remotes with `rclone listremotes`.",
                config_path()?.display()
            );
        }
        for remote in &self.remotes {
            validate_remote(remote)?;
        }
        Ok(())
    }

    pub fn db_path(&self) -> PathBuf {
        self.data_dir.join("catalog.db")
    }

    pub fn cache_dir(&self) -> PathBuf {
        self.data_dir.join("cache")
    }

    /// Content-addressed cache location. Two-char shard keeps directories small.
    pub fn cache_path(&self, hash: &str, ext: &str) -> PathBuf {
        self.cache_dir()
            .join(&hash[..2])
            .join(format!("{hash}.{ext}"))
    }

    /// The same on every remote, since it is derived from the content hash.
    pub fn remote_path(&self, hash: &str, ext: &str) -> String {
        format!("{}/{}.{}", &hash[..2], hash, ext)
    }
}

/// Reject a remote that rclone would accept but that does not mean what the
/// user thinks. Anything without a colon is a local filesystem path, and a
/// relative one resolves against the current directory — so the same command
/// run from two shells would use two different stores.
pub fn validate_remote(remote: &str) -> Result<()> {
    let remote = remote.trim();

    if remote.is_empty() {
        bail!("empty remote in `remotes`");
    }

    // ":s3:bucket" and friends are rclone connection strings: a full backend
    // definition inline, no configured remote needed.
    if remote.starts_with(':') || remote.starts_with('/') {
        return Ok(());
    }

    // A colon before the first slash means "configured remote".
    let head = remote.split('/').next().unwrap_or(remote);
    if head.contains(':') {
        return Ok(());
    }

    bail!(
        "remote {remote:?} is a relative local path, so where files go depends on \
         the directory you run doclib from.\n\
         use a configured rclone remote (\"{remote}:doclib\", if that is the remote's \
         name) or an absolute path (\"/home/you/{remote}\")."
    );
}

const CONFIG_HEADER: &str = r#"# doclib configuration
#
# remotes
#   Every place the library is stored, in rclone's own syntax. Documents are
#   uploaded to all of them; `doclib update` copies whatever is missing between
#   them so they converge. The first is tried first when reading.
#
#   Run `rclone listremotes` to see what you have configured, and
#   `rclone config` to add one. Any backend rclone supports works:
#
#     remotes = ["gdrive:doclib"]                   one configured remote
#     remotes = ["gdrive:doclib", "/mnt/usb/lib"]   cloud plus a local disk
#     remotes = ["b2:my-bucket/doclib"]             bucket storage with a path
#     remotes = [":s3:my-bucket"]                   an inline connection string
#
#   A bare name with no colon ("doclib") is NOT a remote — rclone reads it as a
#   relative local directory, so the destination would follow your shell's
#   working directory. doclib refuses that.
#
# cache_max_bytes
#   Local cache ceiling. `doclib cache prune` evicts least-recently-opened
#   files above it; they are re-fetched from a remote on the next open.
#   Set 0 to never evict.
#
# openers
#   Per-extension launcher. Anything unlisted falls back to $DOCLIB_OPENER,
#   then xdg-open.
"#;

#[cfg(test)]
mod tests {
    use super::*;

    fn with_remotes(remotes: &[&str]) -> Config {
        Config {
            remotes: remotes.iter().map(|r| r.to_string()).collect(),
            ..Default::default()
        }
    }

    #[test]
    fn accepts_every_shape_of_configured_remote() {
        for remote in [
            "gdrive:doclib",
            "dropbox:books",
            "b2:my-bucket/doclib",
            "onedrive:",
            ":s3:my-bucket/doclib",
            "/mnt/usb/doclib",
        ] {
            assert!(
                validate_remote(remote).is_ok(),
                "rejected valid remote {remote:?}"
            );
        }
    }

    #[test]
    fn rejects_a_relative_path_masquerading_as_a_remote() {
        // The trap: rclone accepts these, but resolves them against the
        // working directory rather than any configured backend.
        for remote in ["doclib", "books/doclib", "./doclib", "../shared"] {
            assert!(
                validate_remote(remote).is_err(),
                "accepted relative path {remote:?}"
            );
        }
    }

    #[test]
    fn every_remote_in_the_list_is_checked() {
        assert!(
            with_remotes(&["gdrive:doclib", "/mnt/usb"])
                .validate_remotes()
                .is_ok()
        );
        // One bad entry invalidates the config, wherever it sits.
        assert!(
            with_remotes(&["gdrive:doclib", "books"])
                .validate_remotes()
                .is_err()
        );
        assert!(
            with_remotes(&["books", "gdrive:doclib"])
                .validate_remotes()
                .is_err()
        );
    }

    #[test]
    fn rejects_an_empty_list() {
        assert!(with_remotes(&[]).validate_remotes().is_err());
        assert!(Config::default().validate_remotes().is_err());
    }

    #[test]
    fn a_legacy_single_remote_becomes_a_one_element_list() {
        let mut cfg: Config =
            toml::from_str("remote = \"gdrive:doclib\"\ncache_max_bytes = 0\n\n[openers]\n")
                .unwrap();
        cfg.absorb_legacy_remote();
        assert_eq!(cfg.remotes, vec!["gdrive:doclib"]);
    }

    #[test]
    fn an_explicit_list_wins_over_the_legacy_key() {
        let mut cfg: Config = toml::from_str(
            "remote = \"old:x\"\nremotes = [\"new:y\"]\ncache_max_bytes = 0\n\n[openers]\n",
        )
        .unwrap();
        cfg.absorb_legacy_remote();
        assert_eq!(cfg.remotes, vec!["new:y"]);
    }
}
