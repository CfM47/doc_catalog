use anyhow::{Context, Result, bail};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    /// Every folder the library is stored in. Order is read preference.
    #[serde(default)]
    pub stores: Vec<PathBuf>,

    /// Superseded by `stores`. Kept so configs written against the rclone
    /// version keep loading, though rclone remotes are no longer supported.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    remotes: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    remote: Option<OneOrMany>,

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
            stores: vec![default_store()],
            remotes: Vec::new(),
            remote: None,
            cache_max_bytes: 20 * 1024 * 1024 * 1024,
            openers,
            data_dir: PathBuf::new(),
        }
    }
}

/// `remote` was a single string, but people reasonably write a list once they
/// learn about several stores. Accept either rather than failing to parse.
#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
enum OneOrMany {
    One(String),
    Many(Vec<String>),
}

impl OneOrMany {
    fn into_vec(self) -> Vec<String> {
        match self {
            OneOrMany::One(value) => vec![value],
            OneOrMany::Many(values) => values,
        }
    }
}

fn project_dirs() -> Result<ProjectDirs> {
    ProjectDirs::from("", "", "doclib").context("cannot determine platform config directories")
}

pub fn config_path() -> Result<PathBuf> {
    Ok(project_dirs()?.config_dir().join("config.toml"))
}

