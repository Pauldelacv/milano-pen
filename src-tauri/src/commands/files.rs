//! Exports, imports and backups through native dialogs.

use tauri::{AppHandle, Emitter, EventTarget, State, Webview};

use milano_core::models::Patch;
use milano_core::{Error, Result};

use super::{pick_files, pick_save, safe_file_name};
use crate::state::{AppState, Project};

async fn board_title(project: &Project, board_id: &str) -> Result<String> {
    let id = board_id.to_string();
    project.with(move |ws| Ok(ws.open_board(&id)?.board.title)).await
}

/// Returns the written path, or None if the user cancelled.
#[tauri::command]
pub async fn export_board_markdown(
    app: AppHandle,
    webview: Webview,
    state: State<'_, AppState>,
    board_id: String,
) -> Result<Option<String>> {
    let project = state.project(webview.label())?;
    let name = format!("{}.md", safe_file_name(&board_title(&project, &board_id).await?, "board"));
    let Some(dest) = pick_save(&app, "Export board as Markdown", &name, ("Markdown", &["md"])).await else {
        return Ok(None);
    };
    let md = project.with(move |ws| ws.export_markdown(&board_id)).await?;
    tokio::fs::write(&dest, md).await?;
    Ok(Some(dest.to_string_lossy().into()))
}

#[tauri::command]
pub async fn export_board_json(
    app: AppHandle,
    webview: Webview,
    state: State<'_, AppState>,
    board_id: String,
) -> Result<Option<String>> {
    let project = state.project(webview.label())?;
    let name = format!("{}.milanopen.json", safe_file_name(&board_title(&project, &board_id).await?, "board"));
    let Some(dest) = pick_save(&app, "Export board as JSON", &name, ("JSON", &["json"])).await else {
        return Ok(None);
    };
    let json = project.with(move |ws| ws.export_json(&board_id)).await?;
    tokio::fs::write(&dest, json).await?;
    Ok(Some(dest.to_string_lossy().into()))
}

#[tauri::command]
pub async fn import_board_json(
    app: AppHandle,
    webview: Webview,
    state: State<'_, AppState>,
    board_id: String,
    at: (f64, f64),
) -> Result<Option<Patch>> {
    let Some(src) = pick_files(&app, "Import board", &[("JSON", &["json"])], false).await.pop() else {
        return Ok(None);
    };
    let meta = tokio::fs::metadata(&src).await?;
    if meta.len() > 200 * 1024 * 1024 {
        return Err(Error::Invalid("that file is too large to be a board export".into()));
    }
    let json = tokio::fs::read_to_string(&src).await?;
    state.project(webview.label())?.with(move |ws| ws.import_json(&board_id, &json, at)).await.map(Some)
}

#[tauri::command]
pub async fn backup_database(app: AppHandle, webview: Webview, state: State<'_, AppState>) -> Result<Option<String>> {
    let project = state.project(webview.label())?;
    let name = format!("{} backup {}.sqlite", safe_file_name(&project.name, "Milano Pen"), chrono_like_today());
    let Some(dest) = pick_save(&app, "Back up database", &name, ("SQLite database", &["sqlite"])).await else {
        return Ok(None);
    };
    let d = dest.clone();
    project.with(move |ws| ws.backup_to(&d)).await?;
    Ok(Some(dest.to_string_lossy().into()))
}

/// Replace the database with a backup, then tell the webview to reload.
#[tauri::command]
pub async fn restore_database(app: AppHandle, webview: Webview, state: State<'_, AppState>) -> Result<bool> {
    let project = state.project(webview.label())?;
    let Some(src) =
        pick_files(&app, "Restore from backup", &[("SQLite database", &["sqlite", "db"])], false).await.pop()
    else {
        return Ok(false);
    };
    project.with(move |ws| ws.restore_backup(&src)).await?;
    let _ = app.emit_to(EventTarget::webview_window(webview.label()), "workspace-replaced", ());
    Ok(true)
}

fn chrono_like_today() -> String {
    // YYYY-MM-DD from the system clock without another dependency.
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let days = (secs / 86_400) as i64;
    let (y, m, d) = civil_from_days(days);
    format!("{y:04}-{m:02}-{d:02}")
}

/// Howard Hinnant's days → civil date algorithm.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

#[cfg(test)]
mod tests {
    #[test]
    fn civil_dates() {
        assert_eq!(super::civil_from_days(0), (1970, 1, 1));
        assert_eq!(super::civil_from_days(20_000), (2024, 10, 4));
    }
}
