use tauri::State;

use milano_core::models::{CardUpdate, HistoryState, NewCard, Patch, SearchResult, TrashEntry};
use milano_core::services::trash::EmptyReport;
use milano_core::Result;

use crate::state::AppState;

#[tauri::command]
pub async fn create_cards(state: State<'_, AppState>, cards: Vec<NewCard>) -> Result<Patch> {
    state.with(move |ws| ws.create_cards(&cards)).await
}

#[tauri::command]
pub async fn update_cards(
    state: State<'_, AppState>,
    updates: Vec<CardUpdate>,
    label: Option<String>,
    coalesce: Option<String>,
) -> Result<Patch> {
    state.with(move |ws| ws.update_cards(&updates, label.as_deref().unwrap_or("Edit"), coalesce)).await
}

#[tauri::command]
pub async fn restack_cards(state: State<'_, AppState>, ids: Vec<String>, to_front: bool) -> Result<Patch> {
    state.with(move |ws| ws.restack(&ids, to_front)).await
}

#[tauri::command]
pub async fn trash_cards(state: State<'_, AppState>, ids: Vec<String>) -> Result<Patch> {
    state.with(move |ws| ws.trash_cards(&ids)).await
}

#[tauri::command]
pub async fn duplicate_cards(state: State<'_, AppState>, ids: Vec<String>) -> Result<Patch> {
    state.with(move |ws| ws.duplicate(&ids)).await
}

#[tauri::command]
pub async fn copy_cards(state: State<'_, AppState>, ids: Vec<String>) -> Result<usize> {
    state.with(move |ws| ws.copy(&ids)).await
}

#[tauri::command]
pub async fn paste_cards(state: State<'_, AppState>, board_id: String, at: Option<(f64, f64)>) -> Result<Patch> {
    state.with(move |ws| ws.paste(&board_id, at)).await
}

#[tauri::command]
pub async fn undo(state: State<'_, AppState>) -> Result<Option<Patch>> {
    state.with(|ws| ws.undo()).await
}

#[tauri::command]
pub async fn redo(state: State<'_, AppState>) -> Result<Option<Patch>> {
    state.with(|ws| ws.redo()).await
}

#[tauri::command]
pub async fn history_state(state: State<'_, AppState>) -> Result<HistoryState> {
    state.with(|ws| Ok(ws.history_state())).await
}

#[tauri::command]
pub async fn search(state: State<'_, AppState>, query: String) -> Result<Vec<SearchResult>> {
    state.with(move |ws| ws.search(&query)).await
}

#[tauri::command]
pub async fn list_trash(state: State<'_, AppState>) -> Result<Vec<TrashEntry>> {
    state.with(|ws| ws.trash_list()).await
}

#[tauri::command]
pub async fn restore_trashed(state: State<'_, AppState>, id: String) -> Result<Patch> {
    state.with(move |ws| ws.restore_trashed(&id)).await
}

#[tauri::command]
pub async fn empty_trash(state: State<'_, AppState>) -> Result<EmptyReport> {
    let report = state.with(|ws| ws.empty_trash()).await?;
    Ok(report)
}
