//! Best-effort metadata from the file itself. EPUB is usually trustworthy;
//! PDF info dictionaries are usually garbage, so PDFs lean on the DOI/ISBN
//! found in the text and the online lookup that follows.

use anyhow::Result;
use std::path::Path;

#[derive(Debug, Default, Clone)]
pub struct Extracted {
    pub title: Option<String>,
    pub authors: Option<String>,
    pub publisher: Option<String>,
    pub edition: Option<String>,
    pub year: Option<i64>,
    pub isbn: Option<String>,
    pub doi: Option<String>,
    pub journal: Option<String>,
    pub volume: Option<String>,
    pub issue: Option<String>,
    pub pages: Option<String>,
}

impl Extracted {
    /// Fill empty fields from `other`, keeping anything already set.
    pub fn merge_from(&mut self, other: Extracted) {
        macro_rules! fill {
            ($($f:ident),*) => { $( if self.$f.is_none() { self.$f = other.$f; } )* };
        }
        fill!(
            title, authors, publisher, edition, year, isbn, doi, journal, volume, issue, pages
        );
    }
}

pub fn extract(path: &Path) -> Extracted {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    let mut found = match ext.as_str() {
        "epub" => from_epub(path).unwrap_or_default(),
        "pdf" => from_pdf(path).unwrap_or_default(),
        _ => Extracted::default(),
    };

    if found.title.is_none() {
        found.title = path.file_stem().and_then(|s| s.to_str()).map(tidy_filename);
    }
    found
}

/// `Knuth_-_TAOCP_vol1_(2nd.ed).pdf` is a worse title than nothing, but it is
/// the only guess available when the file carries no metadata.
fn tidy_filename(stem: &str) -> String {
    let replaced: String = stem
        .chars()
        .map(|c| if c == '_' || c == '.' { ' ' } else { c })
        .collect();
    replaced.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn from_epub(path: &Path) -> Result<Extracted> {
    let doc = epub::doc::EpubDoc::new(path)?;
    let field = |name: &str| {
        doc.mdata(name)
            .map(|item| item.value.trim().to_string())
            .filter(|s| !s.is_empty())
    };
    let year = field("date").and_then(|d| d.get(..4).and_then(|y| y.parse::<i64>().ok()));
    Ok(Extracted {
        title: field("title"),
        authors: field("creator"),
        publisher: field("publisher"),
        year,
        isbn: field("identifier").filter(|s| looks_like_isbn(s)),
        ..Default::default()
    })
}

fn from_pdf(path: &Path) -> Result<Extracted> {
    let doc = lopdf::Document::load(path)?;
    let mut out = Extracted::default();

    if let Ok(lopdf::Object::Reference(id)) = doc.trailer.get(b"Info")
        && let Ok(info) = doc.get_dictionary(*id)
    {
        out.title = info
            .get(b"Title")
            .ok()
            .and_then(pdf_string)
            .filter(|t| usable_title(t));
        out.authors = info.get(b"Author").ok().and_then(pdf_string);
    }

    // Scan the opening pages for a DOI or ISBN; that one string beats every
    // other field, since the lookup then returns a complete record.
    let pages: Vec<u32> = doc.get_pages().keys().take(3).copied().collect();
    if let Ok(text) = doc.extract_text(&pages) {
        out.doi = find_doi(&text);
        if out.isbn.is_none() {
            out.isbn = find_isbn(&text);
        }
    }
    Ok(out)
}

/// PDF producers love writing the source document's filename into /Title.
fn usable_title(t: &str) -> bool {
    let lower = t.to_ascii_lowercase();
    !t.trim().is_empty()
        && !lower.starts_with("microsoft word")
        && !lower.ends_with(".doc")
        && !lower.ends_with(".docx")
        && !lower.ends_with(".tex")
        && !lower.ends_with(".dvi")
        && !lower.ends_with(".indd")
}

fn pdf_string(obj: &lopdf::Object) -> Option<String> {
    let bytes = obj.as_str().ok()?;
    // UTF-16BE with a byte-order mark, or PDFDocEncoding (latin-1 compatible).
    if bytes.starts_with(&[0xFE, 0xFF]) {
        let units: Vec<u16> = bytes[2..]
            .chunks_exact(2)
            .map(|c| u16::from_be_bytes([c[0], c[1]]))
            .collect();
        String::from_utf16(&units).ok()
    } else {
        Some(bytes.iter().map(|&b| b as char).collect())
    }
    .map(|s| s.trim().to_string())
    .filter(|s| !s.is_empty())
}

/// A DOI is `10.<registrant>/<suffix>`. Hand-rolled to avoid a regex dependency.
pub fn find_doi(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut i = 0;
    while let Some(pos) = text[i..].find("10.") {
        let start = i + pos;
        let mut j = start + 3;
        while j < bytes.len() && bytes[j].is_ascii_digit() {
            j += 1;
        }
        // Registrant code is 4+ digits and must be followed by a slash.
        if j - (start + 3) >= 4 && j < bytes.len() && bytes[j] == b'/' {
            let mut end = j + 1;
            while end < bytes.len() && !bytes[end].is_ascii_whitespace() {
                end += 1;
            }
            let doi = text[start..end].trim_end_matches(['.', ',', ';', ')', ']']);
            if doi.len() > j - start + 1 {
                return Some(doi.to_string());
            }
        }
        i = start + 3;
    }
    None
}

pub fn find_isbn(text: &str) -> Option<String> {
    let idx = text.to_ascii_uppercase().find("ISBN")?;
    let tail = &text[idx + 4..];
    let digits: String = tail
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '-' || *c == ' ' || *c == 'X' || *c == ':')
        .filter(|c| c.is_ascii_digit() || *c == 'X')
        .collect();
    if digits.len() == 10 || digits.len() == 13 {
        Some(digits)
    } else {
        None
    }
}

