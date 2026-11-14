//! Importing files as cards. Paths are accepted only if the user picked them
//! in a native dialog (here) or dropped them from Finder (recorded by the
//! window event handler in `lib.rs`), never from arbitrary frontend input.

use std::path::PathBuf;

use tauri::ipc::{InvokeBody, Request};
use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;

use milano_core::services::workspace::ImportResult;
use milano_core::{Error, Result};

use super::pick_files;
use crate::state::AppState;

const IMAGE_EXTS: &[&str] =
    &["png", "jpg", "jpeg", "gif", "webp", "heic", "heif", "avif", "bmp", "tif", "tiff", "svg", "ico"];

async fn import(state: &AppState, board_id: String, paths: Vec<PathBuf>, at: (f64, f64)) -> Result<ImportResult> {
    let result = state.with(move |ws| ws.import_files(&board_id, &paths, at)).await?;
    state.prefetch_thumbnails(result.assets.iter().filter(|a| a.is_image()).map(|a| a.id.clone()).collect());
    Ok(result)
}

#[tauri::command]
pub async fn pick_and_import_files(
    app: AppHandle,
    state: State<'_, AppState>,
    board_id: String,
    at: (f64, f64),
    images_only: bool,
) -> Result<Option<ImportResult>> {
    let filters: Vec<(&str, &[&str])> = if images_only { vec![("Images", IMAGE_EXTS)] } else { vec![] };
    let title = if images_only { "Add images" } else { "Add files" };
    let paths = pick_files(&app, title, &filters, true).await;
    if paths.is_empty() {
        return Ok(None);
    }
    import(&state, board_id, paths, at).await.map(Some)
}

#[tauri::command]
pub async fn import_dropped_files(
    state: State<'_, AppState>,
    board_id: String,
    paths: Vec<PathBuf>,
    at: (f64, f64),
) -> Result<ImportResult> {
    let allowed: Vec<PathBuf> = {
        let mut granted = state.granted_paths.lock().map_err(|_| Error::Conflict("busy".into()))?;
        paths.into_iter().filter(|p| granted.remove(p)).collect()
    };
    if allowed.is_empty() {
        return Err(Error::Invalid("those files were not dropped on the window".into()));
    }
    import(&state, board_id, allowed, at).await
}

/// Raw bytes from the clipboard (pasted screenshots, copied images). The body
/// is the file content; the name and position travel in headers.
#[tauri::command]
pub async fn import_clipboard_file(state: State<'_, AppState>, request: Request<'_>) -> Result<ImportResult> {
    let InvokeBody::Raw(bytes) = request.body() else {
        return Err(Error::Invalid("expected raw file bytes".into()));
    };
    let header = |k: &str| request.headers().get(k).and_then(|v| v.to_str().ok()).map(String::from);
    let board_id = header("x-board-id").ok_or_else(|| Error::Invalid("missing board".into()))?;
    let name = header("x-file-name").map(|n| percent_decode(&n)).unwrap_or_else(|| "pasted.png".into());
    let x = header("x-at-x").and_then(|v| v.parse().ok()).unwrap_or(0.0);
    let y = header("x-at-y").and_then(|v| v.parse().ok()).unwrap_or(0.0);
    let bytes = bytes.clone();
    let result = state.with(move |ws| ws.import_bytes(&board_id, &name, &bytes, (x, y))).await?;
    state.prefetch_thumbnails(result.assets.iter().filter(|a| a.is_image()).map(|a| a.id.clone()).collect());
    Ok(result)
}

fn percent_decode(s: &str) -> String {
    url::form_urlencoded::parse(format!("n={s}").as_bytes())
        .next()
        .map(|(_, v)| v.into_owned())
        .unwrap_or_else(|| s.to_string())
}

/// Open an imported file with its default macOS application.
#[tauri::command]
pub async fn open_asset(app: AppHandle, state: State<'_, AppState>, asset_id: String) -> Result<()> {
    let path = state.thumbnails.original_path(&asset_id).ok_or_else(|| Error::NotFound("file".into()))?;
    app.opener().open_path(path.to_string_lossy(), None::<&str>).map_err(|e| Error::Invalid(e.to_string()))
}

/// Show an imported file in Finder.
#[tauri::command]
pub async fn reveal_asset(app: AppHandle, state: State<'_, AppState>, asset_id: String) -> Result<()> {
    let path = state.thumbnails.original_path(&asset_id).ok_or_else(|| Error::NotFound("file".into()))?;
    app.opener().reveal_item_in_dir(path).map_err(|e| Error::Invalid(e.to_string()))
}
