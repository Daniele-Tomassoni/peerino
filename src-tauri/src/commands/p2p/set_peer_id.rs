// Store the current PeerID in AppState
// Called by the frontend when PeerJS establishes a connection

use tauri::State;
use crate::AppState;

/// Update the current PeerID in the application state
/// This is called from the frontend when PeerJS connects
#[tauri::command]
pub async fn set_peer_id(
    state: State<'_, AppState>,
    peer_id: String,
) -> Result<(), String> {
    if peer_id.is_empty() {
        return Err("PeerID cannot be empty".to_string());
    }

    let mut current = state.peer_id.lock().await;
    *current = Some(peer_id.clone());

    log::info!("🆔 PeerID updated in state: {}", peer_id);

    Ok(())
}