fn looks_like_isbn(s: &str) -> bool {
    let digits: String = s
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == 'X')
        .collect();
    (digits.len() == 10 || digits.len() == 13) && s.to_ascii_lowercase().contains("isbn")
        || digits.len() == 13 && digits.starts_with("97")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_a_doi_in_running_text() {
        let text = "Published in Nature. doi:10.1038/s41586-021-03819-2 Received 2021.";
        assert_eq!(
            find_doi(text).as_deref(),
            Some("10.1038/s41586-021-03819-2")
        );
    }

    #[test]
    fn strips_trailing_sentence_punctuation_from_a_doi() {
        assert_eq!(
            find_doi("see https://doi.org/10.1145/3292500.3330701.").as_deref(),
            Some("10.1145/3292500.3330701")
        );
    }

    #[test]
    fn ignores_numbers_that_only_look_like_dois() {
        // Too few registrant digits, and no slash.
        assert_eq!(find_doi("version 10.15 released"), None);
        assert_eq!(find_doi("10.1038 without a suffix"), None);
        assert_eq!(find_doi("no identifier at all"), None);
    }

    #[test]
    fn finds_isbn_in_both_lengths() {
        assert_eq!(
            find_isbn("ISBN: 978-0-201-89683-1").as_deref(),
            Some("9780201896831")
        );
        assert_eq!(find_isbn("isbn 0201896834").as_deref(), Some("0201896834"));
        assert_eq!(find_isbn("ISBN 12345"), None);
    }

    #[test]
    fn rejects_producer_filenames_as_titles() {
        assert!(!usable_title("Microsoft Word - draft_v3.doc"));
        assert!(!usable_title("paper.tex"));
        assert!(!usable_title("   "));
        assert!(usable_title("On Computable Numbers"));
    }

    #[test]
    fn filename_fallback_is_readable() {
        assert_eq!(tidy_filename("Knuth_-_TAOCP_vol1"), "Knuth - TAOCP vol1");
    }

    #[test]
    fn merge_keeps_existing_fields() {
        let mut base = Extracted {
            title: Some("Local title".into()),
            ..Default::default()
        };
        base.merge_from(Extracted {
            title: Some("Other title".into()),
            authors: Some("Someone".into()),
            ..Default::default()
        });
        assert_eq!(base.title.as_deref(), Some("Local title"));
        assert_eq!(base.authors.as_deref(), Some("Someone"));
    }
}
