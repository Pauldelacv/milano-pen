//! The Tauri shell of Milano Pen: windows, menus, IPC commands and the asset
//! protocol. All data logic lives in `milano-core`.

mod commands;
mod menu;
mod protocol;
mod state;
mod windows;

use std::path::PathBuf;
use std::sync::atomic::Ordering;

use tauri::{Emitter, EventTarget, Manager, RunEvent, WindowEvent};

use crate::state::{AppState, Project};

/// Folder name under Application Support.
const DATA_FOLDER: &str = "Milano Pen";

fn data_dir(app: &tauri::App) -> tauri::Result<PathBuf> {
    // MILANO_PEN_DATA_DIR keeps development and test data away from real data.
    if let Some(dir) = std::env::var_os("MILANO_PEN_DATA_DIR") {
        return Ok(PathBuf::from(dir));
    }
    Ok(app.path().data_dir()?.join(DATA_FOLDER))
}

/// The window menu commands go to: the focused one, else any.
fn menu_target<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> Option<String> {
    let windows = app.webview_windows();
    windows
        .iter()
        .find(|(_, w)| w.is_focused().unwrap_or(false))
        .or_else(|| windows.iter().next())
        .map(|(label, _)| label.clone())
}

pub fn run() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .register_asynchronous_uri_scheme_protocol(protocol::SCHEME, protocol::handle)
        .setup(|app| {
            let root = data_dir(app)?;
            log::info!("library: {}", root.display());
            let library = Project::open(&root, "Library".into(), true)
                .map_err(|e| format!("could not open the database: {e}"))?;
            let auto_backup =
                library.workspace.lock().ok().and_then(|ws| ws.settings().ok()).is_none_or(|s| s.auto_backup);
            let paths = library.paths.clone();
            app.manage(AppState::new(library));

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
                if let Some(label) = menu_target(app) {
                    let _ = app.emit_to(EventTarget::webview_window(label), "menu", event.id().as_ref());
                }
            });
            Ok(())
        })
        .on_window_event(|window, event| match event {
            // Each window flushes its own pending saves before closing.
            WindowEvent::CloseRequested { api, .. } => {
                let state = window.state::<AppState>();
                let allowed = state.ready_to_quit.load(Ordering::SeqCst)
                    || state.closing.lock().map(|c| c.contains(window.label())).unwrap_or(true);
                if !allowed {
                    api.prevent_close();
                    windows::request_close(window.app_handle(), window.label());
                }
            }
            WindowEvent::Destroyed => window.state::<AppState>().detach(window.label()),
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
            commands::system::ready_to_close,
            commands::system::project_info,
            commands::system::new_project,
            commands::system::open_project,
            commands::system::open_library,
            commands::system::print_board,
        ])
        .build(tauri::generate_context!())
        .expect("error while building Milano Pen");

    app.run(|handle, event| {
        if let RunEvent::ExitRequested { api, .. } = event {
            let ready = handle.try_state::<AppState>().is_none_or(|s| s.ready_to_quit.load(Ordering::SeqCst));
            if !ready {
                api.prevent_exit();
                windows::request_quit(handle);
            }
        }
    });
}
