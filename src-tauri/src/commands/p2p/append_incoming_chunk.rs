// Append a chunk to an in-progress upload (browser → app)
// Writes to temp file incrementally, no memory accumulation

use tauri::State;
use crate::AppState;
use sha2::Digest;
use tokio::io::AsyncWriteExt;

#[tauri::command]
pub async fn append_incoming_chunk(
    state: State<'_, AppState>,
    peer_id: String,
    chunk: Vec<u8>,
) -> Result<(), String> {
    log::info!("📥 append_incoming_chunk: peer_id={}, chunk_size={}", peer_id, chunk.len());
    let mut uploads = state.incoming_uploads.lock().await;
    let upload = uploads.get_mut(&peer_id)
        .ok_or_else(|| format!("Upload non inizializzato per peer_id={}", peer_id))?;

    upload.file.write_all(&chunk).await
        .map_err(|e| format!("Errore scrittura chunk: {}", e))?;
    upload.hasher.update(&chunk);
    upload.written += chunk.len() as u64;
    log::info!("✅ Chunk written, total_written={}", upload.written);

    Ok(())
}