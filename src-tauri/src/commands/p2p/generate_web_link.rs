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
// Generate a P2P-to-Web link that includes the sender's PeerID, file hash and filename.
// The recipient opens the link and the web page auto-connects to the sender via PeerJS
// (direct browser<->sender P2P transfer, NO server relay).
//
// Signaling configuration embedded in the link (env-driven):
// - signal=   : self-hosted signaling host (SIGNALING_URL); omitted = cloud 0.peerjs.com
// - TURN credentials are fetched by the receiver from the Worker.
// - The receiver can still be opened manually from the local HTTP server.
//
// The signaling_url command override is still accepted for compatibility.

use tauri::State;
use crate::AppState;
use crate::utils::turn_creds::signaling_url_from_env;
use urlencoding::encode;

/// Generate a unique P2P-to-Web link that includes the PeerID, hash and filename of the sender.
/// The recipient can open the link and connect automatically without typing anything.
#[tauri::command]
pub async fn generate_web_link(
    state: State<'_, AppState>,
    hash: String,
    signaling_url: Option<String>,
) -> Result<String, String> {
    // 1. Get file info from index (validates existence and gets filename)
    let file_info = {
        let file_index = state.file_index.lock().await;
        file_index.get(&hash).cloned()
            .ok_or_else(|| "File not found in index".to_string())?
        };

    // 1b. FIX #3: Verify the file actually exists on disk.
    // This prevents generating a link for a file that is in the index
    // but was deleted/moved, which would otherwise cause a silent hang
    // on the receiver side.
    let file_path = std::path::Path::new(&state.shared_folder).join(&file_info.filename);
    if !file_path.exists() {
        return Err(format!("File not present on disk: {}", file_info.filename));
    }

    // 2. Get the current PeerID (updated by PeerJS on connect)
    let peer_id = state.peer_id.lock().await.clone()
        .ok_or_else(|| "PeerID unavailable. Ensure PeerJS is connected.".to_string())?;

    // 3. Store the pending file hash for automatic P2P-to-Web transfer
    {
        let mut pending = state.pending_file_hash.lock().await;
        *pending = Some(hash.clone());
    }

    // 4. Determine the base URL of the receiver page.
    // Uses P2P_WEB_URL (env) as the base URL: reachable from the internet.
    let page_base = std::env::var("P2P_WEB_URL")
        .map(|u| u.trim_end_matches('/').to_string())
        .unwrap_or_else(|_| {
            log::warn!("P2P_WEB_URL not set, using fallback to peerino.com");
            "https://peerino.com".to_string()
        });

    // 5. Build the link with explicit mode, peerId, hash and filename (URL encoded)
    let encoded_filename = encode(&file_info.filename);
    let mut link = format!(
        "{}?mode=download&peerId={}&hash={}&filename={}",
        page_base, peer_id, hash, encoded_filename
    );

    // 6. Optional signaling override; ICE is fetched by the receiver.
    let signal = signaling_url
        .filter(|s| !s.is_empty())
        .or_else(signaling_url_from_env);
    if let Some(sig) = signal {
        if sig != "0.peerjs.com" {
            link.push_str(&format!("&signal={}", encode(&sig)));
        }
    }

    // LAN hint: pass the local server address as a parameter.
    // The Netlify page will try to use it as a direct fallback, but if the
    // browser blocks due to mixed content, it will automatically fall back to

    log::info!("🔗 Generated P2P-to-Web link: {}", link);

    Ok(link)
}
