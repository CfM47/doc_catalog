use anyhow::Result;
use inquire::Text;
use rusqlite::Connection;

use crate::db;
use crate::ui;

pub fn run(conn: &Connection, query: &str) -> Result<()> {
    let Some(doc) = ui::resolve(conn, query)? else {
        return Ok(());
    };

    let current = db::tags_for(conn, &doc.id)?;
    println!("{}", doc.title);

    let known = db::all_tags(conn)?;
    if !known.is_empty() {
        println!("existing tags: {}", known.join(", "));
    }

    // Editing the full comma-separated list makes add and remove the same
    // action, which keeps the command to a single prompt.
    let answer = Text::new("tags")
        .with_initial_value(&current.join(", "))
        .prompt()?;

    let tags: Vec<String> = answer
        .split(',')
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .collect();

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
