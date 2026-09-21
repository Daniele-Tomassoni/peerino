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
// Stream file via Tauri Channel with flow control (ack-based backpressure).
// Uses 64KB buffer and streaming (NOT Vec<u8> for the whole file).
// FIX B: Added stream_id, Notify-based ack, timeout, explicit cleanup helper.

use tauri::{State, ipc::Channel};
use crate::AppState;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Notify;

/// Helper: remove the Notify for a given stream_id from the shared map.
/// Called explicitly before every return path (success, error, cancel, timeout)
/// to guarantee cleanup without relying on Drop + spawn (which can fail if the
/// Tokio runtime is already shutting down).
async fn cleanup_stream_ack(state: &AppState, stream_id: &str) {
    let mut acks = state.stream_acks.lock().await;
    acks.remove(stream_id);
}

/// Stream a file via Tauri Channel with flow control.
/// Sends 64KB chunks sequentially to prevent OOM.
/// After every K chunks (default 8 = 512KB), waits for an ack from the JS
/// via the `stream_ack` command before sending the next block.
/// If no ack arrives within 30s, the stream is aborted with an error.
#[tauri::command]
pub async fn stream_file(
    state: State<'_, AppState>,
    hash: String,
    channel: Channel<Vec<u8>>,
    stream_id: String,
    chunk_limit: Option<usize>,
) -> Result<(), String> {
    // Validate hash
    if hash.is_empty() {
        return Err("File hash cannot be empty".to_string());
    }

    // FIX B AGGIUNTA 3: register cancellation flag before starting the loop
    // to avoid a race condition where cancel_download is called before the
    // flag exists. The flag is reused if already present and not cancelled.
    state.download_tracker.register_cancellation_flag(&hash).await;

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

    // FIX B: create or retrieve the Arc<Notify> for this stream_id
    let k = chunk_limit.unwrap_or(8);
    {
        let mut acks = state.stream_acks.lock().await;
        acks.entry(stream_id.clone())
            .or_insert_with(|| Arc::new(Notify::new()));
    }

    // Get a reference to the Notify (clone the Arc to avoid holding the lock)
    let notify = {
        let acks = state.stream_acks.lock().await;
        acks.get(&stream_id).cloned()
            .ok_or_else(|| "Failed to get Notify for stream".to_string())?
    };

    // Stream in 64KB chunks
    let mut buffer = vec![0u8; 64 * 1024]; // 64KB buffer
    let mut chunks_in_block: usize = 0;

    let result: Result<(), String> = async {
        loop {
            // Check cancellation flag (frontend "X" button calls cancel_download
            // which sets this flag). The check is cheap (atomic load + try_lock)
            // and lets the user abort a long-running stream without waiting for EOF.
            // FIX #5: is_cancelled is now synchronous (try_lock) — no .await needed
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

            chunks_in_block += 1;

            // FIX B: flow control — wait for ack after every K chunks
            if chunks_in_block >= k {
                // FIX B AGGIUNTA 2: timeout 30s on ack using tokio::select!
                // (notified() is not consumed by the timeout branch, so the
                // Notify remains valid for the next ack cycle)
                tokio::select! {
                    _ = notify.notified() => {
                        // ack received from JS via stream_ack command
                    }
                    _ = tokio::time::sleep(std::time::Duration::from_secs(30)) => {
                        log::warn!("Timeout waiting for ack on stream {}", stream_id);
                        return Err("Timeout waiting for ack (JS unresponsive)".to_string());
                    }
                }
                chunks_in_block = 0;
            }
        }

        Ok(())
    }
    .await;

    // FIX B AGGIUNTA 4: explicit cleanup on every exit path
    cleanup_stream_ack(&state, &stream_id).await;

    result
}

/// Ack command: called by the JS frontend every K chunks to allow the
/// Rust stream_file command to send the next block of chunks.
#[tauri::command]
pub async fn stream_ack(
    state: State<'_, AppState>,
    stream_id: String,
) -> Result<(), String> {
    let acks = state.stream_acks.lock().await;
    if let Some(notify) = acks.get(&stream_id) {
        notify.notify_one();
    }
    Ok(())
}
