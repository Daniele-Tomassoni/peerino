// Persistent PeerID storage
//
// Stores a stable PeerJS PeerID in config/peer_id.json so that generated
// P2P-to-Web links remain valid across app restarts. Without this, PeerJS
// assigns a new random ID on every launch, which orphans any previously
// generated link (the receiver hangs on "Connessione al mittente...").

use std::fs;
use std::path::PathBuf;

/// Read the persistent PeerID from config/peer_id.json, or create and store
/// a new one if the file is missing or invalid.
pub fn get_or_create_peer_id(config_dir: &str) -> Result<String, String> {
    let path = PathBuf::from(config_dir).join("peer_id.json");

    if path.exists() {
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(id) = serde_json::from_str::<String>(&content) {
                if !id.is_empty() {
                    return Ok(id);
                }
            }
        }
    }

    // Generate a new stable ID and persist it
    let new_id = format!("peerino-{}", uuid::Uuid::new_v4());
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    fs::write(&path, serde_json::to_string(&new_id).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    Ok(new_id)
}

#[tauri::command]
pub async fn get_persistent_peer_id() -> Result<String, String> {
    // Usa la stessa directory dell'app (dove si trova l'eseguibile)
    let config_dir = format!("{}/config", super::get_app_dir());
    get_or_create_peer_id(&config_dir)
}
