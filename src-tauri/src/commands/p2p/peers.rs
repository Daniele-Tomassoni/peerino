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