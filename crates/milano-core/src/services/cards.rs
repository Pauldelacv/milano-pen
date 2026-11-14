//! Card operations: create, update, z-order, trash/restore, copy/paste.

use std::collections::{HashMap, HashSet};

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::boards;
use super::history::Recorder;
use crate::models::{is_known_kind, BoardRow, CardRow, CardUpdate, NewCard};
use crate::{new_id, now_ms, Error, Result};

pub const MIN_SIZE: f64 = 24.0;
pub const MAX_COORD: f64 = 1.0e7;

pub fn default_width(kind: &str) -> f64 {
    match kind {
        "board" | "shortcut" => 140.0,
        "image" => 280.0,
        "document" => 320.0,
        "column" => 300.0,
        "swatch" => 120.0,
        "sketch" => 400.0,
        "table" => 360.0,
        _ => 240.0,
    }
}

fn check_coord(v: f64, what: &str) -> Result<f64> {
    if !v.is_finite() || v.abs() > MAX_COORD {
        return Err(Error::Invalid(format!("invalid {what}: {v}")));
    }
    Ok(v)
}

fn check_size(v: f64, what: &str) -> Result<f64> {
    Ok(check_coord(v, what)?.max(MIN_SIZE))
}

fn top_z(conn: &Connection, board_id: &str) -> Result<i64> {
    Ok(conn
        .prepare_cached("SELECT COALESCE(MAX(z), 0) FROM cards WHERE board_id = ?1 AND trashed_by IS NULL")?
        .query_row([board_id], |r| r.get(0))?)
}

fn bottom_z(conn: &Connection, board_id: &str) -> Result<i64> {
    Ok(conn
        .prepare_cached("SELECT COALESCE(MIN(z), 0) FROM cards WHERE board_id = ?1 AND trashed_by IS NULL")?
        .query_row([board_id], |r| r.get(0))?)
}

fn live_card(conn: &Connection, id: &str) -> Result<CardRow> {
    CardRow::get_live(conn, id)?.ok_or_else(|| Error::NotFound(format!("card {id}")))
}

fn check_parent(
    conn: &Connection,
    parent: Option<&str>,
    board_id: &str,
    kind: &str,
    self_id: Option<&str>,
) -> Result<()> {
    let Some(pid) = parent else { return Ok(()) };
    let p = CardRow::get_live(conn, pid)?;
    let ok = kind != "column"
        && matches!(&p, Some(p) if p.kind == "column" && p.board_id == board_id && Some(p.id.as_str()) != self_id);
    if ok {
        Ok(())
    } else {
        Err(Error::Invalid("a card can only sit in a live column on its own board".into()))
    }
}

/// Create one card. Returns its id.
pub fn create(conn: &Connection, rec: &mut Recorder, n: &NewCard) -> Result<String> {
    if !is_known_kind(&n.kind) {
        return Err(Error::Invalid(format!("unknown card kind {:?}", n.kind)));
    }
    BoardRow::get_live(conn, &n.board_id)?.ok_or_else(|| Error::NotFound(format!("board {}", n.board_id)))?;
    check_parent(conn, n.parent_card_id.as_deref(), &n.board_id, &n.kind, None)?;
    let mut data = match &n.data {
        Some(Value::Object(m)) => Value::Object(m.clone()),
        Some(_) => return Err(Error::Invalid("card data must be an object".into())),
        None => Value::Object(Map::new()),
    };
    if n.kind == "board" {
        let title = data.get("title").and_then(Value::as_str).map(String::from);
        let icon = data.get("icon").and_then(Value::as_str).map(String::from);
        let child = boards::new_board_row(&n.board_id, title.as_deref(), n.color.clone(), icon);
        rec.put_board(conn, &child)?;
        data = serde_json::json!({ "childBoardId": child.id });
    }
    if n.kind == "shortcut" {
        let target = data.get("targetBoardId").and_then(Value::as_str).unwrap_or_default();
        BoardRow::get_live(conn, target)?.ok_or_else(|| Error::Invalid("shortcut target board not found".into()))?;
    }
    let now = now_ms();
    let row = CardRow {
        id: new_id(),
        board_id: n.board_id.clone(),
        parent_card_id: n.parent_card_id.clone(),
        kind: n.kind.clone(),
        x: check_coord(n.x, "x")?,
        y: check_coord(n.y, "y")?,
        w: check_size(n.w.unwrap_or_else(|| default_width(&n.kind)), "width")?,
        h: n.h.filter(|h| *h > 0.0).map(|h| check_size(h, "height")).transpose()?,
        z: top_z(conn, &n.board_id)? + 1,
        color: n.color.clone().filter(|c| !c.is_empty()),
        data,
        search_text: String::new(),
        created_at: now,
        updated_at: now,
        trashed_by: None,
        trashed_at: None,
    };
    rec.put_card(conn, &row)?;
    Ok(row.id)
}

