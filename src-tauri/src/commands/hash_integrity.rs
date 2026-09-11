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
// Data integrity telemetry: global counters for hash mismatch and unverified uploads.
// Allows diagnosing bugs in browser-side hash calculation or chunk corruption
// during transfer.

use serde::Serialize;
use tauri::State;

/// Snapshot of data integrity counters
#[derive(Debug, Clone, Serialize)]
pub struct IntegrityMetrics {
    /// Total number of received uploads whose hash did not match the one
    /// declared by the browser. A value > 0 indicates chunk corruption or a bug.
    pub hash_mismatch_total: u64,
    /// Total number of received uploads WITHOUT expected_hash (browser legacy
    /// or sender-side hash calculation error). If high, integrity is
    /// partially bypassed.
    pub unverified_uploads_total: u64,
}

/// Tauri command: get a snapshot of the integrity counters.
#[tauri::command]
pub async fn get_integrity_metrics(
    state: State<'_, crate::AppState>,
) -> Result<IntegrityMetrics, String> {
    Ok(IntegrityMetrics {
        hash_mismatch_total: state
            .hash_mismatch_total
            .load(std::sync::atomic::Ordering::Relaxed),
        unverified_uploads_total: state
            .unverified_uploads_total
            .load(std::sync::atomic::Ordering::Relaxed),
    })
}
