//! Raw table rows, including the trash columns the frontend never sees.
//! All reads and writes of `boards`/`cards` go through these helpers.

use rusqlite::{params, Connection, OptionalExtension, Row};
use serde_json::Value;

use super::{Board, Card};
use crate::Result;

#[derive(Debug, Clone, PartialEq)]
pub struct BoardRow {
    pub id: String,
    pub parent_board_id: Option<String>,
    pub title: String,
    pub color: Option<String>,
    pub icon: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub trashed_by: Option<String>,
    pub trashed_at: Option<i64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CardRow {
    pub id: String,
    pub board_id: String,
    pub parent_card_id: Option<String>,
    pub kind: String,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: Option<f64>,
    pub z: i64,
    pub color: Option<String>,
    pub data: Value,
    pub search_text: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub trashed_by: Option<String>,
    pub trashed_at: Option<i64>,
}

pub(crate) const BOARD_COLS: &str =
    "id, parent_board_id, title, color, icon, created_at, updated_at, trashed_by, trashed_at";
pub(crate) const CARD_COLS: &str = "id, board_id, parent_card_id, kind, x, y, w, h, z, color, data, \
     search_text, created_at, updated_at, trashed_by, trashed_at";

impl BoardRow {
    pub fn is_live(&self) -> bool {
        self.trashed_by.is_none()
    }

    pub(crate) fn from_row(r: &Row) -> rusqlite::Result<Self> {
        Ok(Self {
            id: r.get(0)?,
            parent_board_id: r.get(1)?,
            title: r.get(2)?,
            color: r.get(3)?,
            icon: r.get(4)?,
            created_at: r.get(5)?,
            updated_at: r.get(6)?,
            trashed_by: r.get(7)?,
            trashed_at: r.get(8)?,
        })
    }

    pub fn get_any(conn: &Connection, id: &str) -> Result<Option<Self>> {
        Ok(conn
            .prepare_cached(&format!("SELECT {BOARD_COLS} FROM boards WHERE id = ?1"))?
            .query_row([id], Self::from_row)
            .optional()?)
    }

    pub fn get_live(conn: &Connection, id: &str) -> Result<Option<Self>> {
        Ok(Self::get_any(conn, id)?.filter(|b| b.is_live()))
    }

    pub fn query(conn: &Connection, where_sql: &str, args: impl rusqlite::Params) -> Result<Vec<Self>> {
        let mut st = conn.prepare_cached(&format!("SELECT {BOARD_COLS} FROM boards WHERE {where_sql}"))?;
        let rows = st.query_map(args, Self::from_row)?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn write(&self, conn: &Connection) -> Result<()> {
        conn.prepare_cached(&format!(
            "INSERT OR REPLACE INTO boards ({BOARD_COLS}) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)"
        ))?
        .execute(params![
            self.id,
            self.parent_board_id,
            self.title,
            self.color,
            self.icon,
            self.created_at,
            self.updated_at,
            self.trashed_by,
            self.trashed_at
        ])?;
        Ok(())
    }

    pub fn delete(conn: &Connection, id: &str) -> Result<()> {
        conn.prepare_cached("DELETE FROM boards WHERE id = ?1")?.execute([id])?;
        Ok(())
    }

    pub fn to_board(&self, conn: &Connection) -> Result<Board> {
        let card_count: i64 = conn
            .prepare_cached(
                "SELECT COUNT(*) FROM cards WHERE board_id = ?1 AND parent_card_id IS NULL \
                 AND trashed_by IS NULL AND kind != 'line'",
            )?
            .query_row([&self.id], |r| r.get(0))?;
        Ok(Board {
            id: self.id.clone(),
            parent_board_id: self.parent_board_id.clone(),
            title: self.title.clone(),
            color: self.color.clone(),
            icon: self.icon.clone(),
            created_at: self.created_at,
            updated_at: self.updated_at,
            card_count,
        })
    }
}

impl CardRow {
    pub fn is_live(&self) -> bool {
        self.trashed_by.is_none()
    }

    pub(crate) fn from_row(r: &Row) -> rusqlite::Result<Self> {
        let data: String = r.get(10)?;
        Ok(Self {
            id: r.get(0)?,
            board_id: r.get(1)?,
            parent_card_id: r.get(2)?,
            kind: r.get(3)?,
            x: r.get(4)?,
            y: r.get(5)?,
            w: r.get(6)?,
            h: r.get(7)?,
            z: r.get(8)?,
            color: r.get(9)?,
            data: serde_json::from_str(&data).unwrap_or_else(|_| Value::Object(Default::default())),
            search_text: r.get(11)?,
            created_at: r.get(12)?,
            updated_at: r.get(13)?,
            trashed_by: r.get(14)?,
            trashed_at: r.get(15)?,
        })
    }

    pub fn get_any(conn: &Connection, id: &str) -> Result<Option<Self>> {
        Ok(conn
            .prepare_cached(&format!("SELECT {CARD_COLS} FROM cards WHERE id = ?1"))?
            .query_row([id], Self::from_row)
            .optional()?)
    }

    pub fn get_live(conn: &Connection, id: &str) -> Result<Option<Self>> {
        Ok(Self::get_any(conn, id)?.filter(|c| c.is_live()))
    }

    pub fn query(conn: &Connection, where_sql: &str, args: impl rusqlite::Params) -> Result<Vec<Self>> {
        let mut st = conn.prepare_cached(&format!("SELECT {CARD_COLS} FROM cards WHERE {where_sql}"))?;
        let rows = st.query_map(args, Self::from_row)?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    /// Insert or replace the row, refreshing its search text.
    pub fn write(&self, conn: &Connection) -> Result<()> {
        let search = crate::services::text::search_text(&self.kind, &self.data);
        conn.prepare_cached(&format!(
            "INSERT OR REPLACE INTO cards ({CARD_COLS}) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)"
        ))?
        .execute(params![
            self.id,
            self.board_id,
            self.parent_card_id,
            self.kind,
            self.x,
            self.y,
            self.w,
            self.h,
            self.z,
            self.color,
            serde_json::to_string(&self.data)?,
            search,
            self.created_at,
            self.updated_at,
            self.trashed_by,
            self.trashed_at
        ])?;
        Ok(())
    }

    pub fn delete(conn: &Connection, id: &str) -> Result<()> {
        conn.prepare_cached("DELETE FROM cards WHERE id = ?1")?.execute([id])?;
        Ok(())
    }

    pub fn to_card(&self) -> Card {
        Card {
            id: self.id.clone(),
            board_id: self.board_id.clone(),
            parent_card_id: self.parent_card_id.clone(),
            kind: self.kind.clone(),
            x: self.x,
            y: self.y,
            w: self.w,
            h: self.h,
            z: self.z,
            color: self.color.clone(),
            data: self.data.clone(),
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }

    /// The board a `board` card opens.
    pub fn child_board_id(&self) -> Option<&str> {
        if self.kind == "board" {
            self.data.get("childBoardId").and_then(Value::as_str)
        } else {
            None
        }
    }
}
