use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use milano_core::images::Thumbnailer;
use milano_core::{AppPaths, Error, Result, Workspace};

/// Shared application state, managed by Tauri.
pub struct AppState {
    pub workspace: Arc<Mutex<Workspace>>,
    pub thumbnails: Arc<Thumbnailer>,
    pub paths: AppPaths,
    /// Paths the user handed us (Finder drop). Import commands accept only these.
    pub granted_paths: Mutex<HashSet<PathBuf>>,
    /// Set once the webview has flushed pending saves; quitting is then allowed.
    pub ready_to_quit: AtomicBool,
}

impl AppState {
    /// Run `f` with the workspace on a blocking thread: database work never
    /// runs on the main (UI) thread.
    pub async fn with<T, F>(&self, f: F) -> Result<T>
    where
        T: Send + 'static,
        F: FnOnce(&mut Workspace) -> Result<T> + Send + 'static,
    {
        let ws = self.workspace.clone();
        tauri::async_runtime::spawn_blocking(move || {
            let mut guard = ws.lock().map_err(|_| Error::Conflict("the workspace is unavailable".into()))?;
            f(&mut guard)
        })
        .await
        .map_err(|e| Error::Invalid(format!("background task failed: {e}")))?
    }

    /// Generate thumbnails for freshly imported images in the background.
    pub fn prefetch_thumbnails(&self, asset_ids: Vec<String>) {
        let thumbs = self.thumbnails.clone();
        tauri::async_runtime::spawn(async move {
            for id in asset_ids {
                if let Err(e) = thumbs.ensure(&id).await {
                    log::warn!("thumbnail prefetch for {id}: {e}");
                }
            }
        });
    }
}
