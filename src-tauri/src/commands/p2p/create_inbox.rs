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
// ICE/signaling configuration embedded in the link (env-driven), same scheme as
// generate_web_link but with the TURN credential TTL ALIGNED to the inbox validity
// (24h): without this alignment ephemeral credentials would expire before the inbox,
// silently degrading NAT-traversal to STUN-only after a couple of hours.

use tauri::State;
use crate::AppState;
use crate::utils::ice_provider::{self, IceResolution};
use urlencoding::encode;

/// Inbox validity in seconds (must match RelayConfig::link_expiry_seconds).
const INBOX_EXPIRY_SECS: u64 = 24 * 60 * 60;

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

    // ICE configuration with TTL aligned to inbox validity (24h): credentials
    // remain valid for the entire life of the inbox. Uses the unified provider
    // (metered > coturn > static) to support any future TURN provider.
    let resolution: IceResolution = ice_provider::fetch_ice_servers(Some(INBOX_EXPIRY_SECS)).await;
    let cfg = &resolution.config;

    if let Some(sig) = &cfg.signaling_url {
        if sig != "0.peerjs.com" {
            link.push_str(&format!("&signal={}", encode(&sig)));
        }
    }

    // NOTE: STUN and TURN are now included ONLY in the `&ice=<base64>` parameter below.
    // Redundant `&stunUrls=` and `&turnUrls/turnUser/turnPass=` parameters removed
    // because the `&ice` parameter (WebRTC standard format) already contains them all.

    // Unified TURN provider: passes the entire iceServers array as a parameter
    // base64 (format &ice=...). Supports any provider (metered/coturn/...).
    let browser_ice = ice_provider::build_browser_ice_servers(&resolution);
    if !browser_ice.is_empty() {
        let encoded = ice_provider::encode_ice_servers_param(&browser_ice);
        link.push_str(&format!("&ice={}", encode(&encoded)));

        let n_turn = browser_ice.iter().filter(|e| e.credential.is_some()).count();
        log::info!(
            "Inbox link: {} iceServers via {:?} provider (TTL 24h, {} TURN)",
            browser_ice.len(), resolution.provider, n_turn
        );

        if n_turn == 0 {
            log::warn!("Inbox WITHOUT TURN: STUN only. Symmetric NAT/CGNAT will not work. Consider metered upgrade or VPS coturn.");
        }
    }

    // Always pass the current TURN limit so the receiver uses the same value.
    let turn_max = crate::commands::get_turn_max_file_size();
    link.push_str(&format!("&turnMax={}", turn_max));


    log::info!("📥 Created reverse inbox link: {}", link);

    Ok(link)
}
