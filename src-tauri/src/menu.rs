//! The native menu bar. Custom items emit a `menu` event carrying their id;
//! the webview performs the action (it knows the selection and the board).
//! Cut/Copy/Paste/Select All are the native items so text fields behave as
//! in every Mac app; the webview turns their DOM events into card actions.

use tauri::menu::{AboutMetadata, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::{AppHandle, Runtime};

fn item<R: Runtime>(app: &AppHandle<R>, id: &str, text: &str, accel: Option<&str>) -> tauri::Result<MenuItem<R>> {
    MenuItem::with_id(app, id, text, true, accel)
}

pub fn build<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    let sep = || PredefinedMenuItem::separator(app);
    let about = AboutMetadata {
        name: Some("Milano Pen".into()),
        version: Some(app.package_info().version.to_string()),
        comments: Some("A local-first visual workspace.".into()),
        license: Some("MIT".into()),
        website: Some("https://github.com/pauldelacv/milano-pen".into()),
        ..Default::default()
    };

    let app_menu = Submenu::with_items(
        app,
        "Milano Pen",
        true,
        &[
            &PredefinedMenuItem::about(app, Some("About Milano Pen"), Some(about))?,
            &sep()?,
            &item(app, "settings", "Settings…", Some("CmdOrCtrl+,"))?,
            &sep()?,
            &PredefinedMenuItem::services(app, None)?,
            &sep()?,
            &PredefinedMenuItem::hide(app, None)?,
            &PredefinedMenuItem::hide_others(app, None)?,
            &PredefinedMenuItem::show_all(app, None)?,
            &sep()?,
            &PredefinedMenuItem::quit(app, None)?,
        ],
    )?;

    let file = Submenu::with_items(
        app,
        "File",
        true,
        &[
            &item(app, "new-note", "New Note", Some("CmdOrCtrl+N"))?,
            &item(app, "new-board", "New Board", Some("CmdOrCtrl+Shift+N"))?,
            &item(app, "add-image", "Add Images…", Some("CmdOrCtrl+Shift+I"))?,
            &item(app, "add-file", "Add Files…", None)?,
            &sep()?,
            &item(app, "import-json", "Import Board (JSON)…", None)?,
            &item(app, "export-markdown", "Export Board as Markdown…", None)?,
            &item(app, "export-json", "Export Board as JSON…", None)?,
            &sep()?,
            &item(app, "backup", "Back Up Database…", None)?,
            &item(app, "restore", "Restore from Backup…", None)?,
            &sep()?,
            &PredefinedMenuItem::close_window(app, None)?,
        ],
    )?;

    let edit = Submenu::with_items(
        app,
        "Edit",
        true,
        &[
            &item(app, "undo", "Undo", Some("CmdOrCtrl+Z"))?,
            &item(app, "redo", "Redo", Some("CmdOrCtrl+Shift+Z"))?,
            &sep()?,
            &PredefinedMenuItem::cut(app, None)?,
            &PredefinedMenuItem::copy(app, None)?,
            &PredefinedMenuItem::paste(app, None)?,
            &item(app, "duplicate", "Duplicate", Some("CmdOrCtrl+D"))?,
            &item(app, "delete", "Delete", None)?,
            &PredefinedMenuItem::select_all(app, None)?,
            &sep()?,
            &item(app, "bring-to-front", "Bring to Front", Some("CmdOrCtrl+Shift+]"))?,
            &item(app, "send-to-back", "Send to Back", Some("CmdOrCtrl+Shift+["))?,
        ],
    )?;

    let view = Submenu::with_items(
        app,
        "View",
        true,
        &[
            &item(app, "search", "Search…", Some("CmdOrCtrl+K"))?,
            &item(app, "trash", "Trash", None)?,
            &sep()?,
            &item(app, "zoom-in", "Zoom In", Some("CmdOrCtrl+="))?,
            &item(app, "zoom-out", "Zoom Out", Some("CmdOrCtrl+-"))?,
            &item(app, "zoom-reset", "Actual Size", Some("CmdOrCtrl+0"))?,
            &item(app, "zoom-fit", "Zoom to Fit", Some("CmdOrCtrl+1"))?,
            &sep()?,
            &item(app, "go-home", "Go to Home Board", Some("CmdOrCtrl+Shift+H"))?,
            &item(app, "go-up", "Go to Parent Board", Some("CmdOrCtrl+Up"))?,
            &sep()?,
            &PredefinedMenuItem::fullscreen(app, None)?,
        ],
    )?;

    let window = Submenu::with_items(
        app,
        "Window",
        true,
        &[&PredefinedMenuItem::minimize(app, None)?, &PredefinedMenuItem::maximize(app, None)?],
    )?;

    let help = Submenu::with_items(
        app,
        "Help",
        true,
        &[
            &item(app, "shortcuts", "Keyboard Shortcuts", Some("CmdOrCtrl+/"))?,
            &sep()?,
            &item(app, "open-data-folder", "Show Data Folder", None)?,
            &item(app, "benchmark", "Generate Benchmark Board", None)?,
            &sep()?,
            &item(app, "website", "Milano Pen on GitHub", None)?,
        ],
    )?;

    Menu::with_items(app, &[&app_menu, &file, &edit, &view, &window, &help])
}
