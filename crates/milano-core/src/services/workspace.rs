//! The single entry point the app talks to: one SQLite connection, the undo
//! history and the in-app clipboard. Every mutation runs in a transaction,
//! is recorded for undo and returns a [`Patch`].

use std::path::{Path, PathBuf};
use std::time::Duration;

use rusqlite::{Connection, Transaction};
use serde::Serialize;
use serde_json::json;

use super::cards::Snapshot;
use super::history::{Entry, History, Recorder};
use super::settings::Settings;
use super::{backup, boards, cards, export, search, settings, trash};
use crate::database;
use crate::files::AssetStore;
use crate::models::{
    Asset, BoardUpdate, BoardView, CardUpdate, HistoryState, NewCard, Patch, SearchResult, TrashEntry,
};
use crate::{AppPaths, Error, Result};

pub struct Workspace {
    conn: Connection,
    history: History,
    clipboard: Option<Snapshot>,
    paths: Option<AppPaths>,
    store: Option<AssetStore>,
}

/// Result of importing files: the cards created, the assets to prefetch
/// thumbnails for, and one message per file that could not be imported.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    pub patch: Patch,
    #[serde(skip)]
    pub assets: Vec<Asset>,
    pub errors: Vec<String>,
}

pub const PASTE_OFFSET: f64 = 24.0;
const IMPORT_GAP: f64 = 24.0;
const IMPORT_PER_ROW: usize = 4;

impl Workspace {
    /// Open the user's workspace under `paths`, creating it on first run.
    pub fn open(paths: &AppPaths) -> Result<Self> {
        paths.ensure()?;
        let conn = database::open(&paths.database_file(), Some(&paths.backups))?;
        boards::ensure_root(&conn)?;
        Ok(Self {
            conn,
            history: History::default(),
            clipboard: None,
            store: Some(AssetStore::new(&paths.assets, &paths.thumbnails, paths.cache.join("tmp"))),
            paths: Some(paths.clone()),
        })
    }

    /// A throwaway in-memory workspace (tests).
    pub fn in_memory() -> Result<Self> {
        let conn = database::open_in_memory()?;
        boards::ensure_root(&conn)?;
        Ok(Self { conn, history: History::default(), clipboard: None, paths: None, store: None })
    }

    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    pub fn paths(&self) -> Option<&AppPaths> {
        self.paths.as_ref()
    }

    pub fn asset_store(&self) -> Option<&AssetStore> {
        self.store.as_ref()
    }

    fn mutate<F>(&mut self, label: &str, coalesce: Option<String>, f: F) -> Result<Patch>
    where
        F: FnOnce(&Transaction, &mut Recorder) -> Result<Vec<String>>,
    {
        let tx = self.conn.transaction()?;
        let mut rec = Recorder::default();
        let created = f(&tx, &mut rec)?;
        let changes = rec.finish(&tx)?;
        let mut patch = changes.patch(&tx)?;
        tx.commit()?;
        patch.created = created;
        patch.label = label.to_string();
        self.history.push(label, coalesce, changes);
        Ok(patch)
    }

    // ---- Boards ------------------------------------------------------------

    pub fn root_board_id(&self) -> Result<String> {
        boards::ensure_root(&self.conn)
    }

    pub fn open_board(&self, id: &str) -> Result<BoardView> {
        boards::open(&self.conn, id)
    }

    pub fn update_board(&mut self, id: &str, u: &BoardUpdate) -> Result<Patch> {
        self.mutate("Edit board", Some(format!("board:{id}")), |tx, rec| {
            boards::update(tx, rec, id, u)?;
            Ok(vec![])
        })
    }

    // ---- Cards -------------------------------------------------------------

    pub fn create_cards(&mut self, new: &[NewCard]) -> Result<Patch> {
        let label = if new.len() == 1 { format!("Add {}", new[0].kind) } else { format!("Add {} cards", new.len()) };
        self.mutate(&label, None, |tx, rec| new.iter().map(|n| cards::create(tx, rec, n)).collect())
    }

    /// `coalesce` merges consecutive updates with the same key (e.g. typing
    /// in one note) into one undo step.
    pub fn update_cards(&mut self, updates: &[CardUpdate], label: &str, coalesce: Option<String>) -> Result<Patch> {
        self.mutate(label, coalesce, |tx, rec| {
            cards::update(tx, rec, updates)?;
            Ok(vec![])
        })
    }

