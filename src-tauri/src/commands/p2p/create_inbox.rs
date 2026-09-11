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
// Create reverse inbox for P2P-to-Web file receiving
// Allows browsers to upload files directly to the peer via WebRTC
//
// Uses P2P_WEB_URL (Netlify, poi https://peerino.com) as the base URL for the receiver page.
// The page connects to the sender via PeerJS and sends files directly (no relay).
//
// ICE/signaling configuration embedded in the link (env-driven), same scheme as
// generate_web_link but with the TURN credential TTL ALIGNED to the inbox validity
// (24h): without this alignment ephemeral credentials would expire before the inbox,
// silently degrading NAT-traversal to STUN-only after a couple of hours.

use tauri::State;
use crate::AppState;
use crate::utils::ice_provider::{self, IceResolution};
use crate::utils::network::get_local_ip;
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
        .ok_or_else(|| "PeerID non disponibile. Assicurati che PeerJS sia connesso.".to_string())?;

    // Store the inbox ID in the relay manager for tracking (24h expiry + cleanup)
    let inbox_id = {
        let relay = state.relay_manager.lock().await;
        relay.create_inbox().await.map_err(|e| e.to_string())?
    };

    // Usa P2P_WEB_URL (Netlify) come base URL: raggiungibile da internet.
    // Il fallback LAN viene tentato tramite il parametro 'lan=' aggiunto sotto:
    // - Se mittente e destinatario sono sulla stessa rete, il browser proverà
    //   il fetch al server locale ma potrebbe essere bloccato per mixed content
    //   (pagina HTTPS che chiama HTTP). In tal caso, fallback automatico a WebRTC.
    // - Il parametro 'lan=' non rompe nulla: è solo un tentativo ottimistico.
    let page_base = std::env::var("P2P_WEB_URL")
        .map(|u| u.trim_end_matches('/').to_string())
        .unwrap_or_else(|_| "https://courageous-crisp-cff298.netlify.app".to_string());

    // Build the link with mode=inbox and peerId (for WebRTC connection)
    let mut link = format!(
        "{}?mode=inbox&peerId={}&id={}",
        page_base, peer_id, inbox_id
    );

    // ICE configuration con TTL allineato alla validità inbox (24h): le credenziali
    // restano valide per tutta la vita dell'inbox. Usa il provider unificato
    // (metered > coturn > static) per supportare qualunque provider TURN futuro.
    let resolution: IceResolution = ice_provider::fetch_ice_servers(Some(INBOX_EXPIRY_SECS)).await;
    let cfg = &resolution.config;

    if let Some(sig) = &cfg.signaling_url {
        link.push_str(&format!("&signal={}", encode(&sig)));
    }

    // NOTA: STUN e TURN sono ora inclusi SOLO nel parametro `&ice=<base64>` sotto.
    // Rimossi i parametri ridondanti `&stunUrls=` e `&turnUrls/turnUser/turnPass=`
    // perché il parametro `&ice` (formato WebRTC standard) li contiene già tutti.

    // Provider TURN unificato: passa l'intero array iceServers come parametro
    // base64 (formato &ice=...). Supporta qualsiasi provider (metered/coturn/...).
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
            log::warn!("Inbox SENZA TURN: solo STUN. NAT simmetrico/CGNAT non funzioneranno. Considera upgrade metered o VPS coturn.");
        }
    }

    // FIX HIGH: pass the current TURN limit into the link as `&turnMax=`.
    // The browser uses it for overlimit control instead of the hardcoded
    // 104857600 value, ensuring browser and backend stay in sync.
    let turn_max = crate::commands::get_turn_max_file_size();
    link.push_str(&format!("&turnMax={}", turn_max));

    // LAN hint: pass the local server address as a parameter.
    // The Netlify page will try to use it as a direct fallback, but if the
    // browser blocks due to mixed content, it will automatically fall back to
    // WebRTC with STUN → TURN. The parameter does not break the flow.
    if *state.server_running.lock().await {
        if let Ok(info) = get_local_ip() {
            link.push_str(&format!("&lan=http://{}:{}", info.ip, info.port));
        }
    }

    log::info!("📥 Created reverse inbox link: {}", link);

    Ok(link)
}
