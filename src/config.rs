use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    /// rclone remote, including any subpath. e.g. "gdrive:doclib"
    pub remote: String,
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
            remote: "gdrive:doclib".to_string(),
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
            eprintln!("set `remote` to your rclone remote before importing.");
        }

        let text = fs::read_to_string(&config_file)
            .with_context(|| format!("reading {}", config_file.display()))?;
        let mut cfg: Config =
            toml::from_str(&text).with_context(|| format!("parsing {}", config_file.display()))?;
        cfg.data_dir = data_dir;
        Ok(cfg)
    }

    fn to_toml(&self) -> Result<String> {
        let body = toml::to_string_pretty(self)?;
        Ok(format!(
            "# doclib configuration\n\
             #\n\
             # remote:          rclone remote and optional subpath, e.g. \"gdrive:doclib\"\n\
             # cache_max_bytes: local cache ceiling; `doclib cache prune` evicts LRU above it\n\
             # openers:         per-extension launcher, otherwise xdg-open\n\n{body}"
        ))
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

    /// Remote layout mirrors the cache so a bare `rclone sync` stays meaningful.
    pub fn remote_path(&self, hash: &str, ext: &str) -> String {
        format!("{}/{}.{}", &hash[..2], hash, ext)
    }
}
