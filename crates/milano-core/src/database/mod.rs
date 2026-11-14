//! SQLite connection setup and schema migrations.

mod migrations;

use std::path::Path;
use std::time::Duration;

use rusqlite::Connection;

pub use migrations::{latest_version, migrate, schema_version};

use crate::Result;

/// Open (or create) the database at `path` and bring its schema up to date.
/// If an existing database needs migrating, a copy is first written to
/// `backups_dir`.
pub fn open(path: &Path, backups_dir: Option<&Path>) -> Result<Connection> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut conn = Connection::open(path)?;
    configure(&conn)?;
    migrate(&mut conn, backups_dir)?;
    Ok(conn)
}

/// A fresh, fully migrated in-memory database (tests, previews).
pub fn open_in_memory() -> Result<Connection> {
    let mut conn = Connection::open_in_memory()?;
    configure(&conn)?;
    migrate(&mut conn, None)?;
    Ok(conn)
}

fn configure(conn: &Connection) -> Result<()> {
    conn.busy_timeout(Duration::from_secs(5))?;
    // WAL keeps reads fast while a write is in flight and survives crashes;
    // NORMAL is durable at checkpoint boundaries, which WAL makes safe.
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    conn.pragma_update(None, "temp_store", "MEMORY")?;
    conn.set_prepared_statement_cache_capacity(64);
    Ok(())
}
