use tauri::State;

use milano_core::models::{BoardUpdate, BoardView, Patch};
use milano_core::Result;

use crate::state::AppState;

#[tauri::command]
pub async fn get_root_board(state: State<'_, AppState>) -> Result<String> {
    state.with(|ws| ws.root_board_id()).await
}

#[tauri::command]
pub async fn open_board(state: State<'_, AppState>, id: String) -> Result<BoardView> {
    state.with(move |ws| ws.open_board(&id)).await
}

#[tauri::command]
pub async fn update_board(state: State<'_, AppState>, id: String, update: BoardUpdate) -> Result<Patch> {
    state.with(move |ws| ws.update_board(&id, &update)).await
}
