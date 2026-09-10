// Read file chunks for P2P transfer
// Returns base64-encoded chunks to send over DataConnection

use tauri::State;
use crate::AppState;
use std::path::Path;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncSeekExt;
use base64::Engine;

const CHUNK_SIZE: usize = 256 * 1024; // 256KB

/// Get file info for P2P transfer
#[tauri::command]
pub async fn get_file_info(
    state: State<'_, AppState>,
    hash: String,
) -> Result<serde_json::Value, String> {
    let file_info = {
        let file_index = state.file_index.lock().await;
        file_index.get(&hash).cloned()
    };

    let file_info = file_info.ok_or_else(|| "File not found in index".to_string())?;

    Ok(serde_json::json!({
        "filename": file_info.filename,
        "size": file_info.size,
        "hash": file_info.hash
    }))
}

/// Read a file chunk and return as base64
#[tauri::command]
pub async fn read_file_chunk(
    state: State<'_, AppState>,
    hash: String,
    offset: u64,
) -> Result<Option<String>, String> {
    // Check cancellation flag before reading the next chunk.
    // The frontend cancel button calls cancel_upload({ hash }) which sets
    // this flag; we abort the transfer here instead of streaming forever.
    // FIX #5: is_cancelled è ora sincrono (try_lock) — non serve .await
    if state.upload_tracker.is_cancelled(&hash) {
        log::info!("P2P upload cancelled by user: {}", hash);
        return Err("Upload cancelled".to_string());
    }

    // Find file in index
    let file_info = {
        let file_index = state.file_index.lock().await;
        file_index.get(&hash).cloned()
    };

    let file_info = match file_info {
        Some(info) => info,
        None => return Err("File not found in index".to_string()),
    };

    // Build file path
    let file_path = Path::new(&state.shared_folder)
        .join(&file_info.filename);

    // Open file
    let mut file = tokio::fs::File::open(&file_path)
        .await
        .map_err(|e| format!("Failed to open file: {}", e))?;

    // Seek to offset
    file.seek(std::io::SeekFrom::Start(offset))
        .await
        .map_err(|e| format!("Failed to seek file: {}", e))?;

    // Read chunk
    let mut buffer = vec![0u8; CHUNK_SIZE];
    let bytes_read = file.read(&mut buffer)
        .await
        .map_err(|e| format!("Failed to read file: {}", e))?;

    if bytes_read == 0 {
        return Ok(None); // EOF
    }

    // Return base64 encoded chunk
    let chunk = base64::engine::general_purpose::STANDARD.encode(&buffer[..bytes_read]);
    Ok(Some(chunk))
}

/// Get the pending file hash for P2P-to-Web transfer
/// This is called when a web page connects to get the file to send
#[tauri::command]
pub async fn get_pending_file_hash(
    state: State<'_, AppState>,
) -> Result<Option<String>, String> {
    let pending = state.pending_file_hash.lock().await;
    Ok(pending.clone())
}

/// Clear the pending file hash after transfer
#[tauri::command]
pub async fn clear_pending_file_hash(
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut pending = state.pending_file_hash.lock().await;
    *pending = None;
    Ok(())
}