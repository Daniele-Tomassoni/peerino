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
use crate::server::http::{HttpServerState, DEFAULT_HTTP_PORT};
use crate::AppState;
use std::sync::Arc;
use tauri::State;

/// Starts the HTTP server on the default port
/// - The server runs in the background in a separate async task
/// - Serves files from shared-folder/ via streaming
/// - Listener binding happens before spawn: bind errors are returned
///   synchronously instead of being hidden inside the task
#[tauri::command]
pub async fn start_http_server(
    state: State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<String, String> {
    // Check if the server is already running
    {
        let running = state.server_running.lock().await;
        if *running {
            return Err("The HTTP server is already running".to_string());
        }
    }

    let shared_folder = state.shared_folder.clone();
    let server_running = state.server_running.clone();
    // Share the Arc, do not clone
    let file_index = state.file_index.clone();
    // Share the download tracker
    let download_tracker = state.download_tracker.active.clone();
    // Share the upload tracker
    let upload_tracker = state.upload_tracker.active.clone();
    // Share the relay manager
    let relay_manager = state.relay_manager.clone();

    let server_state = Arc::new(HttpServerState {
        shared_folder,
        file_index,
        download_tracker: crate::commands::download_progress::DownloadTracker {
            active: download_tracker,
            cancelled_flags: state.download_tracker.cancelled_flags.clone(),
            // FIX: reuse the same AppState telemetry counter
            cancellations_total: state.download_tracker.cancellations_total.clone(),
        },
        upload_tracker: crate::commands::p2p::upload_progress::UploadTracker {
            active: upload_tracker,
            cancelled_flags: state.upload_tracker.cancelled_flags.clone(),
            // FIX: reuse the same AppState telemetry counter
            cancellations_total: state.upload_tracker.cancellations_total.clone(),
        },
        relay_manager,
        db: state.db.clone(),
        app_handle: Some(app_handle),
    });

    // Bind the TCP listener BEFORE spawning so bind errors are returned synchronously.
    let port: u16 = std::env::var("HTTP_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_HTTP_PORT);
    let addr = format!("0.0.0.0:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .map_err(|e| format!("Failed to bind to port {}: {}", port, e))?;

    // Create shutdown channels
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

    // Save the shutdown channel in state
    {
        let mut tx = state.server_shutdown_tx.lock().await;
        *tx = Some(shutdown_tx);
    }

    // Start the server in a separate task
    let handle = tauri::async_runtime::spawn(async move {
        log::info!("HTTP server started on port {}...", port);

        // Use select! to handle both execution and shutdown
        tokio::select! {
            result = crate::server::http::start_server(server_state, listener) => {
                if let Err(e) = result {
                    *server_running.lock().await = false;
                    log::error!("Critical HTTP server error: {}", e);
                } else {
                    *server_running.lock().await = false;
                    log::info!("HTTP server terminated cleanly");
                }
            }
            _ = shutdown_rx => {
                log::info!("HTTP server stopped on request");
            }
        }
    });

    // Save the task handle
    {
        let mut handle_opt = state.server_handle.lock().await;
        *handle_opt = Some(handle);
    }

    // Update server state
    {
        let mut running = state.server_running.lock().await;
        *running = true;
    }

    Ok(format!("🌐 HTTP server started on port {}", port))
}

/// Stops the HTTP server (if running)
/// - Handles graceful shutdown
#[tauri::command]
pub async fn stop_http_server(state: State<'_, AppState>) -> Result<String, String> {
    // Check if the server is running
    {
        let running = state.server_running.lock().await;
        if !*running {
            return Err("The HTTP server is not running".to_string());
        }
    }

    // Send the shutdown signal.
    {
        let tx = state.server_shutdown_tx.lock().await.take();
        if let Some(tx) = tx {
            let _ = tx.send(());
        }
    }

    // Await the server task with a bounded timeout.
    let handle = state.server_handle.lock().await.take();
    if let Some(handle) = handle {
        let _ = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            handle,
        ).await;
    }

    // Update state
    {
        let mut running = state.server_running.lock().await;
        *running = false;
        let mut handle_opt = state.server_handle.lock().await;
        *handle_opt = None;
    }

    Ok("🛑 HTTP server stopped".to_string())
}

#[cfg(test)]
mod tests {
    // Tests require a mocked AppState
    // Will be tested with integration at a later time
}
