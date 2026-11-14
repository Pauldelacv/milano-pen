use std::path::Path;

use rusqlite::Connection;

use crate::Result;

/// Embedded migrations, applied in order. Never edit a shipped migration:
/// add a new file instead.
const MIGRATIONS: &[(i64, &str)] = &[(1, include_str!("../../migrations/0001_initial.sql"))];

pub fn latest_version() -> i64 {
    MIGRATIONS.last().map(|m| m.0).unwrap_or(0)
}

pub fn schema_version(conn: &Connection) -> Result<i64> {
    Ok(conn.pragma_query_value(None, "user_version", |r| r.get(0))?)
}

/// Apply every pending migration, each in its own transaction. Returns the
/// number applied. A database newer than this build is refused rather than
/// risk corrupting it.
pub fn migrate(conn: &mut Connection, backups_dir: Option<&Path>) -> Result<usize> {
    let current = schema_version(conn)?;
    if current > latest_version() {
        return Err(crate::Error::Conflict(format!(
            "this database was created by a newer Milano Pen (schema v{current}); please update the app"
        )));
    }
    let pending: Vec<_> = MIGRATIONS.iter().filter(|(v, _)| *v > current).collect();
    if pending.is_empty() {
        return Ok(0);
    }
    if current > 0 {
        if let Some(dir) = backups_dir {
            std::fs::create_dir_all(dir)?;
            let file =
                dir.join(format!("pre-migration-v{current}-{}.sqlite", chrono::Utc::now().format("%Y%m%d-%H%M%S")));
            conn.execute("VACUUM INTO ?1", [file.to_string_lossy()])?;
        }
    }
    for (version, sql) in &pending {
        let tx = conn.transaction()?;
        tx.execute_batch(sql)?;
        tx.pragma_update(None, "user_version", version)?;
        tx.commit()?;
        log::info!("applied migration v{version}");
    }
    Ok(pending.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrates_a_fresh_database_once() {
        let mut conn = Connection::open_in_memory().unwrap();
        assert_eq!(migrate(&mut conn, None).unwrap(), MIGRATIONS.len());
        assert_eq!(schema_version(&conn).unwrap(), latest_version());
        assert_eq!(migrate(&mut conn, None).unwrap(), 0);
    }

    #[test]
    fn refuses_a_newer_schema() {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "user_version", latest_version() + 1).unwrap();
        assert!(migrate(&mut conn, None).is_err());
    }
}
