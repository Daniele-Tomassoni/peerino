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
    filename: String,
    size: u64,
    expected_hash: String,
    path: Option<String>,
) -> Result<(), String> {
    log::info!("📥 init_incoming_upload: peer_id={}, filename={}, size={}, path={:?}", peer_id, filename, size, path);

    // Validate filename to prevent path traversal (S1)
    if !is_safe_filename(&filename) {
        return Err(format!("Nome file non sicuro: {}", filename));
    }

    // Post-Fase 2: blocca upload > TURN_MAX_FILE_SIZE quando la connessione richiede TURN.
    // Il path è opzionale: se non specificato o LAN/STUN, nessun limite.
    let is_turn = path.as_deref().map(|p| p.eq_ignore_ascii_case("turn") || p.contains("relay")).unwrap_or(false);
    if is_turn && size > get_turn_max_file_size() {
        record_turn_rejection();
        log::warn!("TURN size limit: rejected upload of {} bytes (limit {}) for peer={}", size, get_turn_max_file_size(), peer_id);
        return Err(format!("TURN_SIZE_LIMIT|{}|{}", get_turn_max_file_size(), size));
    }

    let upload_state = UploadState::new(&state.temp_folder, filename, expected_hash)
        .await
        .map_err(|e| e.to_string())?;

    let mut uploads = state.incoming_uploads.lock().await;
    uploads.insert(peer_id.clone(), upload_state);
    log::info!("✅ Upload state initialized for peer_id={}", peer_id);
    Ok(())
}
