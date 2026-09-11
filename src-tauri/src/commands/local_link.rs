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
// Generate a local HTTP link (LAN) for a specific file
// The link can be copied and shared on the local network.
// Requires the HTTP server to be running.

use tauri::State;
use crate::AppState;
use crate::utils::network::get_local_ip;

/// Generate a local HTTP link for a specific file.
/// The link can be opened by other devices on the same LAN to download the file.
/// Returns an error if the HTTP server is not active or file not found.
#[tauri::command]
pub async fn generate_local_link(state: State<'_, AppState>, hash: String) -> Result<String, String> {
    // Verifica che il server HTTP sia attivo
    let running = *state.server_running.lock().await;
    if !running {
        return Err(
            "The HTTP server is not active. Start the server to generate a local link.".to_string(),
        );
    }

    // Verify the file exists in the index
    {
        let file_index = state.file_index.lock().await;
        if !file_index.contains_key(&hash) {
            return Err("File not found. Select a file from the list.".to_string());
        }
    }

    let info = get_local_ip().map_err(|e| e.to_string())?;
    Ok(format!("http://{}:{}/download/{}", info.ip, info.port, hash))
}
