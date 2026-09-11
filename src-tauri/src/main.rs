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
// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod database;
mod server;
mod utils;
mod p2p;

use std::collections::HashMap;
use std::sync::Arc;
use tauri::Emitter;
use tauri::Listener;
use tauri::Manager;

// File metadata structure
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FileInfo {
    pub filename: String,
    pub size: u64,
    pub hash: String,
    pub uploaded_at: String,
}

// Global application state
pub struct AppState {
    pub file_index: Arc<tokio::sync::Mutex<HashMap<String, FileInfo>>>,
    pub db: Arc<tokio::sync::Mutex<rusqlite::Connection>>,
    pub shared_folder: String,
    pub temp_folder: String,
    pub config_folder: String,
    // HTTP server state
    pub server_running: tokio::sync::Mutex<bool>,
    pub server_shutdown_tx: tokio::sync::Mutex<Option<tokio::sync::oneshot::Sender<()>>>,
    pub server_handle: tokio::sync::Mutex<Option<tauri::async_runtime::JoinHandle<()>>>,
    // Download tracker
    pub download_tracker: commands::download_progress::DownloadTracker,
    // Upload tracker
    pub upload_tracker: commands::p2p::upload_progress::UploadTracker,
    // P2P state
    pub relay_manager: Arc<tokio::sync::Mutex<p2p::RelayManager>>,
    // Current PeerID (from PeerJS) for generating P2P-to-Web web links
    pub peer_id: tokio::sync::Mutex<Option<String>>,
    // Pending file hash for P2P-to-Web transfer (set when generating web link)
    pub pending_file_hash: tokio::sync::Mutex<Option<String>>,
    // Incoming uploads state (browser → app)
    pub incoming_uploads: Arc<tokio::sync::Mutex<HashMap<String, commands::p2p::upload_state::UploadState>>>,
    // FIX data integrity: global hash mismatch counter (atomic, no lock).
    // Incremented in finalize_incoming_file when the backend receives a file
    // whose hash does not match the one declared by the browser. A value > 0
    // indicates chunk corruption or a bug in the frontend hash calculation.
    pub hash_mismatch_total: std::sync::Arc<std::sync::atomic::AtomicU64>,
    // Counter for received uploads WITHOUT expected_hash (unverified): indicates
    // browser legacy or a bug in the sender-side hash calculation. If too high,
    // the integrity system is partially bypassed.
    pub unverified_uploads_total: std::sync::Arc<std::sync::atomic::AtomicU64>,
}

