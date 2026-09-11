// Peerino - P2P file sharing without cloud, without accounts, without intermediaries.
// Copyright (C) 2026 Daniele Tomassoni
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.
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