    pub fn restack(&mut self, ids: &[String], to_front: bool) -> Result<Patch> {
        let label = if to_front { "Bring to front" } else { "Send to back" };
        self.mutate(label, None, |tx, rec| {
            cards::restack(tx, rec, ids, to_front)?;
            Ok(vec![])
        })
    }

    pub fn trash_cards(&mut self, ids: &[String]) -> Result<Patch> {
        let label = if ids.len() == 1 { "Delete card".to_string() } else { format!("Delete {} cards", ids.len()) };
        self.mutate(&label, None, |tx, rec| {
            cards::trash(tx, rec, ids)?;
            Ok(vec![])
        })
    }

    pub fn restore_trashed(&mut self, root_id: &str) -> Result<Patch> {
        self.mutate("Restore from trash", None, |tx, rec| {
            cards::restore(tx, rec, root_id)?;
            Ok(vec![root_id.to_string()])
        })
    }

    pub fn duplicate(&mut self, ids: &[String]) -> Result<Patch> {
        let snap = cards::snapshot(&self.conn, ids)?;
        let Some(board) = snap.cards.iter().find(|c| snap.roots.contains(&c.id)).map(|c| c.board_id.clone()) else {
            return Ok(Patch::default());
        };
        self.mutate("Duplicate", None, |tx, rec| {
            cards::paste(tx, rec, &snap, &board, None, (PASTE_OFFSET, PASTE_OFFSET))
        })
    }

    /// Put a snapshot of the cards on the in-app clipboard. Returns how many
    /// cards it holds.
    pub fn copy(&mut self, ids: &[String]) -> Result<usize> {
        let snap = cards::snapshot(&self.conn, ids)?;
        let n = snap.roots.len();
        self.clipboard = Some(snap);
        Ok(n)
    }

    /// A snapshot of `ids` for a clipboard held outside this workspace (the
    /// app's clipboard is shared between open projects).
    pub fn snapshot(&self, ids: &[String]) -> Result<Snapshot> {
        cards::snapshot(&self.conn, ids)
    }

    /// Paste a snapshot taken from this or another workspace. Assets it
    /// references must already be here (see [`Workspace::adopt_assets`]).
    pub fn paste_snapshot(&mut self, board_id: &str, snap: &Snapshot, at: Option<(f64, f64)>) -> Result<Patch> {
        self.mutate("Paste", None, |tx, rec| cards::paste(tx, rec, snap, board_id, at, (PASTE_OFFSET, PASTE_OFFSET)))
    }

    /// The assets a snapshot's cards use, with the path of each original in
    /// this workspace's store (assets whose file is missing are skipped).
    pub fn snapshot_assets(&self, snap: &Snapshot) -> Result<Vec<(Asset, PathBuf)>> {
        let Some(store) = &self.store else { return Ok(vec![]) };
        let mut seen = std::collections::HashSet::new();
        let mut out = Vec::new();
        for c in &snap.cards {
            let Some(id) = c.data.get("assetId").and_then(|v| v.as_str()) else { continue };
            if !seen.insert(id.to_string()) {
                continue;
            }
            if let Some(asset) = store.get(&self.conn, id)? {
                let path = store.original_path(&asset.id, &asset.ext);
                if path.is_file() {
                    out.push((asset, path));
                }
            }
        }
        Ok(out)
    }

    /// Copy assets from another workspace's store into this one (originals
    /// and metadata; thumbnails are regenerated on demand). Returns the
    /// assets that were new here.
    pub fn adopt_assets(&mut self, assets: &[(Asset, PathBuf)]) -> Result<Vec<Asset>> {
        let store = self.store()?.clone();
        let mut added = Vec::new();
        for (asset, src) in assets {
            if store.adopt(&self.conn, asset, src)? {
                added.push(asset.clone());
            }
        }
        Ok(added)
    }

    pub fn has_clipboard(&self) -> bool {
        self.clipboard.as_ref().is_some_and(|s| !s.roots.is_empty())
    }

    /// Paste the clipboard with its top-left at `at` (or offset from the
    /// originals' position when `at` is None).
    pub fn paste(&mut self, board_id: &str, at: Option<(f64, f64)>) -> Result<Patch> {
        let Some(snap) = self.clipboard.clone() else { return Ok(Patch::default()) };
        self.mutate("Paste", None, |tx, rec| cards::paste(tx, rec, &snap, board_id, at, (PASTE_OFFSET, PASTE_OFFSET)))
    }

