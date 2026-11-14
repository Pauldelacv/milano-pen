//! Database backups: manual snapshots, automatic rotation and validation.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use rusqlite::{Connection, OpenFlags};

use crate::database;
use crate::{Error, Result};

const PREFIX: &str = "milano-pen-";

/// A consistent copy of the live database (works while it is in use).
pub fn backup_to(conn: &Connection, dest: &Path) -> Result<()> {
    if dest.exists() {
        std::fs::remove_file(dest)?;
    }
    conn.execute("VACUUM INTO ?1", [dest.to_string_lossy()])?;
    Ok(())
}

fn auto_backups(dir: &Path) -> Result<Vec<(PathBuf, SystemTime)>> {
    let mut files = Vec::new();
    if !dir.exists() {
        return Ok(files);
    }
    for e in std::fs::read_dir(dir)? {
        let e = e?;
        let name = e.file_name().to_string_lossy().to_string();
        if name.starts_with(PREFIX) && name.ends_with(".sqlite") {
            files.push((e.path(), e.metadata()?.modified()?));
        }
    }
    files.sort_by_key(|(_, t)| *t);
    Ok(files)
}

/// Write a timestamped backup into `dir` unless one newer than `min_age`
/// exists, then keep only the `keep` most recent. Returns the new file.
pub fn auto_backup(conn: &Connection, dir: &Path, min_age: Duration, keep: usize) -> Result<Option<PathBuf>> {
    std::fs::create_dir_all(dir)?;
    let existing = auto_backups(dir)?;
    if let Some((_, newest)) = existing.last() {
        if SystemTime::now().duration_since(*newest).unwrap_or_default() < min_age {
            return Ok(None);
        }
    }
    let file = dir.join(format!("{PREFIX}{}.sqlite", chrono::Utc::now().format("%Y%m%d-%H%M%S%.3f")));
    backup_to(conn, &file)?;
    let all = auto_backups(dir)?;
    if all.len() > keep {
        for (p, _) in &all[..all.len() - keep] {
            let _ = std::fs::remove_file(p);
        }
    }
    Ok(Some(file))
}

/// Check that `path` is a healthy Milano Pen database this build can open.
pub fn validate(path: &Path) -> Result<()> {
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|e| Error::Invalid(format!("not a readable database: {e}")))?;
    let check: String = conn
        .query_row("PRAGMA quick_check", [], |r| r.get(0))
        .map_err(|e| Error::Invalid(format!("not a valid database: {e}")))?;
    if check != "ok" {
        return Err(Error::Invalid(format!("the backup is damaged: {check}")));
    }
    let version = database::schema_version(&conn)?;
    if version < 1 || version > database::latest_version() {
        return Err(Error::Invalid(format!("unsupported backup (schema v{version})")));
    }
    let tables: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name IN ('boards', 'cards', 'meta')",
        [],
        |r| r.get(0),
    )?;
    if tables != 3 {
        return Err(Error::Invalid("this file is not a Milano Pen database".into()));
    }
    Ok(())
}

/// [`auto_backup`] on a separate connection to `db_file`, so a large backup
/// never holds the app's main connection.
pub fn auto_backup_file(db_file: &Path, dir: &Path) -> Result<Option<PathBuf>> {
    let conn = Connection::open_with_flags(db_file, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    auto_backup(&conn, dir, Duration::from_secs(24 * 3600), 10)
}
