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
// Generate public link for P2P-to-Web file sharing
// Creates a link that can be used by browsers to download files

use tauri::State;
use crate::AppState;
use crate::utils::network::get_local_ip;

/// Generate a public link for a file
/// The link can be used by browsers to download the file via relay
#[tauri::command]
pub async fn generate_public_link(
    state: State<'_, AppState>,
    hash: String,
    expires_in: Option<u64>,
    max_downloads: Option<u32>,
) -> Result<String, String> {
    // Validate hash
    if hash.is_empty() {
        return Err("File hash cannot be empty".to_string());
    }
    
    // Check if file exists in index
    {
        let file_index = state.file_index.lock().await;
        if !file_index.contains_key(&hash) {
            return Err("File not found in index".to_string());
        }
    }
    
    // Use the shared relay manager from AppState
    let link_id = state.relay_manager.lock().await
        .generate_public_link(hash, expires_in, max_downloads).await
        .map_err(|e| e.to_string())?;
    
    // Get local IP for the URL, or use environment variable for production
    let base_url = std::env::var("P2P_SHARE_URL")
        .unwrap_or_else(|_| {
            // Use local IP for development
            get_local_ip()
                .map(|info| format!("http://{}:{}", info.ip, info.port))
                .unwrap_or_else(|_| "http://127.0.0.1:3000".to_string())
        });
    
    // Return the full URL
    Ok(format!("{}/get/{}", base_url, link_id))
}