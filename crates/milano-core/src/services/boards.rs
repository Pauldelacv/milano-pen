//! Boards: the root board, navigation and board metadata.
//!
//! As in Milaclone, a `board` card on board A owns a child board B; the card's
//! title, colour and icon live on B's row so every view of B agrees.

use rusqlite::{Connection, OptionalExtension};

use super::history::Recorder;
use crate::models::{Board, BoardRow, BoardUpdate, BoardView, CardRow, Crumb};
use crate::{new_id, now_ms, Error, Result};

pub const ROOT_KEY: &str = "root_board_id";
pub const DEFAULT_ICON: &str = "layout-grid";
pub const MAX_TITLE: usize = 200;

/// The Home board's id, creating it on first run.
pub fn ensure_root(conn: &Connection) -> Result<String> {
    let existing: Option<String> =
        conn.query_row("SELECT value FROM meta WHERE key = ?1", [ROOT_KEY], |r| r.get(0)).optional()?;
    if let Some(id) = existing {
        if BoardRow::get_any(conn, &id)?.is_some() {
            return Ok(id);
        }
    }
    let now = now_ms();
    let row = BoardRow {
        id: new_id(),
        parent_board_id: None,
        title: "Home".into(),
        color: Some("slate".into()),
        icon: Some("house".into()),
        created_at: now,
        updated_at: now,
        trashed_by: None,
        trashed_at: None,
    };
    row.write(conn)?;
    conn.execute(
        "INSERT INTO meta (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        [ROOT_KEY, &row.id],
    )?;
    Ok(row.id)
}

pub fn get(conn: &Connection, id: &str) -> Result<Board> {
    BoardRow::get_live(conn, id)?.ok_or_else(|| Error::NotFound(format!("board {id}")))?.to_board(conn)
}

/// Home → … → this board.
pub fn breadcrumb(conn: &Connection, id: &str) -> Result<Vec<Crumb>> {
    let mut trail = Vec::new();
    let mut cur = BoardRow::get_live(conn, id)?;
    while let Some(b) = cur {
        if trail.len() > 256 {
            break; // corrupt cycle guard
        }
        let parent = b.parent_board_id.clone();
        trail.push(Crumb { id: b.id, title: b.title, color: b.color });
        cur = match parent {
            Some(p) => BoardRow::get_live(conn, &p)?,
            None => None,
        };
    }
    trail.reverse();
    Ok(trail)
}

pub fn open(conn: &Connection, id: &str) -> Result<BoardView> {
    let board = get(conn, id)?;
    let rows = CardRow::query(conn, "board_id = ?1 AND trashed_by IS NULL ORDER BY z, created_at, id", [id])?;
    let mut child_boards = Vec::new();
    for r in &rows {
        if let Some(cid) = r.child_board_id() {
            if let Some(b) = BoardRow::get_live(conn, cid)? {
                child_boards.push(b.to_board(conn)?);
            }
        }
    }
    Ok(BoardView {
        breadcrumb: breadcrumb(conn, id)?,
        board,
        cards: rows.iter().map(CardRow::to_card).collect(),
        child_boards,
    })
}

pub fn new_board_row(parent: &str, title: Option<&str>, color: Option<String>, icon: Option<String>) -> BoardRow {
    let now = now_ms();
    BoardRow {
        id: new_id(),
        parent_board_id: Some(parent.to_string()),
        title: clean_title(title.unwrap_or("Untitled board")),
        color: color.or_else(|| Some("slate".into())),
        icon: icon.or_else(|| Some(DEFAULT_ICON.into())),
        created_at: now,
        updated_at: now,
        trashed_by: None,
        trashed_at: None,
    }
}

pub fn clean_title(t: &str) -> String {
    let t = t.trim();
    let t = if t.is_empty() { "Untitled board" } else { t };
    t.chars().take(MAX_TITLE).collect()
}

pub fn update(conn: &Connection, rec: &mut Recorder, id: &str, u: &BoardUpdate) -> Result<()> {
    let mut row = BoardRow::get_live(conn, id)?.ok_or_else(|| Error::NotFound(format!("board {id}")))?;
    if let Some(t) = &u.title {
        row.title = clean_title(t);
    }
    if let Some(c) = &u.color {
        row.color = if c.is_empty() { None } else { Some(c.clone()) };
    }
    if let Some(i) = &u.icon {
        row.icon = if i.is_empty() { None } else { Some(i.clone()) };
    }
    row.updated_at = now_ms();
    rec.put_board(conn, &row)
}
