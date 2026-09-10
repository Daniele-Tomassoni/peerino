// Peerino - P2P file sharing senza cloud, senza account, senza intermediari.
// Copyright (C) 2025 Daniele
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
