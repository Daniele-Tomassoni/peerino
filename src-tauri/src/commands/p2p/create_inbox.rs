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
// Create reverse inbox for P2P-to-Web file receiving
// Allows browsers to upload files directly to the peer via WebRTC
//
// Uses P2P_WEB_URL (Netlify, later https://peerino.com) as the base URL for the receiver page.
// The page connects to the sender via PeerJS and sends files directly (no relay).
//
// Signaling configuration embedded in the link (env-driven). TURN credentials
// are fetched by the receiver from the Worker.

use tauri::State;
use crate::AppState;
use crate::utils::turn_creds::signaling_url_from_env;
use urlencoding::encode;

/// Create a reverse inbox for receiving files from browsers via WebRTC.
/// Returns a URL (with ?mode=inbox&peerId=...) that opens the unified web page
/// (served at Netlify root), which connects to the sender via PeerJS.
#[tauri::command]
pub async fn create_inbox(state: State<'_, AppState>) -> Result<String, String> {
    // Get the current PeerID (updated by PeerJS on connect)
    let peer_id = state.peer_id.lock().await.clone()
        .ok_or_else(|| "PeerID unavailable. Ensure PeerJS is connected.".to_string())?;

    // Store the inbox ID in the relay manager for tracking (24h expiry + cleanup)
    let inbox_id = {
        let relay = state.relay_manager.lock().await;
        relay.create_inbox().await.map_err(|e| e.to_string())?
    };

    // Use P2P_WEB_URL (env) as the base URL: reachable from the internet.
    let page_base = std::env::var("P2P_WEB_URL")
        .map(|u| u.trim_end_matches('/').to_string())
        .unwrap_or_else(|_| {
            log::warn!("P2P_WEB_URL not set, using fallback to peerino.com");
            "https://peerino.com".to_string()
        });

    // Build the link with mode=inbox and peerId (for WebRTC connection)
    let mut link = format!(
        "{}?mode=inbox&peerId={}&id={}",
        page_base, peer_id, inbox_id
    );

    // Optional signaling override; TURN is fetched by the receiver.
    if let Some(sig) = signaling_url_from_env() {
        if sig != "0.peerjs.com" {
            link.push_str(&format!("&signal={}", encode(&sig)));
        }
    }


    log::info!("📥 Created reverse inbox link: {}", link);

    Ok(link)
}
