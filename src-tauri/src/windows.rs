//! Windows and the projects they show. The `main` window (from
//! tauri.conf.json) shows the Library; File ▸ New Project / Open Project
//! open a project folder in a window of its own. Each window flushes its
//! pending saves before it closes, and every window does before the app quits.

use std::path::Path;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use tauri::{AppHandle, Emitter, EventTarget, Manager, Runtime, WebviewUrl, WebviewWindowBuilder};

use milano_core::Result;

use crate::state::{project_name, AppState, Project};

/// How long closing or quitting waits for a webview to flush its saves.
const FLUSH_TIMEOUT: Duration = Duration::from_secs(3);

/// Show `project` in a window: focus the one already showing it, else open
/// a new one.
pub fn show_project<R: Runtime>(app: &AppHandle<R>, project: Arc<Project>, label_prefix: &str) -> Result<()> {
    let state = app.state::<AppState>();
    if let Some(label) = state.window_for(&project.key) {
        if let Some(w) = app.get_webview_window(&label) {
            let _ = w.unminimize();
            let _ = w.set_focus();
            return Ok(());
        }
    }
    let label = state.next_label(label_prefix);
    // Attach first so the webview's very first command finds its project.
    state.attach(&label, project.clone())?;
    let builder = WebviewWindowBuilder::new(app, &label, WebviewUrl::App("index.html".into()))
        .title(&project.name)
        .inner_size(1280.0, 820.0)
        .min_inner_size(720.0, 480.0)
        // The webview handles drops itself (files, browser images, links).
        .disable_drag_drop_handler();
    #[cfg(target_os = "macos")]
    let builder = builder.title_bar_style(tauri::TitleBarStyle::Overlay).hidden_title(true);
    if let Err(e) = builder.build() {
        state.detach(&label);
        return Err(milano_core::Error::Invalid(format!("could not open a window: {e}")));
    }
    Ok(())
}

/// Open (or create) the project folder at `root` in its own window.
pub fn open_project_at<R: Runtime>(app: &AppHandle<R>, root: &Path) -> Result<()> {
    let state = app.state::<AppState>();
    let key = std::fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
    let project = match state.open_project(&key) {
        Some(p) => p,
        None => {
            let p = Project::open(root, project_name(root), false)?;
            let paths = p.paths.clone();
            // Same daily safety copy as the Library, off the main thread.
            std::thread::spawn(move || {
                if let Err(e) = milano_core::services::backup::auto_backup_file(&paths.database_file(), &paths.backups)
                {
                    log::warn!("automatic backup of {}: {e}", paths.root.display());
                }
            });
            Arc::new(p)
        }
    };
    show_project(app, project, "project")
}

/// A window was asked to close: let its webview flush first.
pub fn request_close<R: Runtime>(app: &AppHandle<R>, label: &str) {
    let _ = app.emit_to(EventTarget::webview_window(label), "window-will-close", ());
    let handle = app.clone();
    let label = label.to_string();
    std::thread::spawn(move || {
        std::thread::sleep(FLUSH_TIMEOUT);
        allow_close(&handle, &label);
    });
}

/// The webview flushed (or did not answer in time): close its window.
pub fn allow_close<R: Runtime>(app: &AppHandle<R>, label: &str) {
    let state = app.state::<AppState>();
    if let Ok(mut c) = state.closing.lock() {
        c.insert(label.to_string());
    }
    if let Some(w) = app.get_webview_window(label) {
        let _ = w.close();
    }
}

/// Ask every webview to flush pending saves, then quit (or quit anyway after
/// a timeout, so a hung webview can never block quitting).
pub fn request_quit<R: Runtime>(app: &AppHandle<R>) {
    let state = app.state::<AppState>();
    let labels: Vec<String> =
        state.window_labels().into_iter().filter(|l| app.get_webview_window(l).is_some()).collect();
    if labels.is_empty() {
        state.ready_to_quit.store(true, Ordering::SeqCst);
        app.exit(0);
        return;
    }
    if let Ok(mut p) = state.quit_pending.lock() {
        p.extend(labels.iter().cloned());
    }
    for l in &labels {
        let _ = app.emit_to(EventTarget::webview_window(l), "app-will-quit", ());
    }
    let handle = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(FLUSH_TIMEOUT);
        handle.state::<AppState>().ready_to_quit.store(true, Ordering::SeqCst);
        handle.exit(0);
    });
}

/// Window `label` flushed for quitting; quit once all have.
pub fn quit_flushed<R: Runtime>(app: &AppHandle<R>, label: &str) {
    let state = app.state::<AppState>();
    let done = state.quit_pending.lock().map(|mut p| {
        p.remove(label);
        p.is_empty()
    });
    if done.unwrap_or(true) {
        state.ready_to_quit.store(true, Ordering::SeqCst);
        app.exit(0);
    }
}
