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
// - lan=      : address of the integrated HTTP server (only if it is running), so the
//   receiver page can offer a direct-LAN fallback when WebRTC is unavailable.
//
// Manual overrides (signaling_url / turn_username / turn_password) are still accepted
// for backward compatibility and take precedence ONLY when env TURN is not configured.

use tauri::State;
use crate::AppState;
use crate::utils::ice_provider::{self, IceResolution};
use crate::utils::network::get_local_ip;
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
            .ok_or_else(|| "File non trovato nell'indice".to_string())?
    };

    // 1b. FIX #3: Verify the file actually exists on disk.
    // This prevents generating a link for a file that is in the index
    // but was deleted/moved, which would otherwise cause a silent hang
    // on the receiver side.
    let file_path = std::path::Path::new(&state.shared_folder).join(&file_info.filename);
    if !file_path.exists() {
        return Err(format!("File non presente su disco: {}", file_info.filename));
    }

    // 2. Get the current PeerID (updated by PeerJS on connect)
    let peer_id = state.peer_id.lock().await.clone()
        .ok_or_else(|| "PeerID non disponibile. Assicurati che PeerJS sia connesso.".to_string())?;

    // 3. Store the pending file hash for automatic P2P-to-Web transfer
    {
        let mut pending = state.pending_file_hash.lock().await;
        *pending = Some(hash.clone());
    }

    // 4. Determine the base URL of the receiver page.
    // Usa P2P_WEB_URL (Netlify) come base URL: raggiungibile da internet.
    // Il fallback LAN viene tentato tramite il parametro 'lan=' aggiunto sotto.
    let page_base = std::env::var("P2P_WEB_URL")
        .map(|u| u.trim_end_matches('/').to_string())
        .unwrap_or_else(|_| "https://courageous-crisp-cff298.netlify.app".to_string());

    // 5. Build the link with explicit mode, peerId, hash and filename (URL encoded)
    let encoded_filename = encode(&file_info.filename);
    let mut link = format!(
        "{}?mode=download&peerId={}&hash={}&filename={}",
        page_base, peer_id, hash, encoded_filename
    );

    // 6. ICE/signaling configuration: provider unificato (metered > coturn > static).
    //    Risolve automaticamente quale provider usare in base alle env, con
    //    fetch asincrono per metered.ca. Restituisce un IceResolution che include
    //    l'array iceServers pronto per il browser (formato WebRTC standard).
    let resolution: IceResolution = ice_provider::fetch_ice_servers(None).await;
    let cfg = &resolution.config;

    // Override manuale del signaling (parametro esplicito del comando Tauri).
    let signal = signaling_url
        .filter(|s| !s.is_empty())
        .or_else(|| cfg.signaling_url.clone());
    if let Some(sig) = signal {
        link.push_str(&format!("&signal={}", encode(&sig)));
    }

    // NOTA: STUN e TURN sono ora inclusi SOLO nel parametro `&ice=<base64>` sotto.
    // Rimossi i parametri ridondanti `&stunUrls=` e `&turnUrls/turnUser/turnPass=`
    // perché il parametro `&ice` (formato WebRTC standard) li contiene già tutti.
    // Questo evita config duplicata/conflict nel browser.

    // TURN: credenziali statiche legacy (backward compat). Se il chiamante passa
    // esplicitamente turn_username + turn_password e NON c'è un TURN provider
    // configurato, li aggiungiamo come fallback legacy (parametri separati).
    if cfg.turn.is_none() {
        if let (Some(u), Some(p)) = (
            turn_username.filter(|s| !s.is_empty()),
            turn_password.filter(|s| !s.is_empty()),
        ) {
            link.push_str(&format!("&turnUser={}&turnPass={}", encode(&u), encode(&p)));
        }
    }

    // Provider TURN unificato: passa l'intero array iceServers come parametro
    // base64. Il browser lo deserializza e lo passa direttamente a RTCPeerConnection.
    // Formato: &ice=<base64(JSON)>. Questo è il modo raccomandato per il browser
    // perché supporta QUALSIASI provider (metered, coturn self-hosted, misto).
    let browser_ice = ice_provider::build_browser_ice_servers(&resolution);
    if !browser_ice.is_empty() {
        let encoded = ice_provider::encode_ice_servers_param(&browser_ice);
        link.push_str(&format!("&ice={}", encode(&encoded)));

        // Diagnostica dettagliata: quanti STUN, quanti TURN, quale provider.
        let n_stun = browser_ice.iter()
            .filter(|e| e.credential.is_none()
                && e.urls.iter().all(|u| u.starts_with("stun:")))
            .count();
        let n_turn = browser_ice.iter()
            .filter(|e| e.credential.is_some())
            .count();
        log::info!(
            "❄️ Link include {} iceServers via {:?} provider ({} STUN, {} TURN)",
            browser_ice.len(), resolution.provider, n_stun, n_turn
        );

        // Warning esplicito quando il link è solo-STUN: l'utente (e lo sviluppatore
        // nei log) capisce subito perché la connessione potrebbe fallire su NAT
        // simmetrico o CGNAT. Suggerisce anche la remediation concreta.
        if n_turn == 0 {
            log::warn!("⚠️  Link SENZA TURN servers: solo STUN. Connessioni P2P su");
            log::warn!("   NAT simmetrico o dietro CGNAT (es. Iliad/Ho.Mobile) falliranno.");
            log::warn!("   Remediation: upgrade metered a piano paid OPPURE configura");
            log::warn!("   un VPS con coturn self-hosted (TURN_URLS + TURN_AUTH_SECRET).");
        }

        if let Some(warn) = &resolution.warning {
            log::warn!("ICE provider warning: {}", warn);
        }
    }

    // FIX ALTO: passa il limite TURN corrente nel link come `&turnMax=`.
    // Il browser lo usa per il controllo overlimit invece del valore hardcoded
    // 104857600, garantendo che browser e backend siano sempre sincronizzati.
    // Usiamo la funzione centralizzata per coerenza con il resto del backend.
    let turn_max = crate::commands::get_turn_max_file_size();
    link.push_str(&format!("&turnMax={}", turn_max));

    // LAN hint: passa l'indirizzo del server locale come parametro.
    // La pagina Netlify proverà a usarlo come fallback diretto, ma se il
    // browser blocca per mixed content, farà automaticamente fallback a
    // WebRTC con STUN → TURN. Il parametro non rompe il flusso.
    if *state.server_running.lock().await {
        if let Ok(info) = get_local_ip() {
            link.push_str(&format!("&lan=http://{}:{}", info.ip, info.port));
        }
    }

    log::info!("🔗 Generated P2P-to-Web link: {}", link);

    Ok(link)
}
