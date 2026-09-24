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
// Tauri commands for managing the TURN limit and associated telemetry.
// Exposes to the frontend:
// - max_file_size (read from env or default 1KB)
// - max_file_size_buffer (with 10% margin for protocol overhead)
// - rejections_total (global counter, atomicu64)

use crate::commands::{get_turn_max_file_size, get_turn_max_file_size_buffer};
use serde::Serialize;
use std::sync::atomic::{AtomicU64, Ordering};

/// Snapshot of TURN limits and telemetry
#[derive(Debug, Clone, Serialize)]
pub struct TurnLimits {
    /// Diagnostic-only TURN limit (default 1 KB).
    pub max_file_size: u64,
    /// Effective limit with overhead margin.
    pub max_file_size_buffer: u64,
    /// How many transfers have been rejected for exceeding the limit.
    pub rejections_total: u64,
}

/// Global counter for rejections (atomicu64, no lock).
/// Incremented both by the backend (for inbox uploads) and read
/// by the frontend for diagnostics.
static TURN_REJECTIONS: AtomicU64 = AtomicU64::new(0);

/// Increment the TURN rejection counter. Called by the Rust backend when
/// an over-limit file is detected on a TURN connection (future: hook into
/// `init_incoming_upload` to validate size).
pub fn record_turn_rejection() {
    TURN_REJECTIONS.fetch_add(1, Ordering::Relaxed);
}

/// Tauri command: record a TURN rejection from the frontend (e.g. pre-stream block).
/// Returns the updated counter.
#[tauri::command]
pub async fn record_turn_rejection_cmd() -> Result<u64, String> {
    record_turn_rejection();
    Ok(TURN_REJECTIONS.load(Ordering::Relaxed))
}

/// Tauri command: get current TURN limits and telemetry.
#[tauri::command]
pub async fn get_turn_limits() -> Result<TurnLimits, String> {
    Ok(TurnLimits {
        max_file_size: get_turn_max_file_size(),
        max_file_size_buffer: get_turn_max_file_size_buffer(),
        rejections_total: TURN_REJECTIONS.load(Ordering::Relaxed),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_limits() {
        // Default 0 (no files via TURN).
        assert_eq!(get_turn_max_file_size(), 0);
        // Buffer = 0 + 0 = 0 bytes (+10% of 0)
        assert_eq!(get_turn_max_file_size_buffer(), 0);
    }
}
