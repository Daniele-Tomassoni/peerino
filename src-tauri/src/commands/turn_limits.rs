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
// Comandi Tauri per gestire il limite TURN e la telemetria associata.
// Esponono al frontend:
// - max_file_size (letto da env o default 100MB)
// - max_file_size_buffer (con margine 10% per overhead protocollo)
// - rejections_total (counter globale, atomicu64)

use crate::commands::{get_turn_max_file_size, get_turn_max_file_size_buffer};
use serde::Serialize;
use std::sync::atomic::{AtomicU64, Ordering};

/// Snapshot dei limiti TURN e telemetria
#[derive(Debug, Clone, Serialize)]
pub struct TurnLimits {
    /// Limite dichiarato all'utente (es. 100 MB).
    pub max_file_size: u64,
    /// Limite effettivo con margine overhead (es. 110 MB).
    pub max_file_size_buffer: u64,
    /// Quanti trasferimenti sono stati rifiutati per superamento limite.
    pub rejections_total: u64,
}

/// Counter globale per i rifiuti (atomicu64, no lock).
/// Viene incrementato sia dal backend (per upload inbox) sia letto
/// dal frontend per diagnostica.
static TURN_REJECTIONS: AtomicU64 = AtomicU64::new(0);

/// Incrementa il counter di rifiuti TURN. Chiamato dal backend Rust quando
/// rileva un file over-limit su connessione TURN (futuro: hook su
/// `init_incoming_upload` per validare la dimensione).
pub fn record_turn_rejection() {
    TURN_REJECTIONS.fetch_add(1, Ordering::Relaxed);
}

/// Comando Tauri: registra un rifiuto TURN dal frontend (es. blocco pre-stream).
/// Ritorna il counter aggiornato.
#[tauri::command]
pub async fn record_turn_rejection_cmd() -> Result<u64, String> {
    record_turn_rejection();
    Ok(TURN_REJECTIONS.load(Ordering::Relaxed))
}

/// Comando Tauri: ottieni i limiti TURN correnti e la telemetria.
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
        // Default 100 MB
        assert_eq!(get_turn_max_file_size(), 104_857_600);
        // Buffer 110 MB
        assert_eq!(get_turn_max_file_size_buffer(), 115_343_360);
    }
}
