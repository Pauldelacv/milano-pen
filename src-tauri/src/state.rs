use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use serde::Serialize;

use milano_core::images::Thumbnailer;
use milano_core::services::cards::Snapshot;
use milano_core::{AppPaths, Error, Result, Workspace};

/// Hot thumbnails kept in memory per open project.
const THUMB_CACHE_BYTES: usize = 64 * 1024 * 1024;
/// Folder extension of a project created with File ▸ New Project.
pub const PROJECT_EXT: &str = "milanopen";

/// One open workspace: the Library (Application Support) or a project folder.
pub struct Project {
    /// Canonical root path: two windows never open the same project twice.
    pub key: PathBuf,
    pub name: String,
    pub is_library: bool,
    pub paths: AppPaths,
    pub workspace: Arc<Mutex<Workspace>>,
    pub thumbnails: Arc<Thumbnailer>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectInfo {
    pub name: String,
    pub path: String,
    pub is_library: bool,
}

impl Project {
    pub fn open(root: &Path, name: String, is_library: bool) -> Result<Self> {
        let paths = AppPaths::new(root);
        let workspace = Workspace::open(&paths)?;
        let store = workspace.asset_store().cloned().ok_or_else(|| Error::Invalid("no asset store".into()))?;
        let thumbnails = Thumbnailer::new(store, Thumbnailer::default_workers(), THUMB_CACHE_BYTES);
        let key = std::fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
        Ok(Self {
            key,
            name,
            is_library,
            paths,
            workspace: Arc::new(Mutex::new(workspace)),
            thumbnails: Arc::new(thumbnails),
        })
    }

    pub fn info(&self) -> ProjectInfo {
        ProjectInfo {
            name: self.name.clone(),
            path: self.paths.root.to_string_lossy().into(),
            is_library: self.is_library,
        }
    }

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

/// Cards copied in any window, pasteable in any other.
pub struct Clip {
    pub snapshot: Snapshot,
    /// Kept alive so its files can still be copied after its window closes.
    pub source: Arc<Project>,
}

/// Shared application state, managed by Tauri.
pub struct AppState {
    /// The default workspace in Application Support; holds the app settings.
    pub library: Arc<Project>,
    /// Window label → the project it shows.
    windows: Mutex<HashMap<String, Arc<Project>>>,
    pub clipboard: Mutex<Option<Clip>>,
    /// Windows whose webview has flushed its saves and may now close.
    pub closing: Mutex<HashSet<String>>,
    /// While quitting: the windows that have not flushed yet.
    pub quit_pending: Mutex<HashSet<String>>,
    /// Set once every webview has flushed; quitting is then allowed.
    pub ready_to_quit: AtomicBool,
    window_seq: AtomicU64,
}

fn lock<T>(m: &Mutex<T>) -> Result<MutexGuard<'_, T>> {
    m.lock().map_err(|_| Error::Conflict("the app state is unavailable".into()))
}

impl AppState {
    pub fn new(library: Project) -> Self {
        let library = Arc::new(library);
        let windows = HashMap::from([("main".to_string(), library.clone())]);
        Self {
            library,
            windows: Mutex::new(windows),
            clipboard: Mutex::new(None),
            closing: Mutex::new(HashSet::new()),
            quit_pending: Mutex::new(HashSet::new()),
            ready_to_quit: AtomicBool::new(false),
            window_seq: AtomicU64::new(1),
        }
    }

    /// The project shown by window `label`.
    pub fn project(&self, label: &str) -> Result<Arc<Project>> {
        lock(&self.windows)?.get(label).cloned().ok_or_else(|| Error::NotFound(format!("window {label}")))
    }

    /// The window already showing the project at `key`, if any.
    pub fn window_for(&self, key: &Path) -> Option<String> {
        let windows = self.windows.lock().ok()?;
        windows.iter().find(|(_, p)| p.key == key).map(|(l, _)| l.clone())
    }

    /// An open project at `key` (shared by its windows), if any.
    pub fn open_project(&self, key: &Path) -> Option<Arc<Project>> {
        if self.library.key == key {
            return Some(self.library.clone());
        }
        let windows = self.windows.lock().ok()?;
        windows.values().find(|p| p.key == key).cloned()
    }

    pub fn next_label(&self, prefix: &str) -> String {
        format!("{prefix}-{}", self.window_seq.fetch_add(1, Ordering::SeqCst))
    }

    pub fn attach(&self, label: &str, project: Arc<Project>) -> Result<()> {
        lock(&self.windows)?.insert(label.to_string(), project);
        Ok(())
    }

    /// Forget a closed window. Its project's database closes when the last
    /// reference (window or clipboard) goes away.
    pub fn detach(&self, label: &str) {
        if let Ok(mut w) = self.windows.lock() {
            w.remove(label);
        }
        if let Ok(mut c) = self.closing.lock() {
            c.remove(label);
        }
    }

    pub fn window_labels(&self) -> Vec<String> {
        self.windows.lock().map(|w| w.keys().cloned().collect()).unwrap_or_default()
    }
}

/// The display name of a project folder ("Trip.milanopen" → "Trip").
pub fn project_name(root: &Path) -> String {
    let stem = if root.extension().is_some_and(|e| e == PROJECT_EXT) { root.file_stem() } else { root.file_name() };
    stem.map(|s| s.to_string_lossy().into_owned()).filter(|s| !s.is_empty()).unwrap_or_else(|| "Project".into())
}

/// True if `dir` holds a Milano Pen workspace.
pub fn is_project_dir(dir: &Path) -> bool {
    AppPaths::new(dir).database_file().is_file()
}

/// Where File ▸ New Project creates its folder, from the path picked in the
/// save dialog: the `.milanopen` extension is added when missing.
pub fn new_project_root(picked: &Path) -> PathBuf {
    if picked.extension().is_some_and(|e| e == PROJECT_EXT) {
        picked.to_path_buf()
    } else {
        let mut name = picked.file_name().map(|n| n.to_os_string()).unwrap_or_else(|| "Untitled".into());
        name.push(format!(".{PROJECT_EXT}"));
        picked.with_file_name(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_names_and_roots() {
        assert_eq!(project_name(Path::new("/x/Trip.milanopen")), "Trip");
        assert_eq!(project_name(Path::new("/x/Plain folder")), "Plain folder");
        assert_eq!(new_project_root(Path::new("/x/Trip")), PathBuf::from("/x/Trip.milanopen"));
        assert_eq!(new_project_root(Path::new("/x/Trip.milanopen")), PathBuf::from("/x/Trip.milanopen"));
        assert_eq!(new_project_root(Path::new("/x/v1.2")), PathBuf::from("/x/v1.2.milanopen"));
    }

    #[test]
    fn projects_are_detected_by_their_database() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!is_project_dir(dir.path()));
        Workspace::open(&AppPaths::new(dir.path())).unwrap();
        assert!(is_project_dir(dir.path()));
    }
}
