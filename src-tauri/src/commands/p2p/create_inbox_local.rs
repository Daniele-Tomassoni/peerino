// Create reverse inbox for local network file receiving.
// The recipient opens http://{IP}:3000/inbox/{id} and uploads the file with a
// plain HTTP POST (raw body streaming): NO PeerJS signaling, NO TURN, works
// even on an isolated LAN. The inbox id is validated server-side on both GET
// (page) and POST (upload) and expires after RelayConfig::link_expiry_seconds.
//
// NOTE: this is intentionally pure HTTP. The WebRTC receiver page
// (/receiver?mode=inbox) remains available for the INTERNET inbox only,
// where PeerJS signaling is actually required.

use tauri::State;
use crate::AppState;
use crate::utils::network::get_local_ip;

/// Create a reverse inbox for receiving files from browsers on the local network.
/// Returns a URL of the form http://{ip}:{port}/inbox/{id} served by the
/// integrated HTTP server (axum). The transfer is a plain HTTP POST:
/// no external signaling service is involved.
#[tauri::command]
pub async fn create_inbox_local(
    state: State<'_, AppState>,
) -> Result<String, String> {
    // Track the inbox in the relay manager (24h expiry + hourly cleanup task)
    let inbox_id = {
        let relay = state.relay_manager.lock().await;
        relay.create_inbox().await.map_err(|e| e.to_string())?
    };

    // Get the local IP address for the URL
    let network_info = get_local_ip()
        .map_err(|e| format!("Impossibile ottenere IP locale: {}", e))?;

    // Direct link to the HTTP upload page (no redirect, no WebRTC params)
    let link = format!(
        "http://{}:{}/inbox/{}",
        network_info.ip,
        crate::server::DEFAULT_HTTP_PORT,
        inbox_id
    );

    log::info!("📥 Created local inbox link: {}", link);

    Ok(link)
}
