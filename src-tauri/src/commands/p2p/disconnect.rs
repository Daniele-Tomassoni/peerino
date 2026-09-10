// Disconnect from a remote peer
// Closes the WebRTC connection

use tauri::State;
use crate::AppState;

/// Disconnect from a remote peer
#[tauri::command]
pub async fn disconnect_from_peer(
    _state: State<'_, AppState>,
    peer_id: String,
) -> Result<(), String> {
    // In Phase 4, the actual WebRTC disconnection is handled by the frontend
    // using PeerJS. This command is a placeholder for future Rust-based P2P.
    
    log::info!("🔌 Disconnecting from peer: {}", peer_id);
    
    Ok(())
}