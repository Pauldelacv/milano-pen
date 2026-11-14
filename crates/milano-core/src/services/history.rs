//! Row-level change capture and the undo/redo stacks.
//!
//! Every mutation writes rows through a [`Recorder`], which snapshots each row
//! before its first write and again when the operation finishes. The
//! resulting [`ChangeSet`] is both the [`Patch`] sent to the UI and the undo
//! entry: undo writes the "before" rows back, redo the "after" rows. No
//! operation needs hand-written inverse logic.

use std::collections::HashSet;
use std::time::{Duration, Instant};

use rusqlite::Connection;

use crate::models::{BoardRow, CardRow, HistoryState, Patch};
use crate::Result;

#[derive(Debug, Clone)]
pub struct RowChange<T> {
    pub id: String,
    pub before: Option<T>,
    pub after: Option<T>,
}

#[derive(Debug, Clone, Default)]
pub struct ChangeSet {
    pub cards: Vec<RowChange<CardRow>>,
    pub boards: Vec<RowChange<BoardRow>>,
}

impl ChangeSet {
    /// True when nothing actually changed (e.g. a no-op move).
    pub fn is_empty(&self) -> bool {
        self.cards.iter().all(|c| c.before == c.after) && self.boards.iter().all(|b| b.before == b.after)
    }

    pub fn inverse(&self) -> ChangeSet {
        ChangeSet {
            cards: self
                .cards
                .iter()
                .map(|c| RowChange { id: c.id.clone(), before: c.after.clone(), after: c.before.clone() })
                .collect(),
            boards: self
                .boards
                .iter()
                .map(|b| RowChange { id: b.id.clone(), before: b.after.clone(), after: b.before.clone() })
                .collect(),
        }
    }

    /// Write every row's "after" state.
    pub fn apply(&self, conn: &Connection) -> Result<()> {
        for b in &self.boards {
            match &b.after {
                Some(row) => row.write(conn)?,
                None => BoardRow::delete(conn, &b.id)?,
            }
        }
        for c in &self.cards {
            match &c.after {
                Some(row) => row.write(conn)?,
                None => CardRow::delete(conn, &c.id)?,
            }
        }
        Ok(())
    }

    /// The UI patch describing the "after" state.
    pub fn patch(&self, conn: &Connection) -> Result<Patch> {
        let mut patch = Patch::default();
        let mut boards_to_refresh: Vec<String> = Vec::new();
        for c in &self.cards {
            for row in [&c.before, &c.after].into_iter().flatten() {
                boards_to_refresh.push(row.board_id.clone());
            }
            match &c.after {
                Some(row) if row.is_live() => patch.cards.push(row.to_card()),
                _ => patch.removed_cards.push(c.id.clone()),
            }
        }
        let mut seen = HashSet::new();
        for b in &self.boards {
            seen.insert(b.id.clone());
            match &b.after {
                Some(row) if row.is_live() => patch.boards.push(row.to_board(conn)?),
                _ => patch.removed_boards.push(b.id.clone()),
            }
        }
        // Card counts of boards that gained or lost cards.
        for id in boards_to_refresh {
            if seen.insert(id.clone()) {
                if let Some(row) = BoardRow::get_live(conn, &id)? {
                    patch.boards.push(row.to_board(conn)?);
                }
            }
        }
        Ok(patch)
    }

    /// Fold a later change set into this one: earliest "before", latest "after".
    fn absorb(&mut self, later: ChangeSet) {
        for c in later.cards {
            match self.cards.iter_mut().find(|e| e.id == c.id) {
                Some(e) => e.after = c.after,
                None => self.cards.push(c),
            }
        }
        for b in later.boards {
            match self.boards.iter_mut().find(|e| e.id == b.id) {
                Some(e) => e.after = b.after,
                None => self.boards.push(b),
            }
        }
    }
}

/// Collects before/after snapshots for one operation.
#[derive(Default)]
pub struct Recorder {
    cards: Vec<(String, Option<CardRow>)>,
    boards: Vec<(String, Option<BoardRow>)>,
    seen_cards: HashSet<String>,
    seen_boards: HashSet<String>,
}

impl Recorder {
    pub fn put_card(&mut self, conn: &Connection, row: &CardRow) -> Result<()> {
        if self.seen_cards.insert(row.id.clone()) {
            self.cards.push((row.id.clone(), CardRow::get_any(conn, &row.id)?));
        }
        row.write(conn)
    }

    pub fn put_board(&mut self, conn: &Connection, row: &BoardRow) -> Result<()> {
        if self.seen_boards.insert(row.id.clone()) {
            self.boards.push((row.id.clone(), BoardRow::get_any(conn, &row.id)?));
        }
        row.write(conn)
    }

    pub fn finish(self, conn: &Connection) -> Result<ChangeSet> {
        let mut cs = ChangeSet::default();
        for (id, before) in self.cards {
            let after = CardRow::get_any(conn, &id)?;
            cs.cards.push(RowChange { id, before, after });
        }
        for (id, before) in self.boards {
            let after = BoardRow::get_any(conn, &id)?;
            cs.boards.push(RowChange { id, before, after });
        }
        Ok(cs)
    }
}

#[derive(Debug, Clone)]
pub struct Entry {
    pub label: String,
    pub coalesce: Option<String>,
    pub at: Instant,
    pub changes: ChangeSet,
}

/// In-memory undo/redo stacks for this session.
pub struct History {
    undo: Vec<Entry>,
    redo: Vec<Entry>,
    limit: usize,
    coalesce_window: Duration,
}

impl Default for History {
    fn default() -> Self {
        Self::new(500)
    }
}

impl History {
    pub fn new(limit: usize) -> Self {
        Self { undo: Vec::new(), redo: Vec::new(), limit, coalesce_window: Duration::from_millis(1500) }
    }

    /// Record a finished operation. Consecutive operations sharing a
    /// coalescing key within the window (typing in one note) merge into one
    /// undo step.
    pub fn push(&mut self, label: &str, coalesce: Option<String>, changes: ChangeSet) {
        if changes.is_empty() {
            return;
        }
        self.redo.clear();
        let now = Instant::now();
        if let (Some(key), Some(top)) = (&coalesce, self.undo.last_mut()) {
            if top.coalesce.as_ref() == Some(key) && now.duration_since(top.at) < self.coalesce_window {
                top.changes.absorb(changes);
                top.at = now;
                return;
            }
        }
        self.undo.push(Entry { label: label.to_string(), coalesce, at: now, changes });
        if self.undo.len() > self.limit {
            let excess = self.undo.len() - self.limit;
            self.undo.drain(..excess);
        }
    }

    pub fn pop_undo(&mut self) -> Option<Entry> {
        self.undo.pop()
    }

    pub fn pop_redo(&mut self) -> Option<Entry> {
        self.redo.pop()
    }

    /// Put an entry back after it was undone (onto redo) or redone (onto undo).
    pub fn push_redo(&mut self, mut e: Entry) {
        e.coalesce = None;
        self.redo.push(e);
    }

    pub fn push_undo_raw(&mut self, mut e: Entry) {
        e.coalesce = None;
        self.undo.push(e);
    }

    pub fn clear(&mut self) {
        self.undo.clear();
        self.redo.clear();
    }

    pub fn state(&self) -> HistoryState {
        HistoryState {
            can_undo: !self.undo.is_empty(),
            can_redo: !self.redo.is_empty(),
            undo_label: self.undo.last().map(|e| e.label.clone()),
            redo_label: self.redo.last().map(|e| e.label.clone()),
        }
    }
}