/// Shallow merge: `null` removes a key.
fn merge_data(target: &mut Value, patch: &Value) -> Result<()> {
    let (Value::Object(t), Value::Object(p)) = (target, patch) else {
        return Err(Error::Invalid("card data must be an object".into()));
    };
    for (k, v) in p {
        if v.is_null() {
            t.remove(k);
        } else {
            t.insert(k.clone(), v.clone());
        }
    }
    Ok(())
}

pub fn update(conn: &Connection, rec: &mut Recorder, updates: &[CardUpdate]) -> Result<()> {
    let now = now_ms();
    for u in updates {
        let mut row = live_card(conn, &u.id)?;
        if let Some(x) = u.x {
            row.x = check_coord(x, "x")?;
        }
        if let Some(y) = u.y {
            row.y = check_coord(y, "y")?;
        }
        if let Some(w) = u.w {
            row.w = check_size(w, "width")?;
        }
        if let Some(h) = u.h {
            row.h = if h > 0.0 { Some(check_size(h, "height")?) } else { None };
        }
        if let Some(z) = u.z {
            row.z = z;
        }
        if let Some(c) = &u.color {
            row.color = if c.is_empty() { None } else { Some(c.clone()) };
        }
        if let Some(d) = &u.data {
            if let Some(child) = row.child_board_id().map(String::from) {
                // Board cards: title/icon are stored on the child board only.
                let mut b =
                    BoardRow::get_live(conn, &child)?.ok_or_else(|| Error::NotFound(format!("board {child}")))?;
                if let Some(t) = d.get("title").and_then(Value::as_str) {
                    b.title = boards::clean_title(t);
                }
                if let Some(i) = d.get("icon").and_then(Value::as_str) {
                    b.icon = Some(i.to_string());
                }
                b.updated_at = now;
                rec.put_board(conn, &b)?;
            } else {
                merge_data(&mut row.data, d)?;
            }
        }
        if u.color.is_some() {
            if let Some(child) = row.child_board_id().map(String::from) {
                if let Some(mut b) = BoardRow::get_live(conn, &child)? {
                    b.color = row.color.clone();
                    b.updated_at = now;
                    rec.put_board(conn, &b)?;
                }
            }
        }
        row.updated_at = now;
        rec.put_card(conn, &row)?;
    }
    Ok(())
}

/// Ordered by current z, re-stacked above (or below) everything else.
pub fn restack(conn: &Connection, rec: &mut Recorder, ids: &[String], to_front: bool) -> Result<()> {
    let mut rows = ids.iter().map(|id| live_card(conn, id)).collect::<Result<Vec<_>>>()?;
    rows.sort_by_key(|r| r.z);
    let Some(first) = rows.first() else { return Ok(()) };
    let board = first.board_id.clone();
    let now = now_ms();
    if to_front {
        let mut z = top_z(conn, &board)?;
        for mut r in rows {
            z += 1;
            r.z = z;
            r.updated_at = now;
            rec.put_card(conn, &r)?;
        }
    } else {
        let mut z = bottom_z(conn, &board)?;
        for mut r in rows.into_iter().rev() {
            z -= 1;
            r.z = z;
            r.updated_at = now;
            rec.put_card(conn, &r)?;
        }
    }
    Ok(())
}

/// A card's whole live subtree: column contents, boards it opens (recursively,
/// with everything on them) and lines attached to any of those cards.
pub fn subtree(conn: &Connection, root_ids: &[String]) -> Result<(Vec<CardRow>, Vec<BoardRow>)> {
    let mut cards = Vec::new();
    let mut boards = Vec::new();
    let mut seen = HashSet::new();
    let mut stack: Vec<CardRow> = Vec::new();
    for id in root_ids {
        if let Some(r) = CardRow::get_live(conn, id)? {
            stack.push(r);
        }
    }
    while let Some(row) = stack.pop() {
        if !seen.insert(row.id.clone()) {
            continue;
        }
        stack.extend(CardRow::query(conn, "parent_card_id = ?1 AND trashed_by IS NULL", [&row.id])?);
        if let Some(child) = row.child_board_id() {
            if seen.insert(child.to_string()) {
                if let Some(b) = BoardRow::get_live(conn, child)? {
                    stack.extend(CardRow::query(conn, "board_id = ?1 AND trashed_by IS NULL", [&b.id])?);
                    boards.push(b);
                }
            }
        }
        cards.push(row);
    }
    // Lines attached to a card in the subtree go with it.
    let ids: Vec<&str> = cards.iter().map(|c| c.id.as_str()).collect();
    if !ids.is_empty() {
        let json = serde_json::to_string(&ids)?;
        for line in CardRow::query(
            conn,
            "kind = 'line' AND trashed_by IS NULL AND (json_extract(data, '$.from.id') IN (SELECT value FROM json_each(?1)) \
             OR json_extract(data, '$.to.id') IN (SELECT value FROM json_each(?1)))",
            [json],
        )? {
            if seen.insert(line.id.clone()) {
                cards.push(line);
            }
        }
    }
    Ok((cards, boards))
}

