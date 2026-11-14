//! The Tauri shell of Milano Pen: window, menus, IPC commands and the asset
//! protocol. All data logic lives in `milano-core`.

mod commands;
mod menu;
mod protocol;
mod state;

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::Serialize;
use tauri::{DragDropEvent, Emitter, Manager, RunEvent, WindowEvent};

use milano_core::images::Thumbnailer;
use milano_core::{AppPaths, Workspace};

use crate::state::AppState;

/// Folder name under Application Support.
const DATA_FOLDER: &str = "Milano Pen";
/// Hot thumbnails kept in memory by the asset protocol.
const THUMB_CACHE_BYTES: usize = 64 * 1024 * 1024;
/// How long quitting waits for the webview to flush its pending saves.
const QUIT_FLUSH_TIMEOUT: Duration = Duration::from_secs(3);

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct DropPayload {
    paths: Vec<PathBuf>,
    x: f64,
    y: f64,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct DragPayload {
    active: bool,
}

fn data_dir(app: &tauri::App) -> tauri::Result<PathBuf> {
    // MILANO_PEN_DATA_DIR keeps development and test data away from real data.
    if let Some(dir) = std::env::var_os("MILANO_PEN_DATA_DIR") {
        return Ok(PathBuf::from(dir));
    }
    Ok(app.path().data_dir()?.join(DATA_FOLDER))
}

/// Ask the webview to flush pending saves, then quit (or quit anyway after
/// a timeout, so a hung webview can never block quitting).
fn request_quit(app: &tauri::AppHandle) {
    let _ = app.emit("app-will-quit", ());
    let handle = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(QUIT_FLUSH_TIMEOUT);
        handle.state::<AppState>().ready_to_quit.store(true, Ordering::SeqCst);
        handle.exit(0);
    });
}

pub fn run() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .register_asynchronous_uri_scheme_protocol(protocol::SCHEME, protocol::handle)
        .setup(|app| {
            let paths = AppPaths::new(data_dir(app)?);
            log::info!("data directory: {}", paths.root.display());
            let workspace = Workspace::open(&paths).map_err(|e| format!("could not open the database: {e}"))?;
            let store = workspace.asset_store().cloned().expect("an on-disk workspace has an asset store");
            let thumbnails = Thumbnailer::new(store, Thumbnailer::default_workers(), THUMB_CACHE_BYTES);
            let auto_backup = workspace.settings().map(|s| s.auto_backup).unwrap_or(true);
            app.manage(AppState {
                workspace: Arc::new(Mutex::new(workspace)),
                thumbnails: Arc::new(thumbnails),
                paths: paths.clone(),
                granted_paths: Mutex::new(HashSet::new()),
                ready_to_quit: AtomicBool::new(false),
            });

            // Daily backup on its own connection, off the main thread.
            if auto_backup {
                std::thread::spawn(move || {
                    match milano_core::services::backup::auto_backup_file(&paths.database_file(), &paths.backups) {
                        Ok(Some(f)) => log::info!("automatic backup: {}", f.display()),
                        Ok(None) => {}
                        Err(e) => log::warn!("automatic backup failed: {e}"),
                    }
                });
            }

            app.set_menu(menu::build(app.handle())?)?;
            app.on_menu_event(|app, event| {
                let _ = app.emit("menu", event.id().as_ref());
            });
            Ok(())
        })
        .on_window_event(|window, event| match event {
            WindowEvent::DragDrop(DragDropEvent::Drop { paths, position }) => {
                let state = window.state::<AppState>();
                if let Ok(mut granted) = state.granted_paths.lock() {
                    granted.extend(paths.iter().cloned());
                }
                let scale = window.scale_factor().unwrap_or(1.0);
                let pos = position.to_logical::<f64>(scale);
                let _ = window.emit("files-dropped", DropPayload { paths: paths.clone(), x: pos.x, y: pos.y });
                let _ = window.emit("files-drag", DragPayload { active: false });
            }
            WindowEvent::DragDrop(DragDropEvent::Enter { .. } | DragDropEvent::Over { .. }) => {
                let _ = window.emit("files-drag", DragPayload { active: true });
            }
            WindowEvent::DragDrop(DragDropEvent::Leave) => {
                let _ = window.emit("files-drag", DragPayload { active: false });
            }
            // Single-window app: closing the window quits, after a flush.
            WindowEvent::CloseRequested { api, .. }
                if !window.state::<AppState>().ready_to_quit.load(Ordering::SeqCst) =>
            {
                api.prevent_close();
                request_quit(window.app_handle());
            }
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            commands::boards::get_root_board,
            commands::boards::open_board,
            commands::boards::update_board,
            commands::cards::create_cards,
            commands::cards::update_cards,
            commands::cards::restack_cards,
            commands::cards::trash_cards,
            commands::cards::duplicate_cards,
            commands::cards::copy_cards,
            commands::cards::paste_cards,
            commands::cards::undo,
            commands::cards::redo,
            commands::cards::history_state,
            commands::cards::search,
            commands::cards::list_trash,
            commands::cards::restore_trashed,
            commands::cards::empty_trash,
            commands::assets::pick_and_import_files,
            commands::assets::import_dropped_files,
            commands::assets::import_clipboard_file,
            commands::assets::open_asset,
            commands::assets::reveal_asset,
            commands::files::export_board_markdown,
            commands::files::export_board_json,
            commands::files::import_board_json,
            commands::files::backup_database,
            commands::files::restore_database,
            commands::system::get_settings,
            commands::system::save_settings,
            commands::system::app_info,
            commands::system::open_external,
            commands::system::open_data_folder,
            commands::system::generate_benchmark,
            commands::system::ready_to_quit,
        ])
        .build(tauri::generate_context!())
        .expect("error while building Milano Pen");

    app.run(|handle, event| {
        if let RunEvent::ExitRequested { api, .. } = event {
            let ready = handle.try_state::<AppState>().is_none_or(|s| s.ready_to_quit.load(Ordering::SeqCst));
            if !ready {
                api.prevent_exit();
                request_quit(handle);
            }
        }
    });
}
