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
// Telemetria integrità dati: contatori globali per hash mismatch e upload unverified.
// Permette di diagnosticare bug nel calcolo hash lato browser o corruzione chunk
// durante il trasferimento.

use serde::Serialize;
use tauri::State;

/// Snapshot dei contatori di integrità dati
#[derive(Debug, Clone, Serialize)]
pub struct IntegrityMetrics {
    /// Numero totale di upload ricevuti il cui hash non corrispondeva a quello
    /// dichiarato dal browser. Un valore > 0 indica corruzione chunk o bug.
    pub hash_mismatch_total: u64,
    /// Numero totale di upload ricevuti SENZA expected_hash (browser legacy
    /// o errore nel calcolo hash lato mittente). Se alto, l'integrità è
    /// parzialmente bypassata.
    pub unverified_uploads_total: u64,
}

/// Comando Tauri: ottieni snapshot dei contatori di integrità.
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
