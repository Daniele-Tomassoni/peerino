// List active peers on the network
// Uses DHT for discovery

use tauri::State;
use crate::AppState;
use crate::p2p::PeerInfo;

/// List active peers on the network
#[tauri::command]
pub async fn list_peers(
    _state: State<'_, AppState>,
) -> Result<Vec<PeerInfo>, String> {
    // In Phase 4, the actual peer discovery is handled by the frontend
    // using PeerJS with the signaling server. This command is a placeholder.
    
    // Return empty list for now - frontend will use PeerJS for real discovery
    Ok(vec![])
}