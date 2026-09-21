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
// Discard an in-progress incoming upload (browser → app).
// Removes the UploadState, the temp file on disk, and the cancellation flag.
// Idempotent: safe to call multiple times (cancel + close, etc.).

use tauri::State;
use crate::AppState;

/// Discard an incoming upload for a given peer_id.
///
/// 1. Removes the UploadState from incoming_uploads (drops the File handle,
///    which closes it on Windows).
/// 2. Removes the temp file from disk (NotFound is OK — idempotent).
/// 3. Removes the cancellation flag from download_tracker to prevent leaks.
///
/// Safe to call multiple times: if the upload is already gone, returns Ok.
#[tauri::command]
pub async fn discard_incoming_upload(
    state: State<'_, AppState>,
    peer_id: String,
) -> Result<(), String> {
    log::info!("🗑️ discard_incoming_upload: peer_id={}", peer_id);

    // CORREZIONE 1: remove the UploadState FIRST (drops File → closes it on
    // Windows). Only THEN delete the temp file. On Windows, remove_file fails
    // if the file handle is still open.
    let upload = {
        let mut uploads = state.incoming_uploads.lock().await;
        uploads.remove(&peer_id)
    };

    if let Some(upload) = upload {
        // CORREZIONE 3: remove the cancellation flag to prevent memory leak
        // in download_tracker.cancelled_flags.
        if !upload.expected_hash.is_empty() {
            let mut flags = state.download_tracker.cancelled_flags.lock().await;
            flags.remove(&upload.expected_hash);
        }

        // Delete the temp file. NotFound is OK (idempotent).
        match tokio::fs::remove_file(&upload.temp_path).await {
            Ok(_) => {
                log::info!("🗑️ Temp file discarded: {:?}", upload.temp_path);
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                log::info!("🗑️ Temp file already gone: {:?}", upload.temp_path);
            }
            Err(e) => {
                log::warn!("⚠️ Failed to remove temp file {:?}: {}", upload.temp_path, e);
            }
        }
    } else {
        log::info!("🗑️ No upload state found for peer_id={} (already discarded)", peer_id);
    }

    Ok(())
}