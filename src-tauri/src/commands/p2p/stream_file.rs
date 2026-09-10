// Stream file via Tauri Channel
// Uses 64KB buffer and streaming (NOT Vec<u8> for the whole file)

use tauri::{State, ipc::Channel};
use crate::AppState;
use std::path::Path;

/// Stream a file via Tauri Channel
/// Sends 64KB chunks sequentially to prevent OOM
#[tauri::command]
pub async fn stream_file(
    state: State<'_, AppState>,
    hash: String,
    channel: Channel<Vec<u8>>,
) -> Result<(), String> {
    // Validate hash
    if hash.is_empty() {
        return Err("File hash cannot be empty".to_string());
    }

    // Find file in index
    let file_info = {
        let file_index = state.file_index.lock().await;
        file_index.get(&hash).cloned()
    };

    let file_info = file_info.ok_or_else(|| "File not found in index".to_string())?;

    // Build file path
    let file_path = Path::new(&state.shared_folder)
        .join(&file_info.filename);

    // Open file in streaming mode with tokio::fs
    let mut file = tokio::fs::File::open(&file_path)
        .await
        .map_err(|e| format!("Failed to open file: {}", e))?;

    // Stream in 64KB chunks
    let mut buffer = vec![0u8; 64 * 1024]; // 64KB buffer

    loop {
        // Check cancellation flag (frontend "X" button calls cancel_download
        // which sets this flag). The check is cheap (atomic load + try_lock)
        // and lets the user abort a long-running stream without waiting for EOF.
        // FIX #5: is_cancelled è ora sincrono (try_lock) — non serve .await
        if state.download_tracker.is_cancelled(&hash) {
            log::info!("Stream cancelled by user: {}", hash);
            return Err("Stream cancelled".to_string());
        }

        let bytes_read = tokio::io::AsyncReadExt::read(&mut file, &mut buffer)
            .await
            .map_err(|e| format!("Failed to read file: {}", e))?;

        if bytes_read == 0 {
            break; // EOF
        }

        // Send chunk via Channel (NOT returning Vec<u8>)
        let chunk = buffer[..bytes_read].to_vec();
        channel.send(chunk)
            .map_err(|e| format!("Failed to send chunk: {}", e))?;
    }

    Ok(())
}
