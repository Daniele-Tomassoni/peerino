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