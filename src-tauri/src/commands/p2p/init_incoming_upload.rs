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
// Initialize an incoming upload (browser → app)
// Creates a temp file and the upload state for incremental writes

use tauri::State;
use crate::AppState;
use crate::utils::is_safe_filename;
use crate::commands::get_turn_max_file_size;
use crate::commands::turn_limits::record_turn_rejection;
use super::upload_state::UploadState;

#[tauri::command]
pub async fn init_incoming_upload(
    state: State<'_, AppState>,
    peer_id: String,
    inbox_id: String,
    filename: String,
    size: u64,
    expected_hash: String,
    path: Option<String>,
) -> Result<(), String> {
    log::info!("📥 init_incoming_upload: peer_id={}, filename={}, size={}, path={:?}", peer_id, filename, size, path);

    // Authorize the upload against a currently valid inbox. The outer lock
    // serializes this check with inbox creation/cleanup in the application.
    let valid_inbox = {
        let relay = state.relay_manager.lock().await;
        relay.validate_inbox(&inbox_id).await
    };
    if !valid_inbox {
        return Err("Invalid or expired inbox".to_string());
    }

    // Validate filename to prevent path traversal (S1)
    if !is_safe_filename(&filename) {
        return Err(format!("Unsafe filename: {}", filename));
    }

    // Post-Phase 2: block uploads > TURN_MAX_FILE_SIZE when the connection requires TURN.
    // The path is optional: if not specified or LAN/STUN, no limit applies.
    let is_turn = path.as_deref().map(|p| p.eq_ignore_ascii_case("turn") || p.contains("relay")).unwrap_or(false);
    if is_turn && size > get_turn_max_file_size() {
        record_turn_rejection();
        log::warn!("TURN size limit: rejected upload of {} bytes (limit {}) for peer={}", size, get_turn_max_file_size(), peer_id);
        return Err(format!("TURN_SIZE_LIMIT|{}|{}", get_turn_max_file_size(), size));
    }

    let mut upload_state = UploadState::new(&state.temp_folder, filename, expected_hash)
        .await
        .map_err(|e| e.to_string())?;
    upload_state.declared_size = size;
    upload_state.max_allowed_size = size.saturating_add(1024);

    let mut uploads = state.incoming_uploads.lock().await;
    if uploads.contains_key(&peer_id) {
        return Err(format!("Upload already in progress for peer {}", peer_id));
    }
    uploads.insert(peer_id.clone(), upload_state);
    log::info!("✅ Upload state initialized for peer_id={}", peer_id);
    Ok(())
}
