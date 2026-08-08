use anyhow::{Result, bail};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Book,
    Article,
}

impl Kind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Kind::Book => "book",
            Kind::Article => "article",
        }
    }

    pub fn parse(s: &str) -> Result<Kind> {
        match s {
            "book" => Ok(Kind::Book),
            "article" => Ok(Kind::Article),
            other => bail!("unknown document kind: {other}"),
        }
    }
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A catalogued file. Identity is `id` (stable). `content_hash` is the current
/// bytes: used for dedupe on import and integrity checks on sync, never as
/// identity.
#[derive(Debug, Clone)]
pub struct Document {
    pub id: String,
    pub kind: Kind,
    pub title: String,
    pub authors: Option<String>,
    pub year: Option<i64>,

    // book
    pub publisher: Option<String>,
    pub edition: Option<String>,
    pub isbn: Option<String>,

    // article
    pub journal: Option<String>,
    pub volume: Option<String>,
    pub issue: Option<String>,
    pub pages: Option<String>,
    pub doi: Option<String>,

    pub ext: String,
    pub size: i64,
    pub content_hash: String,
    pub remote_path: String,

    pub cached_at: Option<String>,
    pub last_opened: Option<String>,
    pub added_at: String,
    pub extra: Option<String>,
}

impl Document {
    /// Byline for list output: authors if known, else a dash.
    pub fn byline(&self) -> &str {
        self.authors.as_deref().unwrap_or("-")
    }
}
