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
// ICE/signaling configuration embedded in the link (env-driven):
// - signal=   : self-hosted signaling host (SIGNALING_URL); omitted = cloud 0.peerjs.com
// - stunUrls= : STUN servers (STUN_URLS); default Google public STUN
// - turnUrls/turnUser/turnPass : EPHEMERAL credentials derived via HMAC-SHA1 from
//   TURN_AUTH_SECRET (coturn REST scheme). The secret never travels in links and
//   credentials expire after TURN_CRED_TTL_SECS.
// - The receiver can still be opened manually from the local HTTP server.
//
// Manual overrides (signaling_url / turn_username / turn_password) are still accepted
// for backward compatibility and take precedence ONLY when env TURN is not configured.

use tauri::State;
use crate::AppState;
use crate::utils::ice_provider::{self, IceResolution};
use urlencoding::encode;

/// Generate a unique P2P-to-Web link that includes the PeerID, hash and filename of the sender.
/// The recipient can open the link and connect automatically without typing anything.
#[tauri::command]
pub async fn generate_web_link(
    state: State<'_, AppState>,
    hash: String,
    signaling_url: Option<String>,
    turn_username: Option<String>,
    turn_password: Option<String>,
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

    // 6. ICE/signaling configuration: unified provider (metered > coturn > static).
    //    Automatically resolves which provider to use based on env, with
    //    async fetch for metered.ca. Returns an IceResolution that includes
    //    the iceServers array ready for the browser (WebRTC standard format).
    let resolution: IceResolution = ice_provider::fetch_ice_servers(None).await;
    let cfg = &resolution.config;

    // Manual signaling override (explicit Tauri command parameter).
    let signal = signaling_url
        .filter(|s| !s.is_empty())
        .or_else(|| cfg.signaling_url.clone());
    if let Some(sig) = signal {
        if sig != "0.peerjs.com" {
            link.push_str(&format!("&signal={}", encode(&sig)));
        }
    }

    // NOTE: STUN and TURN are now included ONLY in the `&ice=<base64>` parameter below.
    // Redundant `&stunUrls=` and `&turnUrls/turnUser/turnPass=` parameters removed
    // because the `&ice` parameter (WebRTC standard format) already contains them all.
    // This avoids duplicate/conflicting config in the browser.

    // TURN: legacy static credentials (backward compat). If the caller passes
    // explicit turn_username + turn_password and NO TURN provider is
    // configured, we add them as a legacy fallback (separate parameters).
    if cfg.turn.is_none() {
        if let (Some(u), Some(p)) = (
            turn_username.filter(|s| !s.is_empty()),
            turn_password.filter(|s| !s.is_empty()),
        ) {
            link.push_str(&format!("&turnUser={}&turnPass={}", encode(&u), encode(&p)));
        }
    }

    // Unified TURN provider: passes the entire iceServers array as a
    // base64 parameter. The browser deserializes it and passes it directly to
    // RTCPeerConnection. Format: &ice=<base64(JSON)>. This is the recommended
    // approach for the browser because it supports ANY provider
    // (metered, self-hosted coturn, mixed).
    let browser_ice = ice_provider::build_browser_ice_servers(&resolution);
    if !browser_ice.is_empty() {
        let encoded = ice_provider::encode_ice_servers_param(&browser_ice);
        link.push_str(&format!("&ice={}", encode(&encoded)));

        // Detailed diagnostics: how many STUN, how many TURN, which provider.
        let n_stun = browser_ice.iter()
            .filter(|e| e.credential.is_none()
                && e.urls.iter().all(|u| u.starts_with("stun:")))
            .count();
        let n_turn = browser_ice.iter()
            .filter(|e| e.credential.is_some())
            .count();
        log::info!(
            "❄️ Link includes {} iceServers via {:?} provider ({} STUN, {} TURN)",
            browser_ice.len(), resolution.provider, n_stun, n_turn
        );

        // Explicit warning when the link is STUN-only: the user (and the developer
        // reading the logs) immediately understands why the connection might fail
        // on symmetric NAT or CGNAT. Also suggests concrete remediation.
        if n_turn == 0 {
            log::warn!("⚠️  Link WITHOUT TURN servers: STUN only. P2P connections on");
            log::warn!("   symmetric NAT or behind CGNAT (e.g. Iliad/Ho.Mobile) will fail.");
            log::warn!("   Remediation: upgrade metered to a paid plan OR configure");
            log::warn!("   a VPS with self-hosted coturn (TURN_URLS + TURN_AUTH_SECRET).");
        }

        if let Some(warn) = &resolution.warning {
            log::warn!("ICE provider warning: {}", warn);
        }
    }

    // Always pass the current TURN limit so the receiver uses the same value.
    let turn_max = crate::commands::get_turn_max_file_size();
    link.push_str(&format!("&turnMax={}", turn_max));

    // LAN hint: pass the local server address as a parameter.
    // The Netlify page will try to use it as a direct fallback, but if the
    // browser blocks due to mixed content, it will automatically fall back to

    log::info!("🔗 Generated P2P-to-Web link: {}", link);

    Ok(link)
}