pub fn data_dir() -> Result<PathBuf> {
    let dir = project_dirs()?.data_dir().to_path_buf();
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// Create the config directory and, on first run, a commented default file.
/// Deliberately does no parsing: `doclib config` has to be usable against a
/// file that does not parse, since that is how the file gets fixed.
pub fn ensure_file() -> Result<PathBuf> {
    let dirs = project_dirs()?;
    let config_file = dirs.config_dir().join("config.toml");
    fs::create_dir_all(dirs.config_dir())?;

    if !config_file.exists() {
        fs::write(&config_file, Config::default().to_toml()?)?;
        eprintln!("wrote default config to {}", config_file.display());
        eprintln!("storing documents in {}", default_store().display());
    }
    Ok(config_file)
}

/// Overwrite the config with the defaults, keeping the previous file as
/// `config.toml.bak` so a reset is never a one-way door.
pub fn reset_file() -> Result<(PathBuf, Option<PathBuf>)> {
    let config_file = ensure_file()?;
    let backup = config_file.with_extension("toml.bak");

    let saved = if config_file.exists() {
        fs::copy(&config_file, &backup)
            .with_context(|| format!("backing up {}", config_file.display()))?;
        Some(backup)
    } else {
        None
    };

    fs::write(&config_file, Config::default().to_toml()?)
        .with_context(|| format!("writing {}", config_file.display()))?;
    Ok((config_file, saved))
}

impl Config {
    /// Read config, writing a commented default on first run.
    pub fn load() -> Result<Config> {
        let config_file = ensure_file()?;
        let data_dir = data_dir()?;

        let text = fs::read_to_string(&config_file)
            .with_context(|| format!("reading {}", config_file.display()))?;
        let mut cfg: Config =
            toml::from_str(&text).with_context(|| format!("parsing {}", config_file.display()))?;
        cfg.data_dir = data_dir;
        cfg.absorb_legacy_keys();
        cfg.stores = cfg.stores.iter().map(|s| expand_home(s)).collect();
        Ok(cfg)
    }

    /// Configs written against the rclone version used `remotes` / `remote`.
    /// Carry those entries over as-is; anything of the form `name:path` was an
    /// rclone remote and is reported by `validate_stores` rather than silently
    /// dropped, so nobody loses a library to a quiet migration.
    fn absorb_legacy_keys(&mut self) {
        if !self.stores.is_empty() {
            self.remotes.clear();
            self.remote = None;
            return;
        }
        let legacy: Vec<String> = self
            .remotes
            .drain(..)
            .chain(
                self.remote
                    .take()
                    .map(OneOrMany::into_vec)
                    .unwrap_or_default(),
            )
            .filter(|value| !value.trim().is_empty())
            .collect();
        self.stores = legacy.into_iter().map(PathBuf::from).collect();
    }

    fn to_toml(&self) -> Result<String> {
        let body = toml::to_string_pretty(self)?;
        Ok(format!("{CONFIG_HEADER}\n{body}"))
    }

    pub fn validate_stores(&self) -> Result<()> {
        if self.stores.is_empty() {
            bail!(
                "no stores configured.\n\
                 set `stores` in {}\n\
                 for example: stores = [\"/home/you/library\", \"/mnt/usb/library\"]",
                config_path()?.display()
            );
        }
        for store in &self.stores {
            validate_store(store)?;
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

    /// The same in every store, since it is derived from the content hash.
    pub fn stored_path(&self, hash: &str, ext: &str) -> String {
        format!("{}/{}.{}", &hash[..2], hash, ext)
    }
}

/// Where a library lives unless told otherwise. Safe to default to: it is an
/// absolute path under the user's own home, so nothing can be written
/// somewhere surprising.
pub fn default_store() -> PathBuf {
    expand_home(Path::new("~/doclib"))
}

/// `~/library` is what people type; it is not a path the OS understands.
pub fn expand_home(path: &Path) -> PathBuf {
    let Ok(rest) = path.strip_prefix("~") else {
        return path.to_path_buf();
    };
    match std::env::var_os("HOME") {
        Some(home) => PathBuf::from(home).join(rest),
        None => path.to_path_buf(),
    }
}

/// A store must be an absolute path. A relative one resolves against the
/// current directory, so the same command run from two shells would use two
/// different libraries.
pub fn validate_store(store: &Path) -> Result<()> {
    let text = store.to_string_lossy();

    if text.trim().is_empty() {
        bail!("empty path in `stores`");
    }

    // Configs carried over from the rclone version may still hold "gdrive:lib".
    if let Some((name, _)) = text.split_once(':')
        && !name.contains('/')
    {
        bail!(
            "{text:?} looks like an rclone remote, which doclib no longer uses.\n\
             stores are plain folders now — mount the disk and use its path, \
             for example \"/mnt/usb/library\"."
        );
    }

    if !store.is_absolute() {
        bail!(
            "store {text:?} is a relative path, so where files go depends on the \
             directory you run doclib from.\n\
             use an absolute path, for example \"/home/you/library\"."
        );
    }
    Ok(())
}

const CONFIG_HEADER: &str = r#"# doclib configuration
#
# stores
#   Every folder the library is kept in. Documents are copied into all of
#   them; `doclib update` copies whatever is missing between them so they
#   converge. The first is tried first when reading.
#
#   A store is an ordinary directory — on this disk, on a mounted USB stick,
#   on an external drive, on a network share the system has already mounted.
#   Paths must be absolute; ~ is expanded.
#
#     stores = ["~/doclib"]                       the default
#     stores = ["~/doclib", "/mnt/usb/library"]   a copy on a USB disk too
#     stores = ["/run/media/you/BACKUP/library"]
#
#   Each store holds a .doclib-store marker file. That is how an unmounted
#   disk is told apart from an empty folder: without the marker doclib will
#   not write there, so a disconnected USB stick can never be mistaken for an
#   empty library.
#
# cache_max_bytes
#   Local cache ceiling. `doclib cache prune` evicts least-recently-opened
#   files above it; they are copied back from a store on the next open.
#   Set 0 to never evict.
#
# openers
#   Per-extension launcher. Anything unlisted falls back to $DOCLIB_OPENER,
#   then xdg-open.
"#;

#[cfg(test)]
mod tests {
    use super::*;

    fn with_stores(stores: &[&str]) -> Config {
        Config {
            stores: stores.iter().map(PathBuf::from).collect(),
            ..Default::default()
        }
    }

    #[test]
    fn accepts_absolute_paths() {
        for store in ["/home/you/library", "/mnt/usb/library", "/"] {
            assert!(
                validate_store(Path::new(store)).is_ok(),
                "rejected valid store {store:?}"
            );
        }
    }

    #[test]
    fn rejects_a_relative_path() {
        for store in ["library", "books/library", "./library", "../shared"] {
            assert!(
                validate_store(Path::new(store)).is_err(),
                "accepted relative path {store:?}"
            );
        }
    }

    #[test]
    fn explains_that_rclone_remotes_are_gone() {
        let message = format!(
            "{:#}",
            validate_store(Path::new("gdrive:library")).unwrap_err()
        );
        assert!(message.contains("rclone"), "got {message}");
        // A path containing a colon further along is still a path.
        assert!(validate_store(Path::new("/mnt/my:disk/library")).is_ok());
    }

    #[test]
    fn every_store_in_the_list_is_checked() {
        assert!(with_stores(&["/a", "/b"]).validate_stores().is_ok());
        assert!(with_stores(&["/a", "relative"]).validate_stores().is_err());
        assert!(with_stores(&["relative", "/a"]).validate_stores().is_err());
    }

    #[test]
    fn rejects_an_empty_list() {
        assert!(with_stores(&[]).validate_stores().is_err());
    }

    #[test]
    fn the_default_store_is_usable_without_being_edited() {
        // Unlike the rclone version, a default is safe here: it is an absolute
        // path in the user's own home, not a guess at someone's cloud account.
        let cfg = Config::default();
        assert_eq!(cfg.stores, vec![default_store()]);
        assert!(cfg.validate_stores().is_ok());
        assert!(default_store().is_absolute());
        assert!(default_store().ends_with("doclib"));
    }

    #[test]
    fn a_legacy_remote_path_becomes_a_store() {
        let mut cfg: Config =
            toml::from_str("remote = \"/home/you/library\"\ncache_max_bytes = 0\n\n[openers]\n")
                .unwrap();
        cfg.absorb_legacy_keys();
        assert_eq!(cfg.stores, vec![PathBuf::from("/home/you/library")]);
    }

    #[test]
    fn an_explicit_store_list_wins_over_legacy_keys() {
        let mut cfg: Config = toml::from_str(
            "remote = \"/old\"\nremotes = [\"/older\"]\nstores = [\"/new\"]\n\
             cache_max_bytes = 0\n\n[openers]\n",
        )
        .unwrap();
        cfg.absorb_legacy_keys();
        assert_eq!(cfg.stores, vec![PathBuf::from("/new")]);
    }

    #[test]
    fn expands_a_leading_tilde() {
        let home = std::env::var("HOME").unwrap_or_default();
        assert_eq!(
            expand_home(Path::new("~/library")),
            PathBuf::from(format!("{home}/library"))
        );
        // Only a leading tilde, and only as a whole component.
        assert_eq!(
            expand_home(Path::new("/mnt/~backup")),
            PathBuf::from("/mnt/~backup")
        );
    }
}
