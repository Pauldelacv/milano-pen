use serde::Serialize;
use tauri::{AppHandle, Emitter, State, Webview};
use tauri_plugin_opener::OpenerExt;

use milano_core::models::Patch;
use milano_core::services::benchmark::{self, BenchmarkSpec};
use milano_core::services::settings::Settings;
use milano_core::{Error, Result};

use super::{pick_folder, pick_save};
use crate::state::{is_project_dir, new_project_root, AppState, ProjectInfo, PROJECT_EXT};
use crate::windows;

#[tauri::command]
/// App settings live in the Library and apply to every window.
pub async fn get_settings(state: State<'_, AppState>) -> Result<Settings> {
    state.library.with(|ws| ws.settings()).await
}

#[tauri::command]
pub async fn save_settings(app: AppHandle, state: State<'_, AppState>, settings: Settings) -> Result<Settings> {
    let saved = state.library.with(move |ws| ws.save_settings(&settings)).await?;
    // Other windows pick up the change (theme, snapping, …).
    let _ = app.emit("settings-changed", &saved);
    Ok(saved)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    version: String,
    data_dir: String,
    platform: &'static str,
}

#[tauri::command]
pub async fn app_info(app: AppHandle, webview: Webview, state: State<'_, AppState>) -> Result<AppInfo> {
    Ok(AppInfo {
        version: app.package_info().version.to_string(),
        data_dir: state.project(webview.label())?.paths.root.to_string_lossy().into(),
        platform: std::env::consts::OS,
    })
}

/// Only web and mail links leave the app; anything else is refused.
pub fn is_openable_url(raw: &str) -> bool {
    url::Url::parse(raw).is_ok_and(|u| matches!(u.scheme(), "http" | "https" | "mailto"))
}

#[tauri::command]
pub async fn open_external(app: AppHandle, url: String) -> Result<()> {
    if !is_openable_url(&url) {
        return Err(Error::Invalid("only http, https and mailto links can be opened".into()));
    }
    app.opener().open_url(url, None::<&str>).map_err(|e| Error::Invalid(e.to_string()))
}

#[tauri::command]
pub async fn open_data_folder(app: AppHandle, webview: Webview, state: State<'_, AppState>) -> Result<()> {
    let root = state.project(webview.label())?.paths.root.clone();
    app.opener().reveal_item_in_dir(root).map_err(|e| Error::Invalid(e.to_string()))
}

#[tauri::command]
pub async fn generate_benchmark(webview: Webview, state: State<'_, AppState>, board_id: String) -> Result<Patch> {
    let project = state.project(webview.label())?;
    let spec = BenchmarkSpec::default();
    // Encode the images first, without holding the workspace.
    let images = tauri::async_runtime::spawn_blocking(move || benchmark::synthetic_images(&spec))
        .await
        .map_err(|e| Error::Invalid(e.to_string()))?;
    let (patch, assets) = project.with(move |ws| benchmark::generate(ws, &board_id, spec, &images)).await?;
    project.prefetch_thumbnails(assets);
    Ok(patch)
}

/// The webview has flushed its pending saves: quitting may proceed once
/// every window has.
#[tauri::command]
pub async fn ready_to_quit(app: AppHandle, webview: Webview) -> Result<()> {
    windows::quit_flushed(&app, webview.label());
    Ok(())
}

/// The webview has flushed its pending saves: its window may close.
#[tauri::command]
pub async fn ready_to_close(app: AppHandle, webview: Webview) -> Result<()> {
    windows::allow_close(&app, webview.label());
    Ok(())
}

/// Which project this window shows.
#[tauri::command]
pub async fn project_info(webview: Webview, state: State<'_, AppState>) -> Result<ProjectInfo> {
    Ok(state.project(webview.label())?.info())
}

/// File ▸ New Project: pick a name and place, create the folder, open it.
#[tauri::command]
pub async fn new_project(app: AppHandle) -> Result<bool> {
    let Some(picked) = pick_save(&app, "New project", "Untitled", ("Milano Pen project", &[PROJECT_EXT])).await else {
        return Ok(false);
    };
    let root = new_project_root(&picked);
    if root.exists() && !is_project_dir(&root) {
        let non_empty = std::fs::read_dir(&root).map(|mut d| d.next().is_some()).unwrap_or(true);
        if non_empty {
            return Err(Error::Invalid(format!("{} already exists and isn't a Milano Pen project", root.display())));
        }
    }
    let handle = app.clone();
    tauri::async_runtime::spawn_blocking(move || windows::open_project_at(&handle, &root))
        .await
        .map_err(|e| Error::Invalid(e.to_string()))??;
    Ok(true)
}

/// File ▸ Open Project: pick a project folder and open it in a new window
/// (or bring forward the window already showing it).
#[tauri::command]
pub async fn open_project(app: AppHandle) -> Result<bool> {
    let Some(root) = pick_folder(&app, "Open project").await else { return Ok(false) };
    if !is_project_dir(&root) {
        return Err(Error::Invalid(format!(
            "{} isn't a Milano Pen project (choose a folder created with File ▸ New Project)",
            root.display()
        )));
    }
    let handle = app.clone();
    tauri::async_runtime::spawn_blocking(move || windows::open_project_at(&handle, &root))
        .await
        .map_err(|e| Error::Invalid(e.to_string()))??;
    Ok(true)
}

/// File ▸ Open Library: the default workspace, in a window again.
#[tauri::command]
pub async fn open_library(app: AppHandle, state: State<'_, AppState>) -> Result<()> {
    windows::show_project(&app, state.library.clone(), "library")
}

/// File ▸ Print: the macOS print dialog for the current view (it also saves
/// as PDF). The webview applies its print stylesheet: no UI, no grid dots.
#[tauri::command]
pub async fn print_board(webview: Webview) -> Result<()> {
    webview.print().map_err(|e| Error::Invalid(format!("printing isn't available: {e}")))
}

#[cfg(test)]
mod tests {
    #[test]
    fn only_web_links_open() {
        assert!(super::is_openable_url("https://example.com"));
        assert!(super::is_openable_url("mailto:a@b.c"));
        assert!(!super::is_openable_url("file:///etc/passwd"));
        assert!(!super::is_openable_url("javascript:alert(1)"));
        assert!(!super::is_openable_url("not a url"));
    }
}
