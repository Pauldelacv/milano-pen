//! The Trash: one entry per delete, restore, and permanent deletion.

use rusqlite::Connection;

use super::text;
use crate::files::AssetStore;
use crate::models::{BoardRow, CardRow, TrashEntry};
use crate::Result;

pub fn list(conn: &Connection) -> Result<Vec<TrashEntry>> {
    let roots = CardRow::query(conn, "trashed_by = id ORDER BY trashed_at DESC, id", [])?;
    let mut out = Vec::with_capacity(roots.len());
    for r in roots {
        let board = BoardRow::get_any(conn, &r.board_id)?;
        let title = match r.child_board_id() {
            Some(cid) => BoardRow::get_any(conn, cid)?.map(|b| b.title).unwrap_or_else(|| "Board".into()),
            None => text::summary(&r.kind, &r.data, None),
        };
        out.push(TrashEntry {
            id: r.id.clone(),
            kind: r.kind.clone(),
            title: text::truncate(&title, 80),
            board_id: r.board_id.clone(),
            board_title: board.as_ref().map(|b| b.title.clone()).unwrap_or_default(),
            restorable: board.is_some_and(|b| b.is_live()),
            trashed_at: r.trashed_at.unwrap_or_default(),
        });
    }
    Ok(out)
}

#[derive(Debug, Clone, Copy, Default, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EmptyReport {
    pub cards: usize,
    pub boards: usize,
    pub files: usize,
}

/// Permanently delete everything in the trash, then the asset files no live
/// or trashed card references any more. Not undoable.
pub fn empty(conn: &Connection, store: Option<&AssetStore>) -> Result<EmptyReport> {
    let cards = conn.execute("DELETE FROM cards WHERE trashed_by IS NOT NULL", [])?;
    let boards = conn.execute("DELETE FROM boards WHERE trashed_by IS NOT NULL", [])?;
    let orphans: Vec<(String, String)> = conn
        .prepare(
            "SELECT id, ext FROM assets WHERE id NOT IN \
             (SELECT json_extract(data, '$.assetId') FROM cards WHERE json_extract(data, '$.assetId') IS NOT NULL)",
        )?
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<rusqlite::Result<_>>()?;
    let mut files = 0;
    for (id, ext) in &orphans {
        conn.execute("DELETE FROM assets WHERE id = ?1", [id])?;
        if let Some(store) = store {
            if store.remove(id, ext) {
                files += 1;
            }
        }
    }
    Ok(EmptyReport { cards, boards, files })
}
