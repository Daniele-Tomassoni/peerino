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

// Struttura per i metadati del file
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FileInfo {
    pub filename: String,
    pub size: u64,
    pub hash: String,
    pub uploaded_at: String,
}

// Stato globale dell'applicazione
pub struct AppState {
    pub file_index: Arc<tokio::sync::Mutex<HashMap<String, FileInfo>>>,
    pub db: Arc<tokio::sync::Mutex<rusqlite::Connection>>,
    pub shared_folder: String,
    pub temp_folder: String,
    pub config_folder: String,
    // Stato del server HTTP
    pub server_running: tokio::sync::Mutex<bool>,
    pub server_shutdown_tx: tokio::sync::Mutex<Option<tokio::sync::oneshot::Sender<()>>>,
    pub server_handle: tokio::sync::Mutex<Option<tauri::async_runtime::JoinHandle<()>>>,
    // Tracker per download
    pub download_tracker: commands::download_progress::DownloadTracker,
    // Tracker per upload
    pub upload_tracker: commands::p2p::upload_progress::UploadTracker,
    // P2P state
    pub relay_manager: Arc<tokio::sync::Mutex<p2p::RelayManager>>,
    // PeerID corrente (da PeerJS) per la generazione di link web P2P-to-Web
    pub peer_id: tokio::sync::Mutex<Option<String>>,
    // File hash pending for P2P-to-Web transfer (set when generating web link)
    pub pending_file_hash: tokio::sync::Mutex<Option<String>>,
    // Incoming uploads state (browser → app)
    pub incoming_uploads: Arc<tokio::sync::Mutex<HashMap<String, commands::p2p::upload_state::UploadState>>>,
    // FIX integrità dati: contatore globale hash mismatch (atomic, no lock).
    // Incrementato in finalize_incoming_file quando il backend riceve un file
    // il cui hash non corrisponde a quello dichiarato dal browser. Un valore > 0
    // indica corruzione chunk o bug nel calcolo hash lato frontend.
    pub hash_mismatch_total: std::sync::Arc<std::sync::atomic::AtomicU64>,
    // Contatore upload ricevuti SENZA expected_hash (unverified): indica
    // browser legacy o bug nel calcolo hash lato mittente. Se troppo alto,
    // il sistema di integrità è parzialmente bypassato.
    pub unverified_uploads_total: std::sync::Arc<std::sync::atomic::AtomicU64>,
}

