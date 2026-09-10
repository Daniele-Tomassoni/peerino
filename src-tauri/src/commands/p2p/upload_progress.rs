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
// Upload progress tracking for P2P and HTTP uploads
// Used for emitting progress events to the frontend

use serde::Serialize;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Struttura per il progresso dell'upload
#[derive(Debug, Clone, Serialize)]
pub struct UploadProgress {
    pub hash: String,
    pub filename: String,
    pub bytes_processed: u64,
    pub total_bytes: u64,
    pub progress: u32,
    /// Velocità di trasferimento in MB/s (calcolata su tempo trascorso)
    pub speed_mbps: f64,
    pub peer_id: String,
    pub cancelled: bool,
}

impl UploadProgress {
    pub fn new(hash: String, filename: String, bytes_processed: u64, total_bytes: u64, peer_id: String) -> Self {
        let progress = if total_bytes > 0 {
            (bytes_processed as f64 / total_bytes as f64 * 100.0).min(100.0).round() as u32
        } else {
            0
        };
        Self {
            hash,
            filename,
            bytes_processed,
            total_bytes,
            progress,
            speed_mbps: 0.0,
            peer_id,
            cancelled: false,
        }
    }
}

/// Tracker per gli upload attivi
#[derive(Clone)]
pub struct UploadTracker {
    pub active: Arc<tokio::sync::Mutex<HashMap<String, UploadProgress>>>,
    /// Flag di cancellazione per gli upload
    pub cancelled_flags: Arc<tokio::sync::Mutex<HashMap<String, Arc<AtomicBool>>>>,
    /// FIX: telemetria — contatore totale cancellazioni (upload)
    pub cancellations_total: Arc<std::sync::atomic::AtomicU64>,
}

/// FIX: telemetria globale upload counters
#[derive(Debug, Clone, serde::Serialize)]
pub struct UploadMetrics {
    pub cancellations_total: u64,
    pub active_uploads: usize,
}

impl UploadTracker {
    pub fn new() -> Self {
        Self {
            active: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
            cancelled_flags: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
            cancellations_total: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }

    /// FIX: ottieni metriche correnti (per telemetria / debug)
    pub async fn get_metrics(&self) -> UploadMetrics {
        let active = self.active.lock().await;
        UploadMetrics {
            cancellations_total: self.cancellations_total.load(Ordering::Relaxed),
            active_uploads: active.len(),
        }
    }

    /// Aggiorna il progresso di un upload
    pub async fn update_progress(
        &self,
        hash: &str,
        bytes_processed: u64,
        total: u64,
        filename: &str,
        peer_id: &str,
    ) {
        let mut active = self.active.lock().await;
        let progress = if total > 0 {
            (bytes_processed as f64 / total as f64 * 100.0).min(100.0).round() as u32
        } else {
            0
        };

        // Use unique key: peer_id + '-' + hash
        let key = format!("{}-{}", peer_id, hash);
        active.entry(key.clone()).and_modify(|e| {
            e.bytes_processed = bytes_processed;
            e.total_bytes = total;
            e.progress = progress;
        }).or_insert(UploadProgress {
            hash: hash.to_string(),
            filename: filename.to_string(),
            bytes_processed,
            total_bytes: total,
            progress,
            speed_mbps: 0.0,
            peer_id: peer_id.to_string(),
            cancelled: false,
        });
    }

    /// Rimuovi un upload completato
    pub async fn remove_upload(&self, hash: &str) {
        let mut active = self.active.lock().await;
        // Entries are stored under key "{peer_id}-{hash}", so we must scan
        // by the progress.hash field rather than the key directly.
        let keys_to_remove: Vec<String> = active
            .iter()
            .filter(|(_, progress)| progress.hash == hash)
            .map(|(key, _)| key.clone())
            .collect();
        for key in keys_to_remove {
            active.remove(&key);
        }
        let mut flags = self.cancelled_flags.lock().await;
        flags.remove(hash);
    }

    /// Ottieni tutti gli upload attivi
    pub async fn get_all(&self) -> Vec<UploadProgress> {
        let active = self.active.lock().await;
        active.values().cloned().collect()
    }

    /// Annulla un upload in corso
    /// FIX: telemetria — incrementa counter solo se l'upload esisteva e non era già cancellato
    pub async fn cancel_upload(&self, hash: &str) {
        let mut should_count = false;
        // Imposta lo stato di cancellazione nel progresso
        // Nota: gli upload sono memorizzati con chiave "peer_id-hash",
        // ma il frontend invia solo l'hash. Cerchiamo per campo hash.
        {
            let mut active = self.active.lock().await;
            for (_, progress) in active.iter_mut() {
                if progress.hash == hash && !progress.cancelled {
                    progress.cancelled = true;
                    should_count = true;
                }
            }
        }

        // Imposta il flag di cancellazione
        {
            let flags = self.cancelled_flags.lock().await;
            if let Some(flag) = flags.get(hash) {
                flag.store(true, Ordering::SeqCst);
            }
        }

        if should_count {
            self.cancellations_total.fetch_add(1, Ordering::Relaxed);
            log::info!("Upload cancelled (total: {}): {}",
                self.cancellations_total.load(Ordering::Relaxed), hash);
        }
    }

    /// Registra un flag di cancellazione per un upload
    pub async fn register_cancellation_flag(&self, hash: &str) -> Arc<AtomicBool> {
        let flag = Arc::new(AtomicBool::new(false));
        let mut flags = self.cancelled_flags.lock().await;
        flags.insert(hash.to_string(), flag.clone());
        flag
    }

    /// Controlla se un upload è stato annullato
    /// FIX #5: versione sincrona con try_lock per evitare contesa nei loop hot path
    pub fn is_cancelled(&self, hash: &str) -> bool {
        if let Ok(flags) = self.cancelled_flags.try_lock() {
            flags.get(hash).map(|f| f.load(Ordering::Relaxed)).unwrap_or(false)
        } else {
            false
        }
    }
}

/// Comando per ottenere il progresso degli upload
///
/// FIX: short-circuit when there are no active uploads to avoid allocating /
/// serializing an empty Vec on every frontend poll (the UI calls this every
/// 500ms via `setInterval`).
#[tauri::command]
pub async fn get_upload_progress(
    state: tauri::State<'_, crate::AppState>,
) -> Result<Vec<UploadProgress>, String> {
    let active = state.upload_tracker.active.lock().await;
    if active.is_empty() {
        // No allocation, no clone, no serialization work on the idle path.
        return Ok(Vec::new());
    }
    drop(active);
    Ok(state.upload_tracker.get_all().await)
}

/// Comando per annullare un upload
#[tauri::command]
pub async fn cancel_upload(
    hash: String,
    state: tauri::State<'_, crate::AppState>,
) -> Result<String, String> {
    state.upload_tracker.cancel_upload(&hash).await;
    // FIX #4: NON chiamare download_tracker.cancel_download qui.
    // Il frontend già invoca cancel_download separatamente quando necessario.
    // Chiamare entrambi causa race condition (TOCTOU sul mutex active).
    // Se il caller vuole cancellare anche download correlati, deve farlo esplicitamente.
    Ok(format!("Upload annullato: {}", hash))
}

/// FIX: comando telemetria — ottieni metriche upload
#[tauri::command]
pub async fn get_upload_metrics(
    state: tauri::State<'_, crate::AppState>,
) -> Result<UploadMetrics, String> {
    Ok(state.upload_tracker.get_metrics().await)
}