/// Drop ids whose column is also listed: trashing or copying the column
/// already covers them.
fn top_level(conn: &Connection, ids: &[String]) -> Result<Vec<String>> {
    let set: HashSet<&String> = ids.iter().collect();
    let mut out = Vec::new();
    for id in ids {
        let Some(r) = CardRow::get_live(conn, id)? else { continue };
        if r.parent_card_id.as_ref().is_some_and(|p| set.contains(p)) {
            continue;
        }
        if !out.contains(id) {
            out.push(id.clone());
        }
    }
    Ok(out)
}

/// Move cards (and their subtrees) to the trash. Each root is one Trash entry.
pub fn trash(conn: &Connection, rec: &mut Recorder, ids: &[String]) -> Result<()> {
    let now = now_ms();
    for root in top_level(conn, ids)? {
        let (cards, boards) = subtree(conn, std::slice::from_ref(&root))?;
        for mut c in cards {
            c.trashed_by = Some(root.clone());
            c.trashed_at = Some(now);
            rec.put_card(conn, &c)?;
        }
        for mut b in boards {
            b.trashed_by = Some(root.clone());
            b.trashed_at = Some(now);
            rec.put_board(conn, &b)?;
        }
    }
    Ok(())
}

/// Bring back exactly the rows one delete trashed.
pub fn restore(conn: &Connection, rec: &mut Recorder, root_id: &str) -> Result<()> {
    let root = CardRow::get_any(conn, root_id)?.ok_or_else(|| Error::NotFound(format!("card {root_id}")))?;
    if root.trashed_by.as_deref() != Some(root_id) {
        return Err(Error::NotFound("that card is not in the trash".into()));
    }
    if BoardRow::get_live(conn, &root.board_id)?.is_none() {
        return Err(Error::Conflict("its board is in the trash — restore that first".into()));
    }
    for mut c in CardRow::query(conn, "trashed_by = ?1", [root_id])? {
        c.trashed_by = None;
        c.trashed_at = None;
        // A card whose column was deleted separately comes back loose.
        if c.id == root_id {
            if let Some(pid) = c.parent_card_id.clone() {
                if CardRow::get_live(conn, &pid)?.is_none() {
                    if let Some(col) = CardRow::get_any(conn, &pid)? {
                        c.x = col.x;
                        c.y = col.y;
                    }
                    c.parent_card_id = None;
                }
            }
        }
        rec.put_card(conn, &c)?;
    }
    for mut b in BoardRow::query(conn, "trashed_by = ?1", [root_id])? {
        b.trashed_by = None;
        b.trashed_at = None;
        rec.put_board(conn, &b)?;
    }
    Ok(())
}

