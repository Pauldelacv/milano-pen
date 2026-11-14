use tauri::{State, Webview};

use milano_core::models::{BoardUpdate, BoardView, Patch};
use milano_core::Result;

use crate::state::AppState;

#[tauri::command]
pub async fn get_root_board(webview: Webview, state: State<'_, AppState>) -> Result<String> {
    state.project(webview.label())?.with(|ws| ws.root_board_id()).await
}

#[tauri::command]
pub async fn open_board(webview: Webview, state: State<'_, AppState>, id: String) -> Result<BoardView> {
    state.project(webview.label())?.with(move |ws| ws.open_board(&id)).await
}

#[tauri::command]
pub async fn update_board(
    webview: Webview,
    state: State<'_, AppState>,
    id: String,
    update: BoardUpdate,
) -> Result<Patch> {
    state.project(webview.label())?.with(move |ws| ws.update_board(&id, &update)).await
}
