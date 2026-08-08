use anyhow::Result;
use rusqlite::Connection;

use crate::db;
use crate::model::Kind;
use crate::ui;

pub fn run(conn: &Connection, tag: Option<&str>, kind: Option<Kind>) -> Result<()> {
    let docs = db::list(conn, tag, kind)?;
    ui::print_table(&docs);
    Ok(())
}

pub fn tags(conn: &Connection) -> Result<()> {
    let tags = db::all_tags(conn)?;
    if tags.is_empty() {
        println!("no tags yet.");
        return Ok(());
    }
    for tag in &tags {
        let count = db::list(conn, Some(tag), None)?.len();
        println!("{}  {count}", ui::truncate(tag, 24));
    }
    Ok(())
}