/// A self-contained copy of cards (with subtrees) — the in-app clipboard and
/// the JSON export format.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub format: String,
    pub version: u32,
    /// Ids of the cards that were selected (top level of the snapshot).
    pub roots: Vec<String>,
    pub cards: Vec<SnapCard>,
    pub boards: Vec<SnapBoard>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapCard {
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapBoard {
    pub id: String,
    pub parent_board_id: Option<String>,
    pub title: String,
    pub color: Option<String>,
    pub icon: Option<String>,
}

pub const SNAPSHOT_FORMAT: &str = "milano-pen/cards";

pub fn snapshot(conn: &Connection, ids: &[String]) -> Result<Snapshot> {
    let roots = top_level(conn, ids)?;
    let (cards, boards) = subtree(conn, &roots)?;
    let inside: HashSet<&str> = cards.iter().map(|c| c.id.as_str()).collect();
    let cards = cards
        .iter()
        // A line whose other end stays behind is not copied.
        .filter(|c| c.kind != "line" || line_ends(&c.data).iter().all(|e| inside.contains(e.as_str())))
        .map(|c| SnapCard {
            id: c.id.clone(),
            board_id: c.board_id.clone(),
            parent_card_id: c.parent_card_id.clone(),
            kind: c.kind.clone(),
            x: c.x,
            y: c.y,
            w: c.w,
            h: c.h,
            z: c.z,
            color: c.color.clone(),
            data: c.data.clone(),
        })
        .collect();
    let boards = boards
        .into_iter()
        .map(|b| SnapBoard {
            id: b.id,
            parent_board_id: b.parent_board_id,
            title: b.title,
            color: b.color,
            icon: b.icon,
        })
        .collect();
    Ok(Snapshot { format: SNAPSHOT_FORMAT.into(), version: 1, roots, cards, boards })
}

fn line_ends(data: &Value) -> Vec<String> {
    ["from", "to"].iter().filter_map(|k| data.get(*k)?.get("id")?.as_str().map(String::from)).collect()
}

/// Insert a fresh copy of `snap` on `board_id`. The roots keep their relative
/// layout with their top-left corner at `at` (or offset by `offset` from where
/// they were when `at` is None). Returns the new root ids.
pub fn paste(
    conn: &Connection,
    rec: &mut Recorder,
    snap: &Snapshot,
    board_id: &str,
    at: Option<(f64, f64)>,
    offset: (f64, f64),
) -> Result<Vec<String>> {
    if snap.format != SNAPSHOT_FORMAT {
        return Err(Error::Invalid("not a Milano Pen card snapshot".into()));
    }
    BoardRow::get_live(conn, board_id)?.ok_or_else(|| Error::NotFound(format!("board {board_id}")))?;
    let mut map: HashMap<&str, String> = HashMap::new();
    for c in &snap.cards {
        map.insert(&c.id, new_id());
    }
    for b in &snap.boards {
        map.insert(&b.id, new_id());
    }
    let roots: HashSet<&str> = snap.roots.iter().map(String::as_str).collect();
    let root_cards: Vec<&SnapCard> = snap.cards.iter().filter(|c| roots.contains(c.id.as_str())).collect();
    let (min_x, min_y) = root_cards.iter().fold((f64::INFINITY, f64::INFINITY), |(x, y), c| (x.min(c.x), y.min(c.y)));
    let (dx, dy) = match at {
        Some((ax, ay)) if min_x.is_finite() => (ax - min_x, ay - min_y),
        _ => offset,
    };
    let now = now_ms();
    for b in &snap.boards {
        let row = BoardRow {
            id: map[b.id.as_str()].clone(),
            parent_board_id: Some(
                b.parent_board_id.as_deref().and_then(|p| map.get(p).cloned()).unwrap_or_else(|| board_id.to_string()),
            ),
            title: boards::clean_title(&b.title),
            color: b.color.clone(),
            icon: b.icon.clone(),
            created_at: now,
            updated_at: now,
            trashed_by: None,
            trashed_at: None,
        };
        rec.put_board(conn, &row)?;
    }
    let mut z = top_z(conn, board_id)?;
    let mut sorted: Vec<&SnapCard> = snap.cards.iter().collect();
    sorted.sort_by_key(|c| c.z);
    let mut created = Vec::new();
    for c in sorted {
        if !is_known_kind(&c.kind) {
            continue;
        }
        let is_root = roots.contains(c.id.as_str());
        let mut data = c.data.clone();
        remap_refs(&mut data, &map);
        let target_board = map.get(c.board_id.as_str()).cloned().unwrap_or_else(|| board_id.to_string());
        let on_target = target_board == board_id;
        let parent = c.parent_card_id.as_deref().and_then(|p| map.get(p).cloned());
        let row = CardRow {
            id: map[c.id.as_str()].clone(),
            board_id: target_board,
            parent_card_id: if is_root { None } else { parent },
            kind: c.kind.clone(),
            x: if is_root { c.x + dx } else { c.x },
            y: if is_root { c.y + dy } else { c.y },
            w: c.w.max(MIN_SIZE),
            h: c.h,
            z: if on_target {
                z += 1;
                z
            } else {
                c.z
            },
            color: c.color.clone(),
            data,
            search_text: String::new(),
            created_at: now,
            updated_at: now,
            trashed_by: None,
            trashed_at: None,
        };
        // Column contents and lines between pasted cards keep their offsets
        // relative to moved roots: shift free (non-root) cards on the target board.
        let row = if !is_root && on_target && row.parent_card_id.is_none() {
            CardRow { x: row.x + dx, y: row.y + dy, ..row }
        } else {
            row
        };
        rec.put_card(conn, &row)?;
        if is_root {
            created.push(row.id.clone());
        }
    }
    Ok(created)
}

fn remap_refs(data: &mut Value, map: &HashMap<&str, String>) {
    if let Some(Value::String(s)) = data.get_mut("childBoardId") {
        if let Some(n) = map.get(s.as_str()) {
            *s = n.clone();
        }
    }
    if let Some(Value::String(s)) = data.get_mut("targetBoardId") {
        if let Some(n) = map.get(s.as_str()) {
            *s = n.clone();
        }
    }
    for end in ["from", "to"] {
        if let Some(Value::String(s)) = data.get_mut(end).and_then(|e| e.get_mut("id")) {
            if let Some(n) = map.get(s.as_str()) {
                *s = n.clone();
            }
        }
    }
}
