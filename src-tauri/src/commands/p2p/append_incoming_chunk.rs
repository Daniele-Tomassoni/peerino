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