    // ---- History -----------------------------------------------------------

    pub fn undo(&mut self) -> Result<Option<Patch>> {
        let Some(entry) = self.history.pop_undo() else { return Ok(None) };
        let inverse = entry.changes.inverse();
        let patch = self.apply_changes(&inverse, &entry);
        match patch {
            Ok(p) => {
                self.history.push_redo(entry);
                Ok(Some(p))
            }
            Err(e) => {
                self.history.push_undo_raw(entry);
                Err(e)
            }
        }
    }

    pub fn redo(&mut self) -> Result<Option<Patch>> {
        let Some(entry) = self.history.pop_redo() else { return Ok(None) };
        let changes = entry.changes.clone();
        match self.apply_changes(&changes, &entry) {
            Ok(p) => {
                self.history.push_undo_raw(entry);
                Ok(Some(p))
            }
            Err(e) => {
                self.history.push_redo(entry);
                Err(e)
            }
        }
    }

    fn apply_changes(&mut self, cs: &super::history::ChangeSet, entry: &Entry) -> Result<Patch> {
        let tx = self.conn.transaction()?;
        cs.apply(&tx)?;
        let mut patch = cs.patch(&tx)?;
        tx.commit()?;
        patch.label = entry.label.clone();
        Ok(patch)
    }

    pub fn history_state(&self) -> HistoryState {
        self.history.state()
    }

    // ---- Search, trash -----------------------------------------------------

    pub fn search(&self, q: &str) -> Result<Vec<SearchResult>> {
        search::search(&self.conn, q, 50)
    }

    pub fn trash_list(&self) -> Result<Vec<TrashEntry>> {
        trash::list(&self.conn)
    }

    /// Permanently delete the trash. Undo history is cleared, since it could
    /// otherwise resurrect rows whose files are gone.
    pub fn empty_trash(&mut self) -> Result<trash::EmptyReport> {
        let tx = self.conn.transaction()?;
        let report = trash::empty(&tx, self.store.as_ref())?;
        tx.commit()?;
        self.history.clear();
        Ok(report)
    }

    // ---- Import / export ---------------------------------------------------

    pub fn export_markdown(&self, board_id: &str) -> Result<String> {
        export::markdown(&self.conn, board_id)
    }

    pub fn export_json(&self, board_id: &str) -> Result<String> {
        Ok(serde_json::to_string_pretty(&export::json(&self.conn, board_id)?)?)
    }

    pub fn import_json(&mut self, board_id: &str, json: &str, at: (f64, f64)) -> Result<Patch> {
        let snap: Snapshot =
            serde_json::from_str(json).map_err(|e| Error::Invalid(format!("not a board export: {e}")))?;
        self.mutate("Import board", None, |tx, rec| cards::paste(tx, rec, &snap, board_id, Some(at), (0.0, 0.0)))
    }

    fn store(&self) -> Result<&AssetStore> {
        self.store.as_ref().ok_or_else(|| Error::Invalid("this workspace has no asset store".into()))
    }

    fn card_for_asset(board_id: &str, asset: &Asset, x: f64, y: f64) -> NewCard {
        if asset.is_image() {
            let (iw, ih) = (asset.width.unwrap_or(0) as f64, asset.height.unwrap_or(0) as f64);
            let w = if iw > 0.0 { iw.clamp(cards::MIN_SIZE * 2.0, 280.0) } else { 280.0 };
            let h = if iw > 0.0 && ih > 0.0 { Some(w * ih / iw) } else { Some(w * 0.75) };
            NewCard {
                board_id: board_id.to_string(),
                kind: "image".into(),
                x,
                y,
                w: Some(w),
                h,
                data: Some(json!({
                    "assetId": asset.id, "name": asset.original_name,
                    "width": asset.width, "height": asset.height, "mime": asset.mime,
                })),
                ..Default::default()
            }
        } else {
            NewCard {
                board_id: board_id.to_string(),
                kind: "file".into(),
                x,
                y,
                w: Some(240.0),
                data: Some(json!({
                    "assetId": asset.id, "name": asset.original_name, "mime": asset.mime, "size": asset.size,
                })),
                ..Default::default()
            }
        }
    }

