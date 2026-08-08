use anyhow::{Context, Result};
use rusqlite::{Connection, OptionalExtension, Row, params};

use crate::model::{Document, Kind};

const SCHEMA: &str = r#"
CREATE TABLE documents (
    id            TEXT PRIMARY KEY,
    kind          TEXT NOT NULL CHECK(kind IN ('book','article')),
    title         TEXT NOT NULL,
    authors       TEXT,
    year          INTEGER,

    publisher     TEXT,
    edition       TEXT,
    isbn          TEXT,

    journal       TEXT,
    volume        TEXT,
    issue         TEXT,
    pages         TEXT,
    doi           TEXT,

    ext           TEXT NOT NULL,
    size          INTEGER NOT NULL,
    content_hash  TEXT NOT NULL UNIQUE,
    remote_path   TEXT NOT NULL,

    cached_at     TEXT,
    last_opened   TEXT,
    added_at      TEXT NOT NULL,
    extra         TEXT
);

CREATE TABLE tags (
    id   INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE
);

CREATE TABLE document_tags (
    document_id TEXT NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
    tag_id      INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    PRIMARY KEY (document_id, tag_id)
);

CREATE VIRTUAL TABLE documents_fts USING fts5(
    title, authors, publisher, journal,
    content='documents', content_rowid='rowid'
);

CREATE TRIGGER documents_ai AFTER INSERT ON documents BEGIN
    INSERT INTO documents_fts(rowid, title, authors, publisher, journal)
    VALUES (new.rowid, new.title, new.authors, new.publisher, new.journal);
END;

CREATE TRIGGER documents_ad AFTER DELETE ON documents BEGIN
    INSERT INTO documents_fts(documents_fts, rowid, title, authors, publisher, journal)
    VALUES ('delete', old.rowid, old.title, old.authors, old.publisher, old.journal);
END;

CREATE TRIGGER documents_au AFTER UPDATE ON documents BEGIN
    INSERT INTO documents_fts(documents_fts, rowid, title, authors, publisher, journal)
    VALUES ('delete', old.rowid, old.title, old.authors, old.publisher, old.journal);
    INSERT INTO documents_fts(rowid, title, authors, publisher, journal)
    VALUES (new.rowid, new.title, new.authors, new.publisher, new.journal);
END;
"#;

const SELECT_DOC: &str = "SELECT id, kind, title, authors, year, publisher, edition, isbn, \
    journal, volume, issue, pages, doi, ext, size, content_hash, remote_path, \
    cached_at, last_opened, added_at, extra FROM documents";

/// Same column list, aliased, for queries that join another table.
const SELECT_DOC_D: &str = "SELECT d.id, d.kind, d.title, d.authors, d.year, d.publisher, \
    d.edition, d.isbn, d.journal, d.volume, d.issue, d.pages, d.doi, d.ext, d.size, \
    d.content_hash, d.remote_path, d.cached_at, d.last_opened, d.added_at, d.extra";

pub fn open(path: &std::path::Path) -> Result<Connection> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let conn = Connection::open(path).with_context(|| format!("opening {}", path.display()))?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    migrate(&conn)?;
    Ok(conn)
}

/// Records what a deleted document used to be, so its abandoned file can be
/// named rather than shown as a bare hash when purging.
const SCHEMA_V2: &str = r#"
CREATE TABLE deleted_documents (
    content_hash TEXT PRIMARY KEY,
    title        TEXT NOT NULL,
    authors      TEXT,
    ext          TEXT NOT NULL,
    size         INTEGER NOT NULL,
    remote_path  TEXT NOT NULL,
    deleted_at   TEXT NOT NULL
);
"#;

fn migrate(conn: &Connection) -> Result<()> {
    let version: i64 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
    if version < 1 {
        conn.execute_batch(SCHEMA)?;
        conn.pragma_update(None, "user_version", 1)?;
    }
    if version < 2 {
        conn.execute_batch(SCHEMA_V2)?;
        conn.pragma_update(None, "user_version", 2)?;
    }
    Ok(())
}

/// A file left on the remote after its catalog entry was deleted.
#[derive(Debug, Clone)]
pub struct Tombstone {
    pub content_hash: String,
    pub title: String,
    pub authors: Option<String>,
    pub remote_path: String,
    pub deleted_at: String,
}

