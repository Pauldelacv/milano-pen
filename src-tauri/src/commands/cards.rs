use tauri::{State, Webview};

use milano_core::models::{CardUpdate, HistoryState, NewCard, Patch, SearchResult, TrashEntry};
use milano_core::services::trash::EmptyReport;
use milano_core::{Error, Result};

use std::sync::Arc;

use crate::state::{AppState, Clip};

#[tauri::command]
pub async fn create_cards(webview: Webview, state: State<'_, AppState>, cards: Vec<NewCard>) -> Result<Patch> {
    state.project(webview.label())?.with(move |ws| ws.create_cards(&cards)).await
}

#[tauri::command]
pub async fn update_cards(
    webview: Webview,
    state: State<'_, AppState>,
    updates: Vec<CardUpdate>,
    label: Option<String>,
    coalesce: Option<String>,
) -> Result<Patch> {
    state
        .project(webview.label())?
        .with(move |ws| ws.update_cards(&updates, label.as_deref().unwrap_or("Edit"), coalesce))
        .await
}

#[tauri::command]
pub async fn restack_cards(
    webview: Webview,
    state: State<'_, AppState>,
    ids: Vec<String>,
    to_front: bool,
) -> Result<Patch> {
    state.project(webview.label())?.with(move |ws| ws.restack(&ids, to_front)).await
}

#[tauri::command]
pub async fn trash_cards(webview: Webview, state: State<'_, AppState>, ids: Vec<String>) -> Result<Patch> {
    state.project(webview.label())?.with(move |ws| ws.trash_cards(&ids)).await
}

#[tauri::command]
pub async fn duplicate_cards(webview: Webview, state: State<'_, AppState>, ids: Vec<String>) -> Result<Patch> {
    state.project(webview.label())?.with(move |ws| ws.duplicate(&ids)).await
}

/// Copy cards to the app-wide clipboard (shared by every open project).
#[tauri::command]
pub async fn copy_cards(webview: Webview, state: State<'_, AppState>, ids: Vec<String>) -> Result<usize> {
    let source = state.project(webview.label())?;
    let snapshot = source.with(move |ws| ws.snapshot(&ids)).await?;
    let n = snapshot.roots.len();
    *state.clipboard.lock().map_err(|_| Error::Conflict("busy".into()))? = Some(Clip { snapshot, source });
    Ok(n)
}

/// Paste the clipboard. Cards copied from another project bring their
/// files with them (copied into this project's asset store).
#[tauri::command]
pub async fn paste_cards(
    webview: Webview,
    state: State<'_, AppState>,
    board_id: String,
    at: Option<(f64, f64)>,
) -> Result<Patch> {
    let target = state.project(webview.label())?;
    let clip = {
        let guard = state.clipboard.lock().map_err(|_| Error::Conflict("busy".into()))?;
        match guard.as_ref() {
            Some(c) => (c.snapshot.clone(), c.source.clone()),
            None => return Ok(Patch::default()),
        }
    };
    let (snapshot, source) = clip;
    let files = if Arc::ptr_eq(&source, &target) {
        vec![]
    } else {
        let snap = snapshot.clone();
        source.with(move |ws| ws.snapshot_assets(&snap)).await?
    };
    let (patch, added) = target
        .with(move |ws| {
            let added = ws.adopt_assets(&files)?;
            Ok((ws.paste_snapshot(&board_id, &snapshot, at)?, added))
        })
        .await?;
    target.prefetch_thumbnails(added.into_iter().filter(|a| a.is_image()).map(|a| a.id).collect());
    Ok(patch)
}

#[tauri::command]
pub async fn undo(webview: Webview, state: State<'_, AppState>) -> Result<Option<Patch>> {
    state.project(webview.label())?.with(|ws| ws.undo()).await
}

#[tauri::command]
pub async fn redo(webview: Webview, state: State<'_, AppState>) -> Result<Option<Patch>> {
    state.project(webview.label())?.with(|ws| ws.redo()).await
}

#[tauri::command]
pub async fn history_state(webview: Webview, state: State<'_, AppState>) -> Result<HistoryState> {
    state.project(webview.label())?.with(|ws| Ok(ws.history_state())).await
}

#[tauri::command]
pub async fn search(webview: Webview, state: State<'_, AppState>, query: String) -> Result<Vec<SearchResult>> {
    state.project(webview.label())?.with(move |ws| ws.search(&query)).await
}

#[tauri::command]
pub async fn list_trash(webview: Webview, state: State<'_, AppState>) -> Result<Vec<TrashEntry>> {
    state.project(webview.label())?.with(|ws| ws.trash_list()).await
}

#[tauri::command]
pub async fn restore_trashed(webview: Webview, state: State<'_, AppState>, id: String) -> Result<Patch> {
    state.project(webview.label())?.with(move |ws| ws.restore_trashed(&id)).await
}

#[tauri::command]
pub async fn empty_trash(webview: Webview, state: State<'_, AppState>) -> Result<EmptyReport> {
    let report = state.project(webview.label())?.with(|ws| ws.empty_trash()).await?;
    Ok(report)
}