    /// Copy files into the asset store and create one card each, laid out in
    /// a grid from `at`.
    pub fn import_files(&mut self, board_id: &str, files: &[PathBuf], at: (f64, f64)) -> Result<ImportResult> {
        let store = self.store()?.clone();
        let mut assets = Vec::new();
        let mut errors = Vec::new();
        for f in files {
            match store.import_path(&self.conn, f) {
                Ok(a) => assets.push(a),
                Err(e) => errors.push(format!("{}: {e}", f.file_name().unwrap_or_default().to_string_lossy())),
            }
        }
        self.cards_for_assets(board_id, assets, errors, at)
    }

    pub fn import_bytes(&mut self, board_id: &str, name: &str, bytes: &[u8], at: (f64, f64)) -> Result<ImportResult> {
        let asset = self.store()?.clone().import_bytes(&self.conn, name, bytes)?;
        self.cards_for_assets(board_id, vec![asset], vec![], at)
    }

    fn cards_for_assets(
        &mut self,
        board_id: &str,
        assets: Vec<Asset>,
        errors: Vec<String>,
        at: (f64, f64),
    ) -> Result<ImportResult> {
        let mut new = Vec::new();
        let (mut x, mut y, mut row_h) = (at.0, at.1, 0.0_f64);
        for (i, a) in assets.iter().enumerate() {
            if i > 0 && i.is_multiple_of(IMPORT_PER_ROW) {
                x = at.0;
                y += row_h + IMPORT_GAP;
                row_h = 0.0;
            }
            let card = Self::card_for_asset(board_id, a, x, y);
            x += card.w.unwrap_or(240.0) + IMPORT_GAP;
            row_h = row_h.max(card.h.unwrap_or(80.0));
            new.push(card);
        }
        let patch = if new.is_empty() { Patch::default() } else { self.create_cards(&new)? };
        Ok(ImportResult { patch, assets, errors })
    }

    pub fn asset(&self, id: &str) -> Result<Option<Asset>> {
        self.store()?.get(&self.conn, id)
    }

    // ---- Backups -----------------------------------------------------------

    pub fn backup_to(&self, dest: &Path) -> Result<()> {
        backup::backup_to(&self.conn, dest)
    }

    /// Daily automatic backup (keeps the last 10).
    pub fn auto_backup(&self) -> Result<Option<PathBuf>> {
        let Some(p) = &self.paths else { return Ok(None) };
        backup::auto_backup(&self.conn, &p.backups, Duration::from_secs(24 * 3600), 10)
    }

    /// Replace the database with a validated backup. The current database is
    /// first saved to `backups/pre-restore-*.sqlite`.
    pub fn restore_backup(&mut self, src: &Path) -> Result<()> {
        let paths = self.paths.clone().ok_or_else(|| Error::Invalid("no database file to restore into".into()))?;
        backup::validate(src)?;
        let stamp = chrono::Utc::now().format("%Y%m%d-%H%M%S");
        backup::backup_to(&self.conn, &paths.backups.join(format!("pre-restore-{stamp}.sqlite")))?;
        let db = paths.database_file();
        let staged = db.with_extension("restore-tmp");
        std::fs::copy(src, &staged)?;
        let old = std::mem::replace(&mut self.conn, Connection::open_in_memory()?);
        old.close().map_err(|(_, e)| Error::Db(e))?;
        for ext in ["sqlite-wal", "sqlite-shm"] {
            let _ = std::fs::remove_file(db.with_extension(ext));
        }
        std::fs::rename(&staged, &db)?;
        self.conn = database::open(&db, Some(&paths.backups))?;
        boards::ensure_root(&self.conn)?;
        self.history.clear();
        self.clipboard = None;
        Ok(())
    }

    // ---- Settings ----------------------------------------------------------

    pub fn settings(&self) -> Result<Settings> {
        settings::load(&self.conn)
    }

    pub fn save_settings(&self, s: &Settings) -> Result<Settings> {
        settings::save(&self.conn, s)
    }

    /// Run `f` in one recorded transaction (used by generators).
    pub fn batch<F>(&mut self, label: &str, f: F) -> Result<Patch>
    where
        F: FnOnce(&Transaction, &mut Recorder, Option<&AssetStore>) -> Result<Vec<String>>,
    {
        let store = self.store.clone();
        self.mutate(label, None, |tx, rec| f(tx, rec, store.as_ref()))
    }
}
