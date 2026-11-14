//! Types exchanged with the frontend (camelCase JSON) and raw table rows.

mod rows;

pub use rows::{BoardRow, CardRow};

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Every card kind the app knows. The frontend renders one component per kind.
pub const CARD_KINDS: &[&str] = &[
    "note", "document", "todo", "image", "file", "link", "column", "board", "table", "comment", "swatch", "sketch",
    "line", "shortcut",
];

pub fn is_known_kind(kind: &str) -> bool {
    CARD_KINDS.contains(&kind)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Board {
    pub id: String,
    pub parent_board_id: Option<String>,
    pub title: String,
    pub color: Option<String>,
    pub icon: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    /// Live top-level cards on this board (shown on its board card).
    pub card_count: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Card {
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
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Asset {
    pub id: String,
    pub ext: String,
    pub mime: String,
    pub size: i64,
    pub width: Option<i64>,
    pub height: Option<i64>,
    pub original_name: String,
    pub created_at: i64,
}

impl Asset {
    pub fn is_image(&self) -> bool {
        self.mime.starts_with("image/")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Crumb {
    pub id: String,
    pub title: String,
    pub color: Option<String>,
}

/// Everything the canvas needs to show one board.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BoardView {
    pub board: Board,
    pub breadcrumb: Vec<Crumb>,
    pub cards: Vec<Card>,
    /// Boards opened by the board cards on this board (title, colour, count).
    pub child_boards: Vec<Board>,
}

/// The incremental result of any mutation (and of undo/redo). The UI applies
/// it to its store instead of reloading the board.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Patch {
    /// Cards that now exist (created or changed), wherever they live.
    pub cards: Vec<Card>,
    /// Cards that no longer exist (trashed or deleted).
    pub removed_cards: Vec<String>,
    pub boards: Vec<Board>,
    pub removed_boards: Vec<String>,
    /// Ids created by this operation, so the UI can select them.
    pub created: Vec<String>,
    /// Human label of the operation (shown in undo/redo toasts).
    pub label: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewCard {
    pub board_id: String,
    #[serde(default)]
    pub parent_card_id: Option<String>,
    pub kind: String,
    pub x: f64,
    pub y: f64,
    #[serde(default)]
    pub w: Option<f64>,
    #[serde(default)]
    pub h: Option<f64>,
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub data: Option<Value>,
}

/// A partial update. Absent fields are left alone. `color: ""` clears the
/// colour, `h: 0`/negative clears a fixed height (auto height). Keys in
/// `data` are merged shallowly; a `null` value removes that key.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CardUpdate {
    pub id: String,
    #[serde(default)]
    pub x: Option<f64>,
    #[serde(default)]
    pub y: Option<f64>,
    #[serde(default)]
    pub w: Option<f64>,
    #[serde(default)]
    pub h: Option<f64>,
    #[serde(default)]
    pub z: Option<i64>,
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub data: Option<Value>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BoardUpdate {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    /// "board" or "card".
    pub kind: String,
    pub id: String,
    pub board_id: String,
    pub card_kind: Option<String>,
    pub title: String,
    pub path: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrashEntry {
    pub id: String,
    pub kind: String,
    pub title: String,
    pub board_id: String,
    pub board_title: String,
    pub restorable: bool,
    pub trashed_at: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryState {
    pub can_undo: bool,
    pub can_redo: bool,
    pub undo_label: Option<String>,
    pub redo_label: Option<String>,
}