pub fn tombstone(conn: &Connection, doc: &Document) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO deleted_documents
            (content_hash, title, authors, ext, size, remote_path, deleted_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7)",
        params![
            doc.content_hash,
            doc.title,
            doc.authors,
            doc.ext,
            doc.size,
            doc.remote_path,
            crate::now(),
        ],
    )?;
    Ok(())
}

pub fn tombstones(conn: &Connection) -> Result<Vec<Tombstone>> {
    let mut stmt = conn.prepare(
        "SELECT content_hash, title, authors, remote_path, deleted_at \
         FROM deleted_documents ORDER BY deleted_at",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(Tombstone {
            content_hash: row.get(0)?,
            title: row.get(1)?,
            authors: row.get(2)?,
            remote_path: row.get(3)?,
            deleted_at: row.get(4)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

pub fn forget_tombstone(conn: &Connection, hash: &str) -> Result<()> {
    conn.execute(
        "DELETE FROM deleted_documents WHERE content_hash = ?1",
        params![hash],
    )?;
    Ok(())
}

fn from_row(row: &Row<'_>) -> rusqlite::Result<Document> {
    let kind: String = row.get("kind")?;
    Ok(Document {
        id: row.get("id")?,
        kind: Kind::parse(&kind).unwrap_or(Kind::Book),
        title: row.get("title")?,
        authors: row.get("authors")?,
        year: row.get("year")?,
        publisher: row.get("publisher")?,
        edition: row.get("edition")?,
        isbn: row.get("isbn")?,
        journal: row.get("journal")?,
        volume: row.get("volume")?,
        issue: row.get("issue")?,
        pages: row.get("pages")?,
        doi: row.get("doi")?,
        ext: row.get("ext")?,
        size: row.get("size")?,
        content_hash: row.get("content_hash")?,
        remote_path: row.get("remote_path")?,
        cached_at: row.get("cached_at")?,
        last_opened: row.get("last_opened")?,
        added_at: row.get("added_at")?,
        extra: row.get("extra")?,
    })
}

pub fn insert(conn: &Connection, doc: &Document) -> Result<()> {
    conn.execute(
        "INSERT INTO documents (id, kind, title, authors, year, publisher, edition, isbn,
            journal, volume, issue, pages, doi, ext, size, content_hash, remote_path,
            cached_at, last_opened, added_at, extra)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21)",
        params![
            doc.id,
            doc.kind.as_str(),
            doc.title,
            doc.authors,
            doc.year,
            doc.publisher,
            doc.edition,
            doc.isbn,
            doc.journal,
            doc.volume,
            doc.issue,
            doc.pages,
            doc.doi,
            doc.ext,
            doc.size,
            doc.content_hash,
            doc.remote_path,
            doc.cached_at,
            doc.last_opened,
            doc.added_at,
            doc.extra,
        ],
    )?;
    Ok(())
}

/// Update the editable metadata. The bytes-related columns (hash, size,
/// remote_path) are deliberately not touched: they describe the file, not the
/// record, and only import may set them.
pub fn update(conn: &Connection, doc: &Document) -> Result<()> {
    conn.execute(
        "UPDATE documents SET kind = ?2, title = ?3, authors = ?4, year = ?5,
            publisher = ?6, edition = ?7, isbn = ?8, journal = ?9, volume = ?10,
            issue = ?11, pages = ?12, doi = ?13
         WHERE id = ?1",
        params![
            doc.id,
            doc.kind.as_str(),
            doc.title,
            doc.authors,
            doc.year,
            doc.publisher,
            doc.edition,
            doc.isbn,
            doc.journal,
            doc.volume,
            doc.issue,
            doc.pages,
            doc.doi,
        ],
    )?;
    Ok(())
}

/// Remove the catalog row. Tag links cascade; the FTS entry is dropped by the
/// delete trigger. The stored file is not touched here.
pub fn delete(conn: &Connection, id: &str) -> Result<()> {
    conn.execute("DELETE FROM documents WHERE id = ?1", params![id])?;
    drop_orphan_tags(conn)?;
    Ok(())
}

/// A tag nobody uses is noise in the known-tags list and shows up as a zero
/// count in `doclib tags`, so it goes when its last document does.
fn drop_orphan_tags(conn: &Connection) -> Result<()> {
    conn.execute(
        "DELETE FROM tags WHERE id NOT IN (SELECT tag_id FROM document_tags)",
        [],
    )?;
    Ok(())
}

pub fn find_by_hash(conn: &Connection, hash: &str) -> Result<Option<Document>> {
    let sql = format!("{SELECT_DOC} WHERE content_hash = ?1");
    Ok(conn.query_row(&sql, params![hash], from_row).optional()?)
}

pub fn find_by_id(conn: &Connection, id: &str) -> Result<Option<Document>> {
    let sql = format!("{SELECT_DOC} WHERE id = ?1");
    Ok(conn.query_row(&sql, params![id], from_row).optional()?)
}

pub fn all(conn: &Connection) -> Result<Vec<Document>> {
    let sql = format!("{SELECT_DOC} ORDER BY title COLLATE NOCASE");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([], from_row)?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

/// FTS5 treats bare user input as a query expression, so a stray `-` or `"`
/// is a syntax error rather than a search term. Quote every token and make it
/// a prefix match.
fn fts_query(input: &str) -> String {
    input
        .split_whitespace()
        .map(|tok| format!("\"{}\"*", tok.replace('"', "\"\"")))
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn search(conn: &Connection, query: &str) -> Result<Vec<Document>> {
    if query.trim().is_empty() {
        return all(conn);
    }
    let sql = format!(
        "{SELECT_DOC_D} FROM documents_fts f JOIN documents d ON d.rowid = f.rowid \
         WHERE documents_fts MATCH ?1 ORDER BY rank"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params![fts_query(query)], from_row)?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

pub fn list(conn: &Connection, tag: Option<&str>, kind: Option<Kind>) -> Result<Vec<Document>> {
    let mut sql = format!("{SELECT_DOC_D} FROM documents d");
    let mut clauses: Vec<String> = Vec::new();
    let mut args: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if let Some(tag) = tag {
        sql.push_str(
            " JOIN document_tags dt ON dt.document_id = d.id JOIN tags t ON t.id = dt.tag_id",
        );
        clauses.push("t.name = ?".to_string());
        args.push(Box::new(tag.to_string()));
    }
    if let Some(kind) = kind {
        clauses.push("d.kind = ?".to_string());
        args.push(Box::new(kind.as_str().to_string()));
    }
    if !clauses.is_empty() {
        sql.push_str(" WHERE ");
        sql.push_str(&clauses.join(" AND "));
    }
    sql.push_str(" ORDER BY d.title COLLATE NOCASE");

    let mut stmt = conn.prepare(&sql)?;
    let refs: Vec<&dyn rusqlite::ToSql> = args.iter().map(|b| b.as_ref()).collect();
    let rows = stmt.query_map(refs.as_slice(), from_row)?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

pub fn mark_cached(conn: &Connection, id: &str, when: Option<&str>) -> Result<()> {
    conn.execute(
        "UPDATE documents SET cached_at = ?2 WHERE id = ?1",
        params![id, when],
    )?;
    Ok(())
}

pub fn touch_opened(conn: &Connection, id: &str, when: &str) -> Result<()> {
    conn.execute(
        "UPDATE documents SET last_opened = ?2 WHERE id = ?1",
        params![id, when],
    )?;
    Ok(())
}

pub fn set_tags(conn: &Connection, doc_id: &str, tags: &[String]) -> Result<()> {
    let tx = conn.unchecked_transaction()?;
    tx.execute(
        "DELETE FROM document_tags WHERE document_id = ?1",
        params![doc_id],
    )?;
    for name in tags {
        let name = name.trim();
        if name.is_empty() {
            continue;
        }
        tx.execute(
            "INSERT OR IGNORE INTO tags (name) VALUES (?1)",
            params![name],
        )?;
        let tag_id: i64 =
            tx.query_row("SELECT id FROM tags WHERE name = ?1", params![name], |r| {
                r.get(0)
            })?;
        tx.execute(
            "INSERT OR IGNORE INTO document_tags (document_id, tag_id) VALUES (?1, ?2)",
            params![doc_id, tag_id],
        )?;
    }
    drop_orphan_tags(&tx)?;
    tx.commit()?;
    Ok(())
}

pub fn tags_for(conn: &Connection, doc_id: &str) -> Result<Vec<String>> {
    let mut stmt = conn.prepare(
        "SELECT t.name FROM tags t JOIN document_tags dt ON dt.tag_id = t.id \
         WHERE dt.document_id = ?1 ORDER BY t.name",
    )?;
    let rows = stmt.query_map(params![doc_id], |r| r.get::<_, String>(0))?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

/// Every tag with how many documents carry it, most used first.
pub fn tag_counts(conn: &Connection) -> Result<Vec<(String, i64)>> {
    let mut stmt = conn.prepare(
        "SELECT t.name, COUNT(dt.document_id) AS uses \
         FROM tags t LEFT JOIN document_tags dt ON dt.tag_id = t.id \
         GROUP BY t.id ORDER BY uses DESC, t.name COLLATE NOCASE",
    )?;
    let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

pub fn untagged_count(conn: &Connection) -> Result<i64> {
    Ok(conn.query_row(
        "SELECT COUNT(*) FROM documents d \
         WHERE NOT EXISTS (SELECT 1 FROM document_tags dt WHERE dt.document_id = d.id)",
        [],
        |r| r.get(0),
    )?)
}

pub fn all_tags(conn: &Connection) -> Result<Vec<String>> {
    let mut stmt = conn.prepare("SELECT name FROM tags ORDER BY name")?;
    let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn memory_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        migrate(&conn).unwrap();
        conn
    }

    fn doc(id: &str, title: &str, authors: &str, kind: Kind) -> Document {
        Document {
            id: id.to_string(),
            kind,
            title: title.to_string(),
            authors: Some(authors.to_string()),
            year: Some(1997),
            publisher: Some("Addison-Wesley".to_string()),
            edition: None,
            isbn: None,
            journal: None,
            volume: None,
            issue: None,
            pages: None,
            doi: None,
            ext: "pdf".to_string(),
            size: 1024,
            content_hash: format!("hash-{id}"),
            remote_path: format!("ha/hash-{id}.pdf"),
            cached_at: None,
            last_opened: None,
            added_at: "2026-01-01T00:00:00Z".to_string(),
            extra: None,
        }
    }

    #[test]
    fn search_finds_by_title_author_and_publisher() {
        let conn = memory_db();
        insert(
            &conn,
            &doc(
                "a",
                "The Art of Computer Programming",
                "Donald Knuth",
                Kind::Book,
            ),
        )
        .unwrap();
        insert(
            &conn,
            &doc(
                "b",
                "Attention Is All You Need",
                "Ashish Vaswani",
                Kind::Article,
            ),
        )
        .unwrap();

        assert_eq!(search(&conn, "knuth").unwrap().len(), 1);
        assert_eq!(search(&conn, "attention").unwrap().len(), 1);
        assert_eq!(search(&conn, "addison").unwrap().len(), 2);
        assert_eq!(search(&conn, "computer program").unwrap().len(), 1);
        // Empty query lists everything rather than erroring.
        assert_eq!(search(&conn, "  ").unwrap().len(), 2);
    }

    #[test]
    fn search_tolerates_fts_metacharacters() {
        let conn = memory_db();
        insert(
            &conn,
            &doc("a", "Rust in Action", "Tim McNamara", Kind::Book),
        )
        .unwrap();
        // Bare input like this is a syntax error if handed to FTS5 unescaped.
        for query in ["rust -", "\"rust", "rust OR", "(rust", "rust*NEAR"] {
            assert!(search(&conn, query).is_ok(), "query {query:?} failed");
        }
    }

    #[test]
    fn deleting_a_document_removes_it_from_the_index() {
        let conn = memory_db();
        insert(
            &conn,
            &doc("a", "Structure and Interpretation", "Abelson", Kind::Book),
        )
        .unwrap();
        conn.execute("DELETE FROM documents WHERE id = 'a'", [])
            .unwrap();
        assert!(search(&conn, "abelson").unwrap().is_empty());
    }

    #[test]
    fn set_tags_replaces_the_previous_set() {
        let conn = memory_db();
        insert(
            &conn,
            &doc("a", "Types and Programming Languages", "Pierce", Kind::Book),
        )
        .unwrap();

        set_tags(&conn, "a", &["theory".into(), "types".into()]).unwrap();
        assert_eq!(tags_for(&conn, "a").unwrap(), vec!["theory", "types"]);

        set_tags(&conn, "a", &["types".into()]).unwrap();
        assert_eq!(tags_for(&conn, "a").unwrap(), vec!["types"]);
        // "theory" now has no documents, so it stops being a known tag.
        assert_eq!(all_tags(&conn).unwrap(), vec!["types"]);
    }

    #[test]
    fn a_tag_shared_with_another_document_survives() {
        let conn = memory_db();
        insert(&conn, &doc("a", "First", "X", Kind::Book)).unwrap();
        insert(&conn, &doc("b", "Second", "Y", Kind::Book)).unwrap();
        set_tags(&conn, "a", &["shared".into()]).unwrap();
        set_tags(&conn, "b", &["shared".into()]).unwrap();

        delete(&conn, "a").unwrap();
        assert_eq!(all_tags(&conn).unwrap(), vec!["shared"]);
        assert_eq!(tags_for(&conn, "b").unwrap(), vec!["shared"]);
    }

    #[test]
    fn list_filters_by_tag_and_kind() {
        let conn = memory_db();
        insert(&conn, &doc("a", "Book One", "Author A", Kind::Book)).unwrap();
        insert(&conn, &doc("b", "Paper One", "Author B", Kind::Article)).unwrap();
        set_tags(&conn, "a", &["rust".into()]).unwrap();
        set_tags(&conn, "b", &["rust".into()]).unwrap();

        assert_eq!(list(&conn, Some("rust"), None).unwrap().len(), 2);
        assert_eq!(
            list(&conn, Some("rust"), Some(Kind::Article))
                .unwrap()
                .len(),
            1
        );
        assert_eq!(list(&conn, None, Some(Kind::Book)).unwrap().len(), 1);
        assert_eq!(list(&conn, Some("missing"), None).unwrap().len(), 0);
    }

    #[test]
    fn update_reindexes_the_document() {
        let conn = memory_db();
        insert(&conn, &doc("a", "Untitled Scan", "-", Kind::Book)).unwrap();

        let mut fixed = find_by_hash(&conn, "hash-a").unwrap().unwrap();
        fixed.title = "Regulated Grammars and Automata".to_string();
        fixed.authors = Some("Alexander Meduna".to_string());
        update(&conn, &fixed).unwrap();

        assert_eq!(search(&conn, "meduna").unwrap().len(), 1);
        // The stale title must not linger in the index.
        assert!(search(&conn, "untitled").unwrap().is_empty());
    }

    #[test]
    fn update_leaves_the_stored_file_columns_alone() {
        let conn = memory_db();
        insert(&conn, &doc("a", "Book", "Author", Kind::Book)).unwrap();

        let mut tampered = find_by_hash(&conn, "hash-a").unwrap().unwrap();
        tampered.title = "Renamed".to_string();
        tampered.content_hash = "different".to_string();
        tampered.remote_path = "zz/different.pdf".to_string();
        update(&conn, &tampered).unwrap();

        let stored = find_by_hash(&conn, "hash-a").unwrap().unwrap();
        assert_eq!(stored.title, "Renamed");
        assert_eq!(stored.remote_path, "ha/hash-a.pdf");
    }

    #[test]
    fn delete_removes_the_row_its_tags_and_its_index_entry() {
        let conn = memory_db();
        insert(&conn, &doc("a", "Discarded Book", "Nobody", Kind::Book)).unwrap();
        set_tags(&conn, "a", &["temp".into()]).unwrap();

        delete(&conn, "a").unwrap();

        assert!(all(&conn).unwrap().is_empty());
        assert!(search(&conn, "discarded").unwrap().is_empty());
        assert!(tags_for(&conn, "a").unwrap().is_empty());
        // Nothing else used "temp", so it goes too.
        assert!(all_tags(&conn).unwrap().is_empty());
    }

    #[test]
    fn deleting_frees_the_hash_for_reimport() {
        let conn = memory_db();
        insert(&conn, &doc("a", "Book", "Author", Kind::Book)).unwrap();
        delete(&conn, "a").unwrap();
        // Same bytes, new row: the UNIQUE constraint must no longer block it.
        assert!(insert(&conn, &doc("a", "Book", "Author", Kind::Book)).is_ok());
    }

    #[test]
    fn duplicate_content_hash_is_rejected() {
        let conn = memory_db();
        let first = doc("a", "Same Bytes", "X", Kind::Book);
        let mut second = doc("b", "Same Bytes Renamed", "X", Kind::Book);
        second.content_hash = first.content_hash.clone();

        insert(&conn, &first).unwrap();
        assert!(insert(&conn, &second).is_err());
        assert!(find_by_hash(&conn, &first.content_hash).unwrap().is_some());
    }
}
