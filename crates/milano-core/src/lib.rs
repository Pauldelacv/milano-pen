//! Milano Pen's application core.
//!
//! Everything that touches user data lives here, independent of Tauri so it
//! can be tested with a plain `cargo test`: the SQLite schema and migrations,
//! board/card services, the undo/redo history, the content-addressed asset
//! store and the thumbnail pipeline.

pub mod database;
pub mod error;
pub mod files;
pub mod images;
pub mod models;
pub mod paths;
pub mod services;

pub use error::{Error, Result};
pub use paths::AppPaths;
pub use services::workspace::Workspace;

/// Milliseconds since the Unix epoch.
pub fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

/// A new time-ordered identifier.
pub fn new_id() -> String {
    uuid::Uuid::now_v7().to_string()
}