fn main() {
    // Inizializza il logging
    env_logger::init();

    // Carica variabili d'ambiente da .env (es. P2P_WEB_URL, TURN_AUTH_SECRET,
    // METERED_API_KEY, TURN_URLS). Cerca in ordine di priorità:
    //   1) $CARGO_MANIFEST_DIR/.env (== src-tauri/.env): single source of truth
    //      per le variabili backend, valido in dev e release. Valutato a compile-time.
    //   2) <exe_dir>/.env: accanto a peerino.exe per build release portable.
    //   3) CWD/.env: fallback per chi lancia da terminale con path relativo.
    //   4) <exe_dir>/../.env: parent della directory eseguibile (es. src-tauri/).
    // `.ok()` su ogni tentativo: nessun errore deve bloccare l'avvio se il file
    // manca in tutte le posizioni (l'app funziona comunque, TURN resterà disabilitato).
    let app_dir_env = utils::get_app_dir();
    // CARGO_MANIFEST_DIR è la directory di compilazione (= src-tauri/). Aggiungiamo
    // un candidato canonico "src-tauri/.env" che è single source of truth per le
    // variabili backend (P2P_WEB_URL, TURN, METERED_API_KEY, ...). In devmode
    // (cargo run da src-tauri/) questo path esiste. In release (peerino.exe in
    // target/release/) non esiste, ma i candidati 1/2 lo coprono.
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let env_candidates = [
        format!("{}/.env", manifest_dir),  // src-tauri/.env (dev + canonical)
        format!("{}/.env", app_dir_env),   // eseguibile/.env (release portable)
        ".env".to_string(),                 // CWD/.env
        format!("{}/../.env", app_dir_env), // eseguibile/../.env
    ];
    let mut env_loaded = false;
    for candidate in &env_candidates {
        if std::path::Path::new(candidate).exists() {
            match dotenvy::from_filename(candidate) {
                Ok(_) => {
                    log::info!("📄 .env caricato da: {}", candidate);
                    env_loaded = true;
                    break;
                }
                Err(e) => {
                    log::warn!("⚠️ Trovato {} ma errore di parsing: {}", candidate, e);
                }
            }
        }
    }
    if !env_loaded {
        log::info!("ℹ️ Nessun file .env trovato (cercato in: {:?}). Le variabili d'ambiente di sistema saranno usate come fallback.", env_candidates);
    }

    // Ottieni la directory dell'app (dove si trova l'eseguibile)
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
                    // Usa percorsi assoluti basati sulla directory dell'eseguibile
                    // Questo garantisce che le cartelle vengano create accanto a peerino.exe
                    shared_folder,
                    temp_folder,
                    config_folder,
                    server_running: tokio::sync::Mutex::new(false),
                    server_shutdown_tx: tokio::sync::Mutex::new(None),
                    server_handle: tokio::sync::Mutex::new(None),
                    download_tracker: commands::download_progress::DownloadTracker::new(),
                    // Tracker per upload
                    upload_tracker: commands::p2p::upload_progress::UploadTracker::new(),
                    // P2P state
                    relay_manager: Arc::new(tokio::sync::Mutex::new(
                        p2p::RelayManager::new(Default::default())
                    )),
                    // PeerID corrente (aggiornato da PeerJS al connect)
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
         // P2P-to-Web link con PeerID (Internet)
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
            // --- Gestione ripresa da ibernazione/sospensione ---
            // In Tauri 2.x l'evento di resume è emesso come evento globale ascoltabile.
            // Registriamo entrambe le varianti note per robustezza.
            let app_handle = app.handle().clone();
            for evt in ["tauri://resume", "tauri://resumed"] {
                let app_handle = app_handle.clone();
                app.listen(evt, move |_event| {
                    let h = app_handle.clone();
                    tauri::async_runtime::spawn(async move {
                        // Su Windows la rete può essere ancora inattiva: attendiamo 2s
                        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                        let _ = h.emit("system-resumed", ());
                    });
                });
            }

            // Crea le cartelle necessarie in modo asincrono
            let shared_folder = app.state::<AppState>().shared_folder.clone();
            let temp_folder = app.state::<AppState>().temp_folder.clone();
            let config_folder = app.state::<AppState>().config_folder.clone();
            let db = app.state::<AppState>().db.clone();
            let file_index = app.state::<AppState>().file_index.clone();
            
            tauri::async_runtime::spawn(async move {
                if let Err(e) = tokio::fs::create_dir_all(&shared_folder).await {
                    eprintln!("⚠️ Could not create shared folder: {}", e);
                }
                if let Err(e) = tokio::fs::create_dir_all(&temp_folder).await {
                    eprintln!("⚠️ Could not create temp folder: {}", e);
                }
                if let Err(e) = tokio::fs::create_dir_all(&config_folder).await {
                    eprintln!("⚠️ Could not create config folder: {}", e);
                }
                
                // Apri il database dopo aver creato la cartella config
                {
                    let mut db_guard = db.lock().await;
                    // Prova ad aprire il database persistente
                    let db_path = format!("{}/files.db", config_folder);
                    if let Ok(persistent_db) = rusqlite::Connection::open(&db_path) {
                        *db_guard = persistent_db;
                    }
                    
                    // Imposta WAL mode
                    if let Err(e) = db_guard.pragma_update(None, "journal_mode", &"WAL") {
                        eprintln!("⚠️ Could not set WAL mode: {}", e);
                    }
                }
                
                // Inizializza la tabella files
                let repo = database::files::FileRepository::new(db.clone());
                if let Err(e) = repo.init_table().await {
                    eprintln!("⚠️ Could not create files table: {}", e);
                }
                
                // Scansiona la shared-folder e aggiunge i file mancanti al database
                if let Err(e) = repo.scan_and_populate(&shared_folder).await {
                    eprintln!("⚠️ Could not scan shared folder: {}", e);
                }
                
                // Carica i file esistenti dal database nell'indice in memoria
                let files = repo.load_all().await;
                
                if let Ok(files) = files {
                    let mut index = file_index.lock().await;
                    for file in files {
                        index.insert(file.hash.clone(), file);
                    }
                    log::info!("📂 Caricati {} file dall'indice persistente", index.len());
                }
            });

            // Avvia il task di pulizia temporanea
            // In Tauri 2.0, i task spawnati vengono cancellati automaticamente alla chiusura
            let temp_path = app.state::<AppState>().temp_folder.clone();
            let shared_folder = app.state::<AppState>().shared_folder.clone();
            tauri::async_runtime::spawn(async move {
                utils::temp_cleanup::start_cleanup_task(&temp_path, &shared_folder).await;
            });
            
            // Avvia il task di pulizia automatica del relay (link e inbox scaduti)
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