fn main() {
    // Initialize logging
    env_logger::init();

    // Load environment variables from .env (e.g. P2P_WEB_URL, TURN_AUTH_SECRET,
    // METERED_API_KEY, TURN_URLS). Search in order of priority:
    //   1) $CARGO_MANIFEST_DIR/.env (== src-tauri/.env): single source of truth
    //      for backend variables, valid in dev and release. Evaluated at compile-time.
    //   2) <exe_dir>/.env: next to peerino.exe for portable release build.
    //   3) CWD/.env: fallback for those launching from terminal with relative path.
    //   4) <exe_dir>/../.env: parent of the executable directory (es. src-tauri/).
    // `.ok()` on each attempt: no error must block startup if the file
    // is missing in all positions (the app works anyway, TURN will remain disabled).
    let app_dir_env = utils::get_app_dir();
    // CARGO_MANIFEST_DIR is the build directory (= src-tauri/). We add
    // a canonical candidate "src-tauri/.env" which is single source of truth for the
    // backend variables (P2P_WEB_URL, TURN, METERED_API_KEY, ...). In dev mode
    // (cargo run from src-tauri/) this path exists. In release (peerino.exe in
    // target/release/) does not exist, but candidates 1/2 cover it.
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let env_candidates = [
        format!("{}/.env", manifest_dir),  // src-tauri/.env (dev + canonical)
        format!("{}/.env", app_dir_env),   // executable/.env (release portable)
        ".env".to_string(),                 // CWD/.env
        format!("{}/../.env", app_dir_env), // executable/../.env
    ];
    let mut env_loaded = false;
    for candidate in &env_candidates {
        if std::path::Path::new(candidate).exists() {
            match dotenvy::from_filename(candidate) {
                Ok(_) => {
                    log::info!("📄 .env loaded from: {}", candidate);
                    env_loaded = true;
                    break;
                }
                Err(e) => {
                    log::warn!("⚠️ Found {} but parsing error: {}", candidate, e);
                }
            }
        }
    }
    if !env_loaded {
        log::info!("ℹ️ No .env file found (searched in: {:?}). System environment variables will be used as fallback.", env_candidates);
    }

    // Get the app directory (where the executable is located)
    let app_dir = utils::get_app_dir();
    let shared_folder = format!("{}/shared-folder", app_dir);
    let temp_folder = format!("{}/temp", app_dir);
    let config_folder = format!("{}/config", app_dir);
    
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_notification::init())
        .manage(AppState {
                    file_index: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
                    db: Arc::new(tokio::sync::Mutex::new(
                        rusqlite::Connection::open_in_memory().unwrap()
                    )),
                    // Use absolute paths based on the executable directory
                    // This guarantees that folders are created alongside peerino.exe
                    shared_folder,
                    temp_folder,
                    config_folder,
                    server_running: tokio::sync::Mutex::new(false),
                    server_shutdown_tx: tokio::sync::Mutex::new(None),
                    server_handle: tokio::sync::Mutex::new(None),
                    download_tracker: commands::download_progress::DownloadTracker::new(),
                    // Upload tracker
                    upload_tracker: commands::p2p::upload_progress::UploadTracker::new(),
                    // P2P state
                    relay_manager: Arc::new(tokio::sync::Mutex::new(
                        p2p::RelayManager::new(Default::default())
                    )),
                    // Current PeerID (updated by PeerJS on connect)
                    peer_id: tokio::sync::Mutex::new(None),
                    // File hash pending for P2P-to-Web transfer
                    pending_file_hash: tokio::sync::Mutex::new(None),
                    // Incoming uploads state (browser → app)
                    incoming_uploads: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
                    // Telemetria integrità dati
                    hash_mismatch_total: std::sync::Arc::new(
                        std::sync::atomic::AtomicU64::new(0)
                    ),
                    unverified_uploads_total: std::sync::Arc::new(
                        std::sync::atomic::AtomicU64::new(0)
                    ),
            })
        .invoke_handler(tauri::generate_handler![
         commands::register_file::register_file,
         commands::download_file::download_file,
         commands::download_progress::get_download_progress,
         commands::download_progress::cancel_download,
         commands::download_progress::get_download_metrics,
         commands::p2p::upload_progress::get_upload_progress,
         commands::p2p::upload_progress::cancel_upload,
         commands::p2p::upload_progress::get_upload_metrics,
         commands::list_files::list_files,
         commands::list_files::refresh_files,
         commands::network_info::get_network_info,
         commands::open_url::open_url,
         commands::start_server::start_http_server,
         commands::start_server::stop_http_server,
         commands::folder::open_shared_folder,
         commands::folder::check_shared_folder,
         // Local link generation
         commands::local_link::generate_local_link,
         // P2P commands (Phase 4 - Internet)
         commands::p2p::connect::connect_to_peer,
         commands::p2p::disconnect::disconnect_from_peer,
         commands::p2p::peers::list_peers,
         commands::p2p::generate_link::generate_public_link,
         commands::p2p::stream_file::stream_file,
         // P2P-to-Web link with PeerID (Internet)
         commands::p2p::generate_web_link::generate_web_link,
         commands::p2p::set_peer_id::set_peer_id,
          utils::peer_id::get_persistent_peer_id,
         commands::p2p::p2p_file_transfer::get_file_info,
         commands::p2p::p2p_file_transfer::read_file_chunk,
         commands::p2p::p2p_file_transfer::get_pending_file_hash,
         commands::p2p::p2p_file_transfer::clear_pending_file_hash,
         commands::p2p::create_inbox::create_inbox,
         // Local inbox (LAN only)
         commands::p2p::create_inbox_local::create_inbox_local,
         // Incremental upload (browser → app) - replaces save_incoming_file
         commands::p2p::init_incoming_upload::init_incoming_upload,
         commands::p2p::append_incoming_chunk::append_incoming_chunk,
         commands::p2p::finalize_incoming_file::finalize_incoming_file,
         // Telemetria integrità dati (P0: contatori hash mismatch / unverified)
         commands::hash_integrity::get_integrity_metrics,
         commands::turn_limits::get_turn_limits,
         commands::turn_limits::record_turn_rejection_cmd,
        ])
        .setup(|app| {
            // --- Resume handling from hibernation/suspension ---
            // In Tauri 2.x the resume event is emitted as a global listable event.
            // We register both known variants for robustness.
            let app_handle = app.handle().clone();
            for evt in ["tauri://resume", "tauri://resumed"] {
                let app_handle = app_handle.clone();
                app.listen(evt, move |_event| {
                    let h = app_handle.clone();
                    tauri::async_runtime::spawn(async move {
                        // On Windows the network may still be inactive: we wait 2s
                        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                        let _ = h.emit("system-resumed", ());
                    });
                });
            }

            // Create the necessary folders asynchronously
            let shared_folder = app.state::<AppState>().shared_folder.clone();
            let temp_folder = app.state::<AppState>().temp_folder.clone();
            let config_folder = app.state::<AppState>().config_folder.clone();
            let db = app.state::<AppState>().db.clone();
            let file_index = app.state::<AppState>().file_index.clone();
            
            tauri::async_runtime::spawn(async move {
                if let Err(e) = tokio::fs::create_dir_all(&shared_folder).await {
                    log::warn!("⚠️ Could not create shared folder: {}", e);
                }
                if let Err(e) = tokio::fs::create_dir_all(&temp_folder).await {
                    log::warn!("⚠️ Could not create temp folder: {}", e);
                }
                if let Err(e) = tokio::fs::create_dir_all(&config_folder).await {
                    log::warn!("⚠️ Could not create config folder: {}", e);
                }

                // Open the database after creating the config folder
                {
                    let mut db_guard = db.lock().await;
                    // Try to open the persistent database
                    let db_path = format!("{}/files.db", config_folder);
                    if let Ok(persistent_db) = rusqlite::Connection::open(&db_path) {
                        *db_guard = persistent_db;
                    }

                    // Imposta WAL mode
                    if let Err(e) = db_guard.pragma_update(None, "journal_mode", &"WAL") {
                        log::warn!("⚠️ Could not set WAL mode: {}", e);
                    }
                }

                // Initialize the files table
                let repo = database::files::FileRepository::new(db.clone());
                if let Err(e) = repo.init_table().await {
                    log::warn!("⚠️ Could not create files table: {}", e);
                }

                // Scan the shared-folder and add missing files to the database
                if let Err(e) = repo.scan_and_populate(&shared_folder).await {
                    log::warn!("⚠️ Could not scan shared folder: {}", e);
                }

                // Load existing files from the database into the in-memory index
                let files = repo.load_all().await;

                if let Ok(files) = files {
                    let mut index = file_index.lock().await;
                    for file in files {
                        index.insert(file.hash.clone(), file);
                    }
                    log::info!("📂 Loaded {} files from the persistent index", index.len());
                }
            });

            // Start the temp cleanup task
            // In Tauri 2.0, spawned tasks are automatically cancelled on close
            let temp_path = app.state::<AppState>().temp_folder.clone();
            let shared_folder = app.state::<AppState>().shared_folder.clone();
            tauri::async_runtime::spawn(async move {
                utils::temp_cleanup::start_cleanup_task(&temp_path, &shared_folder).await;
            });
            
            // Start the automatic relay cleanup task (expired links and inboxes)
            let relay_manager = app.state::<AppState>().relay_manager.clone();
            tauri::async_runtime::spawn(async move {
                loop {
                    tokio::time::sleep(tokio::time::Duration::from_secs(3600)).await;
                    relay_manager.lock().await.cleanup_expired().await;
                }
            });
            
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
