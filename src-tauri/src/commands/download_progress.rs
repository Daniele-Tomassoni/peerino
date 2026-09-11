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

/// Download progress structure
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

/// Tracker for active downloads
#[derive(Clone)]
pub struct DownloadTracker {
    pub active: Arc<tokio::sync::Mutex<HashMap<String, DownloadProgress>>>,
    /// Cancellation status for HTTP downloads
    pub cancelled_flags: Arc<tokio::sync::Mutex<HashMap<String, Arc<AtomicBool>>>>,
    /// FIX: telemetry — total cancellation counter (download)
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

    /// FIX: get current metrics (for telemetry / debug)
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
    
    /// Cancel an in-progress download
    pub async fn cancel_download(&self, hash: &str) {
        // FIX: telemetry — increment counter only if the download existed
        // (avoid counting cancels on non-existent hashes / no-ops)
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

        // Set the cancellation flag
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
    
    /// Get all active downloads
    pub async fn get_all(&self) -> Vec<DownloadProgress> {
        let active = self.active.lock().await;
        active.values().cloned().collect()
    }
    
    /// Register a cancellation flag for a download
    /// FIX #3: reuse the existing flag for the same hash if it is NOT already
    /// cancelled, avoiding the clobbering that caused a race condition between
    /// re-registration and in-flight cancellation.
    pub async fn register_cancellation_flag(&self, hash: &str) -> Arc<AtomicBool> {
        let mut flags = self.cancelled_flags.lock().await;
        if let Some(existing) = flags.get(hash) {
            // If the existing flag is already true (cancelled), create a new one
            // reset to false. Otherwise reuse it to avoid losing state.
            if !existing.load(Ordering::SeqCst) {
                return existing.clone();
            }
            // Flag already cancelled: create a new one for the next download
        }
        let flag = Arc::new(AtomicBool::new(false));
        flags.insert(hash.to_string(), flag.clone());
        flag
    }
    
    /// Check if a download has been cancelled
    /// FIX #5: use try_lock to avoid blocking the async runtime on every chunk.
    /// If the lock is contended, return false (assume not cancelled): the loop
    /// will re-check at the next chunk. False negative is OK; false positive
    /// would block valid downloads.
    /// NOTE: this version is synchronous (non-async) to be called in
    /// hot loops without await. Call sites must be updated.
    pub fn is_cancelled(&self, hash: &str) -> bool {
        if let Ok(flags) = self.cancelled_flags.try_lock() {
            flags.get(hash).map(|f| f.load(Ordering::Relaxed)).unwrap_or(false)
        } else {
            false
        }
    }
}

/// Command to get download progress
#[tauri::command]
pub async fn get_download_progress(
    state: tauri::State<'_, crate::AppState>,
) -> Result<Vec<DownloadProgress>, String> {
    Ok(state.download_tracker.get_all().await)
}


/// Command to cancel a download
#[tauri::command]
pub async fn cancel_download(
    hash: String,
    state: tauri::State<'_, crate::AppState>,
) -> Result<String, String> {
    state.download_tracker.cancel_download(&hash).await;
    Ok(format!("Download cancelled: {}", hash))
}

/// FIX: telemetry command — get download metrics
#[tauri::command]
pub async fn get_download_metrics(
    state: tauri::State<'_, crate::AppState>,
) -> Result<DownloadMetrics, String> {
    Ok(state.download_tracker.get_metrics().await)
}