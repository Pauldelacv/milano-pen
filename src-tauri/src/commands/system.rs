use std::sync::atomic::Ordering;

use serde::Serialize;
use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;

use milano_core::models::Patch;
use milano_core::services::benchmark::{self, BenchmarkSpec};
use milano_core::services::settings::Settings;
use milano_core::{Error, Result};

use crate::state::AppState;

#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> Result<Settings> {
    state.with(|ws| ws.settings()).await
}

#[tauri::command]
pub async fn save_settings(state: State<'_, AppState>, settings: Settings) -> Result<Settings> {
    state.with(move |ws| ws.save_settings(&settings)).await
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    version: String,
    data_dir: String,
    platform: &'static str,
}

#[tauri::command]
pub async fn app_info(app: AppHandle, state: State<'_, AppState>) -> Result<AppInfo> {
    Ok(AppInfo {
        version: app.package_info().version.to_string(),
        data_dir: state.paths.root.to_string_lossy().into(),
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
pub async fn open_data_folder(app: AppHandle, state: State<'_, AppState>) -> Result<()> {
    app.opener().open_path(state.paths.root.to_string_lossy(), None::<&str>).map_err(|e| Error::Invalid(e.to_string()))
}

#[tauri::command]
pub async fn generate_benchmark(state: State<'_, AppState>, board_id: String) -> Result<Patch> {
    let spec = BenchmarkSpec::default();
    // Encode the images first, without holding the workspace.
    let images = tauri::async_runtime::spawn_blocking(move || benchmark::synthetic_images(&spec))
        .await
        .map_err(|e| Error::Invalid(e.to_string()))?;
    let (patch, assets) = state.with(move |ws| benchmark::generate(ws, &board_id, spec, &images)).await?;
    state.prefetch_thumbnails(assets);
    Ok(patch)
}

/// The webview has flushed its pending saves: quitting may proceed.
#[tauri::command]
pub async fn ready_to_quit(app: AppHandle, state: State<'_, AppState>) -> Result<()> {
    state.ready_to_quit.store(true, Ordering::SeqCst);
    app.exit(0);
    Ok(())
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
