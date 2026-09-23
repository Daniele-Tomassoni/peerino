use tauri::State;
use crate::AppState;

#[tauri::command]
pub async fn get_startup_error(
    state: State<'_, AppState>,
) -> Result<Option<String>, String> {
    Ok(state.startup_error.lock().await.clone())
}
