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
use crate::server::http::{HttpServerState, DEFAULT_HTTP_PORT};
use crate::AppState;
use std::sync::Arc;
use tauri::State;

/// Avvia il server HTTP sulla porta predefinita
/// - Il server gira in background in un task asincrono separato
/// - Serve i file dalla shared-folder/ in streaming
/// - Il binding del listener avviene prima dello spawn: gli errori di bind
///   vengono restituiti sincronamente invece di essere nascosti nel task
#[tauri::command]
pub async fn start_http_server(
    state: State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<String, String> {
    // Verifica se il server è già in esecuzione
    {
        let running = state.server_running.lock().await;
        if *running {
            return Err("Il server HTTP è già in esecuzione".to_string());
        }
    }

    let shared_folder = state.shared_folder.clone();
    // Condividiamo l'Arc, non cloniamo
    let file_index = state.file_index.clone();
    // Condividiamo il download tracker
    let download_tracker = state.download_tracker.active.clone();
    // Condividiamo l'upload tracker
    let upload_tracker = state.upload_tracker.active.clone();
    // Condividiamo il relay manager
    let relay_manager = state.relay_manager.clone();

    let server_state = Arc::new(HttpServerState {
        shared_folder,
        file_index,
        download_tracker: crate::commands::download_progress::DownloadTracker {
            active: download_tracker,
            cancelled_flags: state.download_tracker.cancelled_flags.clone(),
            // FIX: riusa lo stesso contatore telemetria dell'AppState
            cancellations_total: state.download_tracker.cancellations_total.clone(),
        },
        upload_tracker: crate::commands::p2p::upload_progress::UploadTracker {
            active: upload_tracker,
            cancelled_flags: state.upload_tracker.cancelled_flags.clone(),
            // FIX: riusa lo stesso contatore telemetria dell'AppState
            cancellations_total: state.upload_tracker.cancellations_total.clone(),
        },
        relay_manager,
        db: state.db.clone(),
        app_handle: Some(app_handle),
    });

    // Bind the TCP listener BEFORE spawning so bind errors are returned synchronously
    let addr = format!("0.0.0.0:{}", DEFAULT_HTTP_PORT);
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .map_err(|e| format!("Impossibile bindare alla porta {}: {}", DEFAULT_HTTP_PORT, e))?;

    // Crea canali per shutdown
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

    // Salva il canale di shutdown nello stato
    {
        let mut tx = state.server_shutdown_tx.lock().await;
        *tx = Some(shutdown_tx);
    }

    // Avvia il server in un task separato
    let handle = tauri::async_runtime::spawn(async move {
        log::info!("Server HTTP avviato sulla porta {}...", DEFAULT_HTTP_PORT);

        // Usa select! per gestire sia l'esecuzione che lo shutdown
        tokio::select! {
            result = crate::server::http::start_server(server_state, listener) => {
                if let Err(e) = result {
                    log::error!("Errore critico server HTTP: {}", e);
                } else {
                    log::info!("Server HTTP terminato correttamente");
                }
            }
            _ = shutdown_rx => {
                log::info!("Server HTTP fermato su richiesta");
            }
        }
    });

    // Salva l'handle del task
    {
        let mut handle_opt = state.server_handle.lock().await;
        *handle_opt = Some(handle);
    }

    // Aggiorna lo stato del server
    {
        let mut running = state.server_running.lock().await;
        *running = true;
    }

    Ok(format!("🌐 Server HTTP avviato sulla porta {}", DEFAULT_HTTP_PORT))
}

/// Ferma il server HTTP (se in esecuzione)
/// - Gestisce lo shutdown graceful
#[tauri::command]
pub async fn stop_http_server(state: State<'_, AppState>) -> Result<String, String> {
    // Verifica se il server è in esecuzione
    {
        let running = state.server_running.lock().await;
        if !*running {
            return Err("Il server HTTP non è in esecuzione".to_string());
        }
    }

    // Invia il segnale di shutdown
    {
        let tx = state.server_shutdown_tx.lock().await.take();
        if let Some(tx) = tx {
            let _ = tx.send(());
        }
    }

    // Aggiorna lo stato
    {
        let mut running = state.server_running.lock().await;
        *running = false;
        let mut handle_opt = state.server_handle.lock().await;
        *handle_opt = None;
    }

    Ok("🛑 Server HTTP fermato".to_string())
}

#[cfg(test)]
mod tests {
    // I test richiedono un AppState mockato
    // Verranno testati con integrazione in un secondo momento
}
