use anyhow::Result;
use rusqlite::Connection;

use crate::db;
use crate::ui;

pub fn run(conn: &Connection, query: &str) -> Result<()> {
    let Some(doc) = ui::resolve(conn, query)? else {
        return Ok(());
    };

    let current = db::tags_for(conn, &doc.id)?;
    println!("{}\n", doc.title);

    let tags = ui::ask_tags(conn, &current, "")?;
    db::set_tags(conn, &doc.id, &tags)?;
    println!(
        "tags: {}",
        if tags.is_empty() {
            "(none)".to_string()
        } else {
            tags.join(", ")
        }
    );
    Ok(())
}
