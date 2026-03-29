use std::path::Path;
use std::sync::{Arc, Mutex};

use rusqlite::Connection;

pub type ConnectionRef = Arc<Mutex<Connection>>;

pub struct Database {
    pub conn: ConnectionRef,
}

impl Database {
    pub fn new<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let conn = Connection::open(path)?;
        let conn = Arc::new(Mutex::new(conn));
        Ok(Self { conn })
    }

    pub fn in_memory() -> anyhow::Result<Self> {
        let conn = Connection::open_in_memory()?;
        let conn = Arc::new(Mutex::new(conn));
        Ok(Self { conn })
    }
}
