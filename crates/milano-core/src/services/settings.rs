//! User preferences, stored as one JSON value in `meta`.

use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::Result;

const KEY: &str = "settings";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    /// "system", "light" or "dark".
    pub theme: String,
    /// Plain mouse wheel zooms (otherwise it pans and ⌘/pinch zooms).
    pub wheel_zooms: bool,
    pub snap_to_grid: bool,
    pub grid_size: f64,
    pub auto_backup: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self { theme: "system".into(), wheel_zooms: false, snap_to_grid: true, grid_size: 10.0, auto_backup: true }
    }
}

pub fn load(conn: &Connection) -> Result<Settings> {
    let raw: Option<String> =
        conn.query_row("SELECT value FROM meta WHERE key = ?1", [KEY], |r| r.get(0)).optional()?;
    Ok(raw.and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default())
}

pub fn save(conn: &Connection, s: &Settings) -> Result<Settings> {
    let mut s = s.clone();
    if !matches!(s.theme.as_str(), "system" | "light" | "dark") {
        s.theme = "system".into();
    }
    if !(2.0..=200.0).contains(&s.grid_size) {
        s.grid_size = 10.0;
    }
    conn.execute(
        "INSERT INTO meta (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        [KEY, &serde_json::to_string(&s)?],
    )?;
    Ok(s)
}
