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
