// Connect to a remote peer via WebRTC
// Uses signaling server for SDP exchange

use tauri::State;
use crate::AppState;

/// Connect to a remote peer
/// Returns the connection status
#[tauri::command]
pub async fn connect_to_peer(
    _state: State<'_, AppState>,
    peer_id: String,
) -> Result<String, String> {
    // In Phase 4, the actual WebRTC connection is handled by the frontend
    // using PeerJS. This command is a placeholder for future Rust-based P2P.
    
    // Validate peer_id format
    if peer_id.is_empty() {
        return Err("Peer ID cannot be empty".to_string());
    }
    
    // For now, return success - the frontend handles the actual connection
    // via PeerJS with the signaling server
    log::info!("🔗 Connecting to peer: {}", peer_id);
    
    Ok(format!("Connecting to peer {}", peer_id))
}