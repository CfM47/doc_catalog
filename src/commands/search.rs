use anyhow::Result;
use inquire::Text;
use rusqlite::Connection;

use crate::db;
use crate::ui;

pub fn run(conn: &Connection, query: Option<&str>) -> Result<()> {
    let query = match query {
        Some(q) => q.to_string(),
        None => Text::new("search").prompt()?,
    };

    let results = db::search(conn, &query)?;
    ui::print_table(&results);
    Ok(())
}
