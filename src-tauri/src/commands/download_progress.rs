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
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// Struttura per il progresso del download
#[derive(Debug, Clone, serde::Serialize)]
pub struct DownloadProgress {
    pub hash: String,
    pub filename: String,
    pub total_bytes: u64,
    pub downloaded_bytes: u64,
    pub speed_mbps: f64,
    pub peer_ip: String,
    pub progress: u32,
    pub cancelled: bool,
}

/// Tracker per i download attivi
#[derive(Clone)]
pub struct DownloadTracker {
    pub active: Arc<tokio::sync::Mutex<HashMap<String, DownloadProgress>>>,
    /// Stato di cancellazione per i download HTTP
    pub cancelled_flags: Arc<tokio::sync::Mutex<HashMap<String, Arc<AtomicBool>>>>,
    /// FIX: telemetria — contatore totale cancellazioni (download)
    pub cancellations_total: Arc<std::sync::atomic::AtomicU64>,
}

/// FIX: telemetria globale download counters (snapshot via `get_metrics`)
#[derive(Debug, Clone, serde::Serialize)]
pub struct DownloadMetrics {
    pub cancellations_total: u64,
    pub active_downloads: usize,
}

impl DownloadTracker {
    pub fn new() -> Self {
        Self {
            active: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
            cancelled_flags: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
            cancellations_total: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }

    /// FIX: ottieni metriche correnti (per telemetria / debug)
    pub async fn get_metrics(&self) -> DownloadMetrics {
        let active = self.active.lock().await;
        DownloadMetrics {
            cancellations_total: self.cancellations_total.load(Ordering::Relaxed),
            active_downloads: active.len(),
        }
    }
    
    /// Aggiorna il progresso di un download
    pub async fn update_progress(
        &self,
        hash: &str,
        downloaded: u64,
        total: u64,
        speed: f64,
        peer_ip: &str,
        filename: &str,
    ) {
        let mut active = self.active.lock().await;
        let progress = if total > 0 {
            (downloaded as f64 / total as f64 * 100.0).min(100.0).round() as u32
        } else {
            0
        };

        active.entry(hash.to_string()).and_modify(|e| {
            e.downloaded_bytes = downloaded;
            e.total_bytes = total;
            e.speed_mbps = speed;
            e.progress = progress;
        }).or_insert(DownloadProgress {
            hash: hash.to_string(),
            filename: filename.to_string(),
            total_bytes: total,
            downloaded_bytes: downloaded,
            speed_mbps: speed,
            peer_ip: peer_ip.to_string(),
            progress,
            cancelled: false,
        });
    }
    
    /// Rimuovi un download completato
    pub async fn remove_download(&self, hash: &str) {
        let mut active = self.active.lock().await;
        active.remove(hash);
        let mut flags = self.cancelled_flags.lock().await;
        flags.remove(hash);
    }
    
    /// Annulla un download in corso
    pub async fn cancel_download(&self, hash: &str) {
        // FIX: telemetria — incrementa counter solo se il download esisteva
        // (evita di conteggiare cancel su hash inesistenti / no-op)
        let mut should_count = false;
        {
            let mut active = self.active.lock().await;
            if let Some(progress) = active.get_mut(hash) {
                if !progress.cancelled {
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
            log::info!("Download cancelled (total: {}): {}",
                self.cancellations_total.load(Ordering::Relaxed), hash);
        }
    }
    
    /// Ottieni tutti i download attivi
    pub async fn get_all(&self) -> Vec<DownloadProgress> {
        let active = self.active.lock().await;
        active.values().cloned().collect()
    }
    
    /// Registra un flag di cancellazione per un download
    /// FIX #3: riusa il flag esistente per lo stesso hash se NON è già stato
    /// cancellato, evitando il clobbering che causava race condition tra
    /// re-registrazione e cancellazione in volo.
    pub async fn register_cancellation_flag(&self, hash: &str) -> Arc<AtomicBool> {
        let mut flags = self.cancelled_flags.lock().await;
        if let Some(existing) = flags.get(hash) {
            // Se il flag esistente è già true (cancellato), creane uno nuovo
            // resettato a false. Altrimenti riusalo per non perdere lo stato.
            if !existing.load(Ordering::SeqCst) {
                return existing.clone();
            }
            // Flag già cancellato: creane uno nuovo per il prossimo download
        }
        let flag = Arc::new(AtomicBool::new(false));
        flags.insert(hash.to_string(), flag.clone());
        flag
    }
    
    /// Controlla se un download è stato annullato
    /// FIX #5: usa try_lock per non bloccare il runtime async su ogni chunk.
    /// Se il lock è conteso, ritorna false (assumi non cancellato): il loop
    /// ricontrollerà al prossimo chunk. False negative è OK; false positive
    /// bloccherebbe download validi.
    /// NOTA: questa versione è sincrona (non async) per essere chiamata nei
    /// loop hot path senza await. I call site devono essere aggiornati.
    pub fn is_cancelled(&self, hash: &str) -> bool {
        if let Ok(flags) = self.cancelled_flags.try_lock() {
            flags.get(hash).map(|f| f.load(Ordering::Relaxed)).unwrap_or(false)
        } else {
            false
        }
    }
}

/// Comando per ottenere il progresso dei download
#[tauri::command]
pub async fn get_download_progress(
    state: tauri::State<'_, crate::AppState>,
) -> Result<Vec<DownloadProgress>, String> {
    Ok(state.download_tracker.get_all().await)
}


/// Comando per annullare un download
#[tauri::command]
pub async fn cancel_download(
    hash: String,
    state: tauri::State<'_, crate::AppState>,
) -> Result<String, String> {
    state.download_tracker.cancel_download(&hash).await;
    Ok(format!("Download annullato: {}", hash))
}

/// FIX: comando telemetria — ottieni metriche download
#[tauri::command]
pub async fn get_download_metrics(
    state: tauri::State<'_, crate::AppState>,
) -> Result<DownloadMetrics, String> {
    Ok(state.download_tracker.get_metrics().await)
}