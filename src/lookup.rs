//! Online metadata lookup: one identifier in, a full record out. Failures are
//! never fatal — the import prompt just starts from less.

use anyhow::Result;
use serde_json::Value;
use std::time::Duration;

use crate::metadata::Extracted;

fn client() -> Result<reqwest::blocking::Client> {
    Ok(reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(15))
        .user_agent("doclib/0.1 (personal library indexer)")
        .build()?)
}

fn text_at(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// OpenLibrary, keyed by ISBN.
pub fn by_isbn(isbn: &str) -> Result<Option<Extracted>> {
    let isbn: String = isbn.chars().filter(|c| c.is_ascii_alphanumeric()).collect();
    let url =
        format!("https://openlibrary.org/api/books?bibkeys=ISBN:{isbn}&jscmd=data&format=json");
    let body: Value = client()?.get(url).send()?.json()?;
    let record = match body.get(format!("ISBN:{isbn}")) {
        Some(r) => r,
        None => return Ok(None),
    };

    let authors = record
        .get("authors")
        .and_then(|a| a.as_array())
        .map(|list| {
            list.iter()
                .filter_map(|a| text_at(a, "name"))
                .collect::<Vec<_>>()
                .join(", ")
        });
    let publisher = record
        .get("publishers")
        .and_then(|p| p.as_array())
        .and_then(|list| list.first())
        .and_then(|p| text_at(p, "name"));
    let year = text_at(record, "publish_date").and_then(|d| parse_year(&d));

    Ok(Some(Extracted {
        title: text_at(record, "title"),
        authors: authors.filter(|s| !s.is_empty()),
        publisher,
        year,
        isbn: Some(isbn),
        ..Default::default()
    }))
}

/// Crossref, keyed by DOI. Returns a complete article record in one call.
pub fn by_doi(doi: &str) -> Result<Option<Extracted>> {
    let doi = doi.trim().trim_start_matches("https://doi.org/");
    let url = format!("https://api.crossref.org/works/{doi}");
    let response = client()?.get(url).send()?;
    if !response.status().is_success() {
        return Ok(None);
    }
    let body: Value = response.json()?;
    let work = match body.get("message") {
        Some(m) => m,
        None => return Ok(None),
    };

    let title = work
        .get("title")
        .and_then(|t| t.as_array())
        .and_then(|list| list.first())
        .and_then(|t| t.as_str())
        .map(|s| s.trim().to_string());

    let authors = work.get("author").and_then(|a| a.as_array()).map(|list| {
        list.iter()
            .map(|a| {
                let given = text_at(a, "given").unwrap_or_default();
                let family = text_at(a, "family").unwrap_or_default();
                format!("{given} {family}").trim().to_string()
            })
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(", ")
    });

    let journal = work
        .get("container-title")
        .and_then(|t| t.as_array())
        .and_then(|list| list.first())
        .and_then(|t| t.as_str())
        .map(|s| s.trim().to_string());

    let year = work
        .get("issued")
        .and_then(|i| i.get("date-parts"))
        .and_then(|d| d.as_array())
        .and_then(|list| list.first())
        .and_then(|d| d.as_array())
        .and_then(|parts| parts.first())
        .and_then(|y| y.as_i64());

    Ok(Some(Extracted {
        title,
        authors: authors.filter(|s| !s.is_empty()),
        journal,
        volume: text_at(work, "volume"),
        issue: text_at(work, "issue"),
        pages: text_at(work, "page"),
        year,
        doi: Some(doi.to_string()),
        publisher: text_at(work, "publisher"),
        ..Default::default()
    }))
}

/// OpenLibrary publish dates are free text: "2001", "March 2001", "c1998".
fn parse_year(text: &str) -> Option<i64> {
    let mut digits = String::new();
    for c in text.chars() {
        if c.is_ascii_digit() {
            digits.push(c);
            if digits.len() == 4 {
                return digits.parse().ok();
            }
        } else {
            digits.clear();
        }
    }
    None
}
