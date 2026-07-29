use rusqlite::{params, Connection, Result};
use std::path::Path;

pub struct Db {
    conn: Connection,
}

impl Db {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let conn = Connection::open(path)?;
        
        conn.execute(
            "CREATE TABLE IF NOT EXISTS messages (
                id INTEGER PRIMARY KEY,
                timestamp INTEGER NOT NULL,
                group_id TEXT,
                sender_uuid TEXT NOT NULL,
                text TEXT NOT NULL
            )",
            [],
        )?;
        
        Ok(Self { conn })
    }

    pub fn insert_message(
        &self,
        timestamp: i64,
        group_id: Option<&str>,
        sender_uuid: &str,
        text: &str,
    ) -> Result<()> {
        self.conn.execute(
            "INSERT INTO messages (timestamp, group_id, sender_uuid, text) VALUES (?1, ?2, ?3, ?4)",
            params![timestamp, group_id, sender_uuid, text],
        )?;
        Ok(())
    }
}
