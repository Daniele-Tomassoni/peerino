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
// Finalize an incoming upload (browser → app)
// Verifies hash, moves temp file to shared folder, persists to DB

use std::path::Path;
use tauri::State;
use crate::{AppState, FileInfo};
use crate::utils::is_safe_filename;
use crate::database::files::FileRepository;
use sha2::Digest;

#[tauri::command]
pub async fn finalize_incoming_file(
    state: State<'_, AppState>,
    peer_id: String,
) -> Result<String, String> {
    log::info!("📥 finalize_incoming_file: peer_id={}", peer_id);
    // Remove and take ownership of the upload state
    let mut uploads = state.incoming_uploads.lock().await;
    let upload = uploads.remove(&peer_id)
        .ok_or_else(|| format!("Upload not found for peer_id={}", peer_id))?;

    // Reject incomplete uploads before any integrity or persistence operation.
    if upload.written != upload.declared_size {
        drop(upload.file);
        let _ = tokio::fs::remove_file(&upload.temp_path).await;
        return Err(format!(
            "Size mismatch: wrote {} but declared {}",
            upload.written, upload.declared_size
        ));
    }

    // Calculate actual hash
    let actual_hash = hex::encode(upload.hasher.finalize());

    // FAIL-CLOSED: data integrity guaranteed before any save operation.
    // If the browser provided an expected_hash, it must match the one computed
    // by the backend from the received bytes. Any mismatch indicates corruption or
    // tampering during transfer, and in that case the file
    // MUST NOT be saved to shared-folder/.
    //
    // If expected_hash is empty (browser legacy / fallback / error in calculation
    // on the sender side), saving proceeds with an explicit warning but
    // the upload is marked as "unverified" (telemetry field).
    let unverified = upload.expected_hash.is_empty();
    if !unverified && actual_hash != upload.expected_hash {
        // Telemetry: increment global atomic counter.
        state.hash_mismatch_total
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        // Close the file handle BEFORE deleting it (important on Windows).
        drop(upload.file);

        // Delete the orphaned temp file.
        if let Err(e) = tokio::fs::remove_file(&upload.temp_path).await {
            log::error!(
                "Hash mismatch: failed to delete temp file {:?}: {}",
                upload.temp_path, e
            );
        } else {
            log::info!("Temp file deleted after hash mismatch: {:?}", upload.temp_path);
        }

        log::error!(
            "❌ HASH MISMATCH for peer_id={}: expected={}, actual={}. File NOT saved. \
             Possible causes: WebRTC chunk corruption, bug in browser-side hash calculation, \
             or tampering. The recipient must retry.",
            peer_id, upload.expected_hash, actual_hash
        );

        return Err(format!(
            "Hash mismatch: the received file does not match the expected hash ({} vs {}). \
             File corrupted or tampered: NOT saved to guarantee data integrity.",
            upload.expected_hash, actual_hash
        ));
    }

    if unverified {
        log::warn!(
            "⚠️ Upload UNVERIFIED for peer_id={}: expected_hash empty, hash not verified.",
            peer_id
        );
        state.unverified_uploads_total
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    // Close the file before rename (important on Windows)
    drop(upload.file);

    // ✅ Re-validate filename before joining into shared_folder (defense in depth, S1)
    if !is_safe_filename(&upload.filename) {
        let _ = tokio::fs::remove_file(&upload.temp_path).await;
        return Err(format!("Unsafe filename: {}", upload.filename));
    }

    // Move temp file to shared folder with conflict resolution (Windows-safe).
    // On Windows, rename fails if the destination already exists, so we
    // generate a unique name using the {stem}_{counter}.{ext} pattern.
    let shared_folder = Path::new(&state.shared_folder);
    let mut dest_path = shared_folder.join(&upload.filename);
    let mut counter = 1;
    while dest_path.exists() {
        let stem = dest_path.file_stem().unwrap_or_default().to_str().unwrap_or("file");
        let ext = dest_path.extension().and_then(|e| e.to_str()).unwrap_or("");
        let new_name = if ext.is_empty() {
            format!("{}_{}", stem, counter)
        } else {
            format!("{}_{}.{}", stem, counter, ext)
        };
        dest_path = shared_folder.join(new_name);
        counter += 1;
    }
    tokio::fs::rename(&upload.temp_path, &dest_path).await
        .map_err(|e| format!("Failed to move file: {}", e))?;

    // Use the resolved filename (may differ from upload.filename if dedup occurred)
    let resolved_filename = dest_path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(&upload.filename)
        .to_string();

    // Persist to database
    let repo = FileRepository::new(state.db.clone());
    repo.save(&FileInfo {
        filename: resolved_filename.clone(),
        size: upload.written,
        hash: actual_hash.clone(),
        uploaded_at: chrono::Utc::now().to_rfc3339(),
    }).await.map_err(|e| e.to_string())?;

    // Update in-memory index
    let file_info = FileInfo {
        filename: resolved_filename,
        size: upload.written,
        hash: actual_hash.clone(),
        uploaded_at: chrono::Utc::now().to_rfc3339(),
    };
    state.file_index.lock().await.insert(file_info.hash.clone(), file_info);

    Ok(actual_hash)
}
