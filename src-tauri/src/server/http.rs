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
use crate::FileInfo;
use crate::commands::download_progress::DownloadTracker;
use crate::p2p::RelayManager;
use crate::utils::is_safe_filename;
use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use chrono;
use hex;
use rusqlite::Connection;
use serde_json::json;
use sha2::{Digest, Sha256};
use tokio::io::AsyncWriteExt;
use crate::database::files::FileRepository;
use futures_util::stream::{Stream, StreamExt};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::fs::File;
use tokio_util::io::ReaderStream;
use tokio_util::bytes::Bytes;
use tower_http::cors::{Any, CorsLayer};
use tower_http::timeout::TimeoutLayer;
use tauri::Emitter;

/// Porta HTTP predefinita per il server
pub const DEFAULT_HTTP_PORT: u16 = 3000;

/// Stato condiviso per il server HTTP
#[allow(dead_code)]
pub struct HttpServerState {
    pub shared_folder: String,
    pub file_index: Arc<tokio::sync::Mutex<HashMap<String, FileInfo>>>,
    pub download_tracker: DownloadTracker,
    pub upload_tracker: crate::commands::p2p::upload_progress::UploadTracker,
    pub relay_manager: Arc<tokio::sync::Mutex<RelayManager>>,
    pub db: Arc<tokio::sync::Mutex<Connection>>,
    /// AppHandle for emitting Tauri events (upload/download progress)
    pub app_handle: Option<tauri::AppHandle>,
}

/// Wrapper stream that tracks download progress
pub struct ProgressTrackingStream<S> {
    inner: S,
    hash: String,
    filename: String,
    total_bytes: u64,
    downloaded_bytes: u64,
    start_time: Instant,
    tracker: DownloadTracker,
    peer_ip: String,
    app_handle: Option<tauri::AppHandle>,
    /// Cancellation flag (checked synchronously in poll_next)
    cancelled_flag: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl<S> ProgressTrackingStream<S> {
    pub fn new(
        inner: S,
        hash: String,
        filename: String,
        total_bytes: u64,
        tracker: DownloadTracker,
        peer_ip: String,
        app_handle: Option<tauri::AppHandle>,
        cancelled_flag: std::sync::Arc<std::sync::atomic::AtomicBool>,
    ) -> Self {
        Self {
            inner,
            hash,
            filename,
            total_bytes,
            downloaded_bytes: 0,
            start_time: Instant::now(),
            tracker,
            peer_ip,
            app_handle,
            cancelled_flag,
        }
    }
}

impl<S> Stream for ProgressTrackingStream<S>
where
    S: Stream<Item = Result<Bytes, std::io::Error>> + Unpin,
{
    type Item = Result<Bytes, std::io::Error>;

    fn poll_next(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Option<Self::Item>> {
        // Check cancellation flag before processing each chunk
        if self.cancelled_flag.load(std::sync::atomic::Ordering::SeqCst) {
            log::info!("Download cancelled: {}", self.hash);
            let _ = self.tracker.remove_download(&self.hash);
            return std::task::Poll::Ready(None);
        }

        match self.inner.poll_next_unpin(cx) {
            std::task::Poll::Ready(Some(Ok(bytes))) => {
                self.downloaded_bytes += bytes.len() as u64;

                // Calculate speed in MB/s
                let elapsed_secs = self.start_time.elapsed().as_secs_f64();
                let speed_mbps = if elapsed_secs > 0.0 {
                    (self.downloaded_bytes as f64 / (1024.0 * 1024.0)) / elapsed_secs
                } else {
                    0.0
                };

                // Update progress in tracker
                let _ = self.tracker.update_progress(
                    &self.hash,
                    self.downloaded_bytes,
                    self.total_bytes,
                    speed_mbps,
                    &self.peer_ip,
                    &self.filename,
                );

                // Emit Tauri event for real-time progress updates
                // This is an UPLOAD (app sends file), so emit "upload-progress"
                if let Some(ref handle) = self.app_handle {
                    let progress = if self.total_bytes > 0 {
                        (self.downloaded_bytes as f64 / self.total_bytes as f64 * 100.0).min(100.0).round() as u32
                    } else {
                        0
                    };
                    // Use peer_ip as peer_id for unique identification
                    let peer_id = format!("http-{}", self.peer_ip);
                    let _ = handle.emit("upload-progress", crate::commands::p2p::upload_progress::UploadProgress {
                        hash: self.hash.clone(),
                        filename: self.filename.clone(),
                        bytes_processed: self.downloaded_bytes,
                        total_bytes: self.total_bytes,
                        progress,
                        speed_mbps,
                        peer_id,
                        cancelled: false,
                    });
                }

                std::task::Poll::Ready(Some(Ok(bytes)))
            }
            std::task::Poll::Ready(Some(Err(e))) => {
                std::task::Poll::Ready(Some(Err(e)))
            }
            std::task::Poll::Ready(None) => {
                // Download completed, remove from tracker
                let _ = self.tracker.remove_download(&self.hash);
                std::task::Poll::Ready(None)
            }
            std::task::Poll::Pending => std::task::Poll::Pending,
        }
    }
}


/// Handler per elencare tutti i file disponibili (per test e Cloudflare Tunnel)
/// Scansiona sempre la shared-folder per rilevare nuovi file
async fn list_files_handler(
    State(state): State<Arc<HttpServerState>>,
) -> Result<Response, (StatusCode, String)> {
    let shared_folder = state.shared_folder.clone();
    let db = state.db.clone();
    let file_index = state.file_index.clone();
    
    // Load all files from the database (no per-request scan — P1)
    let repo = FileRepository::new(db);
    let files = repo.load_all().await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    
    // Update the in-memory index with the complete set
    {
        let mut index = file_index.lock().await;
        index.clear();
        for file in &files {
            index.insert(file.hash.clone(), file.clone());
        }
    }
    
    // Filter files that don't exist on disk
    let files: Vec<FileInfo> = files
        .into_iter()
        .filter(|f| {
            let file_path = PathBuf::from(&shared_folder).join(&f.filename);
            file_path.exists()
        })
        .collect();
    
    // Sort by uploaded_at descending
    let mut files = files;
    files.sort_by(|a, b| b.uploaded_at.cmp(&a.uploaded_at));
    
    Ok(Json(files).into_response())
}

/// Handler for file download via public link (relay)
async fn relay_download_handler(
    Path(link_id): Path<String>,
    State(state): State<Arc<HttpServerState>>,
) -> Result<Response, (StatusCode, String)> {
    // Validate and consume the link
    let file_hash = {
        let relay = state.relay_manager.lock().await;
        relay.validate_and_consume_link(&link_id).await
    };

    let file_hash = file_hash.ok_or((StatusCode::NOT_FOUND, "Invalid or expired link".to_string()))?;

    stream_file_response(&state, file_hash, "relay").await
}

/// Handler per il download di un file
async fn download_file_handler(
    Path(hash): Path<String>,
    State(state): State<Arc<HttpServerState>>,
) -> Result<Response, (StatusCode, String)> {
    stream_file_response(&state, hash, "local").await
}

/// Builds the streaming response for a file given its hash.
/// Shared logic between `download_file_handler` (LAN) and
/// `relay_download_handler` (public link) to avoid duplication.
async fn stream_file_response(
    state: &Arc<HttpServerState>,
    hash: String,
    peer_ip: &str,
) -> Result<Response, (StatusCode, String)> {
    // Trova il file nell'indice
    let file_info = {
        let file_index = state.file_index.lock().await;
        file_index
            .get(&hash)
            .cloned()
            .ok_or((StatusCode::NOT_FOUND, "File non trovato".to_string()))?
    };

    // Verifica che il filename sia sicuro (no path traversal)
    if !is_safe_filename(&file_info.filename) {
        return Err((StatusCode::BAD_REQUEST, "Nome file non valido".to_string()));
    }

    // Costruisci il percorso del file
    let file_path = PathBuf::from(&state.shared_folder).join(&file_info.filename);

    // Verifica che il file esista
    if !file_path.exists() {
        return Err((StatusCode::NOT_FOUND, "File non trovato sul disco".to_string()));
    }

    // Apri il file
    let file = match File::open(&file_path).await {
        Ok(f) => f,
        Err(e) => return Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    };

    // Register cancellation flag for this download
    let cancelled_flag = state.download_tracker.register_cancellation_flag(&hash).await;

    // Usa ReaderStream per lo streaming con tracciamento progresso
    // Note: ReaderStream uses default 8KB buffer; for 64KB we would need to wrap with a custom buffer
    let stream = ReaderStream::new(file);
    let body = Body::from_stream(ProgressTrackingStream::new(
        stream,
        hash,
        file_info.filename.clone(),
        file_info.size,
        state.download_tracker.clone(),
        peer_ip.to_string(),
        state.app_handle.clone(),
        cancelled_flag,
    ));

    // Costruisci la risposta
    let mut response_headers = axum::http::HeaderMap::new();
    response_headers.insert(
        header::CONTENT_TYPE,
        "application/octet-stream".parse().unwrap(),
    );
    response_headers.insert(
        header::CONTENT_DISPOSITION,
        format!("attachment; filename=\"{}\"", file_info.filename)
            .parse()
            .unwrap(),
    );
    Ok((StatusCode::OK, response_headers, body).into_response())
}

/// Handler for serving the receiver web page for P2P-to-Web.
/// Available on LAN and via Cloudflare Tunnel (reverse box):
/// the recipient opens the link and the page connects P2P to the sender (no relay server).
async fn receiver_page_handler() -> Response {
    let html = include_str!("../../../src/ui/web-receiver.html");
    let mut headers = axum::http::HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, "text/html; charset=utf-8".parse().unwrap());
    (StatusCode::OK, headers, html).into_response()
}

/// Handler per ricevere un file caricato nella Scatola di Consegna Inversa (Locale)
/// Lo streaming del body evita di caricare l'intero file in memoria (regola progetto: no Vec<u8>).
async fn inbox_upload_handler(
    Path(inbox_id): Path<String>,
    Query(params): Query<HashMap<String, String>>,
    State(state): State<Arc<HttpServerState>>,
    headers: axum::http::HeaderMap,
    body: Body,
) -> Result<Response, (StatusCode, String)> {
    // Valida che l'inbox esista
    {
        let relay = state.relay_manager.lock().await;
        if relay.get_inbox(&inbox_id).await.is_none() {
            return Err((StatusCode::NOT_FOUND, "Invalid or expired reverse delivery box".to_string()));
        }
    }

    // Estrae e valida il nome file (no path traversal)
    let filename = params
        .get("filename")
        .cloned()
        .filter(|f| is_safe_filename(f))
        .ok_or((StatusCode::BAD_REQUEST, "Nome file mancante o non valido".to_string()))?;

    // Estrae l'hash del file dalla query string (passato dal frontend).
    // Viene usato come chiave per il flag di cancellazione, in modo che il
    // pulsante "X" del frontend (che chiama cancel_upload con solo l'hash)
    // possa trovare e attivare il flag.
    // FIX #1: usare SEMPRE `inbox-{inbox_id}` come fallback (NON `inbox-{filename}`)
    // per garantire che la chiave di cancellazione corrisponda a quella usata
    // dal pulsante ✕ del frontend quando msg.hash non è disponibile.
    // Lato frontend la chiave è msg.hash || downloadId dove downloadId = msg.hash
    // || `${conn.peer}-${msg.filename}`. Per coerenza usiamo un prefisso deterministico.
    let file_hash = params
        .get("hash")
        .cloned()
        .unwrap_or_else(|| format!("inbox-{}", inbox_id));

    // Gestione conflitti di nome nella shared-folder
    let shared_folder = state.shared_folder.clone();
    let mut target_path = PathBuf::from(&shared_folder).join(&filename);
    let mut counter = 1;
    while target_path.exists() {
        let stem = target_path.file_stem().unwrap_or_default().to_str().unwrap_or("file");
        let ext = target_path.extension().and_then(|e| e.to_str()).unwrap_or("");
        let new_name = if ext.is_empty() {
            format!("{}_{}", stem, counter)
        } else {
            format!("{}_{}.{}", stem, counter, ext)
        };
        target_path = PathBuf::from(&shared_folder).join(new_name);
        counter += 1;
    }

    // Streaming del body verso il file con calcolo SHA-256 in tempo reale
    let mut target_file = File::create(&target_path).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let mut stream = body.into_data_stream();
    let mut hasher = Sha256::new();
    let mut total_size: u64 = 0;
    
    // Usa l'hash del file (passato dal frontend) come chiave di tracking,
    // cosi il pulsante "X" del frontend (che chiama cancel_upload con solo
    // l'hash) puo trovare e attivare il flag di cancellazione.
    let download_hash = file_hash.clone();
    let start_time = Instant::now();

    // Content-Length inviato dal browser (XHR con File): permette di mostrare
    // una percentuale reale invece di progresso indeterminato.
    let total_expected: u64 = headers
        .get(header::CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);

    // Register cancellation flag for this inbox download
    let cancelled_flag = state.download_tracker.register_cancellation_flag(&download_hash).await;

    let mut cancelled = false;
    while let Some(chunk) = stream.next().await {
        // Check cancellation flag
        if cancelled_flag.load(std::sync::atomic::Ordering::SeqCst) {
            log::info!("Inbox download cancelled: {}", download_hash);
            cancelled = true;
            break;
        }

        let chunk = chunk.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        hasher.update(&chunk);
        target_file.write_all(&chunk).await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        total_size += chunk.len() as u64;

        // Update download tracker (app is receiving file via inbox)
        let elapsed_secs = start_time.elapsed().as_secs_f64();
        let speed_mbps = if elapsed_secs > 0.0 {
            (total_size as f64 / (1024.0 * 1024.0)) / elapsed_secs
        } else {
            0.0
        };
        let _ = state.download_tracker.update_progress(
            &download_hash,
            total_size,
            total_expected,
            speed_mbps,
            "inbox", // peer_ip identifier
            &filename,
        ).await;

        // Emit Tauri event for real-time download progress (percentuale reale
        // grazie a Content-Length; 0% indeterminato solo se l'header manca)
        if let Some(ref handle) = state.app_handle {
            let pct = if total_expected > 0 {
                ((total_size.min(total_expected) as f64 / total_expected as f64) * 100.0) as u32
            } else {
                0
            };
            let _ = handle.emit("download-progress", crate::commands::download_progress::DownloadProgress {
                hash: download_hash.clone(),
                filename: filename.clone(),
                total_bytes: total_expected,
                downloaded_bytes: total_size,
                speed_mbps,
                peer_ip: "inbox".to_string(),
                progress: pct,
                cancelled: false,
            });
        }
    }

    // If cancelled, clean up partial file and remove from tracker
    if cancelled {
        let _ = target_file.flush().await;
        let _ = tokio::fs::remove_file(&target_path).await;
        let _ = state.download_tracker.remove_download(&download_hash).await;
        return Err((StatusCode::REQUEST_TIMEOUT, "Download annullato".to_string()));
    }

    target_file.flush().await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let hash = hex::encode(hasher.finalize());
    let final_filename = target_path.file_name().unwrap().to_str().unwrap().to_string();

    // FIX P0: verify hash integrity for inbox HTTP uploads.
    // The browser computes SHA-256 before sending and passes it as ?hash=...
    // If the computed hash doesn't match, the file is corrupt or manipulated:
    // reject it and clean up the temp file (same policy as P2P inbox).
    let unverified_http = file_hash.is_empty();
    if !unverified_http && hash != file_hash {
        log::error!(
            "❌ HASH MISMATCH (inbox HTTP): expected={}, actual={}. File NON salvato.",
            file_hash, hash
        );
        drop(target_file);
        if let Err(e) = tokio::fs::remove_file(&target_path).await {
            log::warn!("Impossibile cancellare file inbox dopo hash mismatch: {}", e);
        }
        return Err((StatusCode::BAD_REQUEST, format!(
            "Hash mismatch: il file ricevuto non corrisponde all'hash dichiarato ({} vs {}). NON salvato.",
            file_hash, hash
        )));
    }
    if unverified_http {
        log::warn!("⚠️ Inbox HTTP UNVERIFIED: expected_hash vuoto, hash non verificato.");
    }

    // Registra nel file_index (in memoria, visibile nella UI) e nel database (persistenza)
    let file_info = FileInfo {
        filename: final_filename.clone(),
        size: total_size,
        hash: hash.clone(),
        uploaded_at: chrono::Utc::now().to_rfc3339(),
    };
    {
        let mut file_index = state.file_index.lock().await;
        file_index.insert(hash.clone(), file_info.clone());
        let db = state.db.clone();
        tauri::async_runtime::spawn(async move {
            let repo = FileRepository::new(db);
            if let Err(e) = repo.save(&file_info).await {
                log::error!("Errore salvataggio database inbox: {}", e);
            }
        });
    }

    // Emetti l'evento finale al 100%: senza questo la UI non conclude mai
    // la riga di download e il file ricevuto non risulta "completato".
    if let Some(ref handle) = state.app_handle {
        let _ = handle.emit("download-progress", crate::commands::download_progress::DownloadProgress {
            hash: download_hash.clone(),
            filename: final_filename.clone(),
            total_bytes: total_size,
            downloaded_bytes: total_size,
            speed_mbps: 0.0,
            peer_ip: "inbox".to_string(),
            progress: 100,
            cancelled: false,
        });
    }

    // Update final download progress (100%) and remove from tracker
    let _ = state.download_tracker.update_progress(
        &download_hash,
        total_size,
        total_size,
        0.0, // speed not needed for final update
        "inbox",
        &final_filename,
    ).await;
    
    // Remove from tracker after a short delay to allow UI to show 100%
    let download_tracker = state.download_tracker.clone();
    let download_hash_clone = download_hash.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        download_tracker.remove_download(&download_hash_clone).await;
    });

    log::info!("File ricevuto via inbox locale: {} (hash: {})", final_filename, hash);
    Ok(Json(json!({"status": "ok", "hash": hash, "filename": final_filename})).into_response())
}

/// Crea il router per il server HTTP
pub fn create_router(state: Arc<HttpServerState>) -> Router {
    Router::new()
        .route("/files", get(list_files_handler))
        .route("/get/:link_id", get(relay_download_handler))
        .route("/download/:hash", get(download_file_handler))
        .route("/inbox/:inbox_id", get(inbox_page_handler).post(inbox_upload_handler))
        // Probe di raggiungibilità LAN: usata dalla pagina receiver per capire
        // se il ricevente è sulla stessa rete del mittente (fallback HTTP diretto)
        .route("/ping", get(ping_handler))
        // Receiver web page for P2P-to-Web (Generate Web Link / Reverse Box)
        // Served at root "/" to work with Netlify
        .route("/", get(receiver_page_handler))
        .route("/receiver", get(receiver_page_handler))
        // Timeout per connessioni lente (5 minuti)
        // CORS per accesso da altri dispositivi
        .layer(TimeoutLayer::new(Duration::from_secs(300)))
        .layer(CorsLayer::new().allow_origin(Any))
        .with_state(state)
}

/// Local Reverse Delivery Box upload page (pure HTTP).
/// The browser sends the file via a direct POST to /inbox/{id}?filename=...
/// No PeerJS signaling and no TURN: works even on an isolated LAN.
/// The {INBOX_ID} placeholder is replaced by the handler (no format!,
/// to avoid conflicts with CSS/JavaScript curly braces).
const INBOX_UPLOAD_PAGE: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>Peerino - Send a file</title>
<style>
  :root { --bg0:#070B14; --panel:rgba(20,30,55,.72); --border:rgba(56,189,248,.18);
          --text:#e2e8f0; --dim:#94a3b8; --accent:#38bdf8; --green:#22c55e; --red:#ef4444; }
  * { box-sizing:border-box; margin:0; padding:0; }
  body { font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,sans-serif;
         background:radial-gradient(circle at 50% 35%, rgba(34,211,238,.12), transparent 55%),
                    linear-gradient(160deg,var(--bg0),#16213E);
         min-height:100vh; color:var(--text); display:flex; align-items:center; justify-content:center; padding:20px; }
  .card { background:var(--panel); border:1px solid var(--border); border-radius:12px;
          padding:28px; width:100%; max-width:460px; backdrop-filter:blur(8px); }
  h1 { font-size:1.15rem; margin-bottom:4px; }
  p.sub { color:var(--dim); font-size:.85rem; margin-bottom:20px; line-height:1.45; }
  .drop { border:2px dashed var(--border); border-radius:10px; padding:34px 16px; text-align:center;
          cursor:pointer; transition:border-color .2s, background .2s; }
  .drop:hover, .drop.over { border-color:var(--accent); background:rgba(56,189,248,.06); }
  .drop .icon { font-size:2rem; display:block; margin-bottom:8px; }
  .drop span { color:var(--dim); font-size:.88rem; }
  .fname { margin-top:12px; font-size:.9rem; word-break:break-all; min-height:1.3em; }
  button { margin-top:16px; width:100%; padding:11px; border:none; border-radius:8px;
           background:var(--accent); color:#04121f; font-weight:600; font-size:.95rem; cursor:pointer; }
  button:disabled { opacity:.45; cursor:not-allowed; }
  .bar-wrap { margin-top:18px; height:8px; background:rgba(148,163,184,.15);
              border-radius:4px; overflow:hidden; display:none; }
  .bar { height:100%; width:0%; background:var(--green); transition:width .2s ease; }
  .status { margin-top:10px; font-size:.85rem; color:var(--dim); min-height:1.3em;
            word-break:break-all; line-height:1.4; }
  .status.ok { color:var(--green); } .status.err { color:var(--red); }
</style>
</head>
<body>
<div class="card">
  <h1>Send a file to Peerino</h1>
  <p class="sub">Direct transfer on the local network: the file goes from your device
     to Peerino without passing through any external server.</p>
  <div class="drop" id="drop">
    <span class="icon">&#128206;</span>
    <span>Drag a file here or click to select it</span>
  </div>
  <input type="file" id="file" hidden />
  <div class="fname" id="fname"></div>
  <button id="send" disabled>Send file</button>
  <!-- FIX #7: Cancel button to stop the active upload -->
  <button id="cancel" style="display:none;background:var(--red);">Cancel upload</button>
  <div class="bar-wrap" id="barwrap"><div class="bar" id="bar"></div></div>
  <div class="status" id="status"></div>
</div>
<script>
(function () {
  var INBOX_ID = "{INBOX_ID}";
  var drop = document.getElementById('drop');
  var fileInput = document.getElementById('file');
  var fnameEl = document.getElementById('fname');
  var sendBtn = document.getElementById('send');
  var barwrap = document.getElementById('barwrap');
  var bar = document.getElementById('bar');
  var statusEl = document.getElementById('status');
  var selectedFile = null;

  function fmtSize(bytes) {
    if (!bytes) return '0 B';
    var u = ['B', 'KB', 'MB', 'GB', 'TB'], i = Math.floor(Math.log(bytes) / Math.log(1024));
    return (bytes / Math.pow(1024, i)).toFixed(1) + ' ' + u[i];
  }
  function pick(f) {
    selectedFile = f || null;
    fnameEl.textContent = f ? f.name + ' (' + fmtSize(f.size) + ')' : '';
    sendBtn.disabled = !f;
    statusEl.textContent = '';
    statusEl.className = 'status';
    bar.style.width = '0%';
    barwrap.style.display = 'none';
  }

  drop.addEventListener('click', function () { fileInput.click(); });
  fileInput.addEventListener('change', function () { pick(fileInput.files[0]); });
  ['dragover', 'dragenter'].forEach(function (ev) {
    drop.addEventListener(ev, function (e) { e.preventDefault(); drop.classList.add('over'); });
  });
  ['dragleave', 'drop'].forEach(function (ev) {
    drop.addEventListener(ev, function (e) { e.preventDefault(); drop.classList.remove('over'); });
  });
  drop.addEventListener('drop', function (e) {
    var f = e.dataTransfer && e.dataTransfer.files && e.dataTransfer.files[0];
    if (f) pick(f);
  });

  var cancelBtn = document.getElementById('cancel');
  sendBtn.addEventListener('click', function () {
    if (!selectedFile) return;
    // Compute the SHA-256 hash of the file BEFORE uploading, to use it as
    // a deletion key consistent with the backend (?hash=... param)
    var fileHash = '';
    try {
      // Use SubtleCrypto if available (requires HTTPS or localhost)
      var reader = new FileReader();
      reader.onload = function (e) {
        var buf = e.target.result;
        if (window.crypto && window.crypto.subtle) {
          window.crypto.subtle.digest('SHA-256', buf).then(function (hashBuf) {
            var hashArr = new Uint8Array(hashBuf);
            var hex = '';
            for (var i = 0; i < hashArr.length; i++) {
              hex += hashArr[i].toString(16).padStart(2, '0');
            }
            fileHash = hex;
            startUpload(fileHash);
          });
        } else {
          startUpload('');
        }
      };
      reader.readAsArrayBuffer(selectedFile);
      return; // the upload starts inside the callback
    } catch (err) {
      startUpload('');
      return;
    }

    function startUpload(hash) {
      var url = '/inbox/' + encodeURIComponent(INBOX_ID) +
                '?filename=' + encodeURIComponent(selectedFile.name) +
                (hash ? '&hash=' + encodeURIComponent(hash) : '');
      var xhr = new XMLHttpRequest();
      var t0 = Date.now();

      sendBtn.disabled = true;
      sendBtn.style.display = 'none';
      cancelBtn.style.display = 'block';
      barwrap.style.display = 'block';
      statusEl.className = 'status';
      statusEl.textContent = 'Uploading...';

      // FIX #7: cancellation handling via xhr.abort()
      cancelBtn.onclick = function () {
        if (xhr.readyState !== XMLHttpRequest.DONE) {
          xhr.abort();
          statusEl.className = 'status err';
          statusEl.textContent = '⏹ Upload cancelled.';
          sendBtn.disabled = false;
          sendBtn.style.display = 'block';
          cancelBtn.style.display = 'none';
        }
      };

      xhr.upload.addEventListener('progress', function (e) {
        if (e.lengthComputable) {
          var pct = Math.round(e.loaded / e.total * 100);
          var secs = (Date.now() - t0) / 1000;
          var mbps = secs > 0 ? (e.loaded / 1048576) / secs : 0;
          bar.style.width = pct + '%';
          statusEl.textContent = 'Uploading... ' + pct + '% - ' + mbps.toFixed(1) + ' MB/s';
        }
      });
      xhr.addEventListener('load', function () {
        sendBtn.style.display = 'block';
        cancelBtn.style.display = 'none';
        if (xhr.status === 200) {
          bar.style.width = '100%';
          statusEl.className = 'status ok';
          statusEl.textContent = 'File sent successfully to Peerino.';
        } else {
          statusEl.className = 'status err';
          statusEl.textContent = 'Error ' + xhr.status + ': ' + (xhr.responseText || 'upload failed');
          sendBtn.disabled = false;
        }
      });
      xhr.addEventListener('error', function () {
        sendBtn.style.display = 'block';
        cancelBtn.style.display = 'none';
        if (xhr.readyState === XMLHttpRequest.UNSENT) return; // abort already handled
        statusEl.className = 'status err';
        statusEl.textContent = 'Network error during upload.';
        sendBtn.disabled = false;
      });
      xhr.addEventListener('abort', function () {
        sendBtn.style.display = 'block';
        cancelBtn.style.display = 'none';
      });

      // Request body = raw file (server-side streaming),
      // the filename travels in the query string as expected by the handler.
      xhr.open('POST', url);
      xhr.send(selectedFile);
    }
  });
})();
</script>
</body>
</html>"#;

/// Handler for /ping GET - minimal page for the LAN reachability probe.
/// The receiver page opens this endpoint in a background tab: here we send
/// postMessage('pong') to the opener (cross-origin, allowed) so that whoever
/// opened the link knows they are on the same network as the sender and can
/// use direct HTTP transfer instead of WebRTC.
async fn ping_handler() -> Response {
    let html = r#"<!DOCTYPE html>
<html><body style="background:#070B14;color:#94a3b8;font-family:sans-serif;text-align:center;padding-top:40px">
<p>Peerino LAN probe...</p>
<script>
  try { if (window.opener) window.opener.postMessage('pong', '*'); } catch (e) {}
  setTimeout(function () { try { window.close(); } catch (e) {} }, 300);
</script>
</body></html>"#;
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
        html,
    )
        .into_response()
}

/// Handler for /inbox/:inbox_id GET - serves the pure HTTP upload page.
/// The ID is validated (404 if expired/non-existent), consistent with the POST.
async fn inbox_page_handler(
    Path(inbox_id): Path<String>,
    State(state): State<Arc<HttpServerState>>,
) -> Result<Response, (StatusCode, String)> {
    {
        let relay = state.relay_manager.lock().await;
        if relay.get_inbox(&inbox_id).await.is_none() {
            return Err((StatusCode::NOT_FOUND, "Invalid or expired reverse delivery box".to_string()));
        }
    }
    let html = INBOX_UPLOAD_PAGE.replace("{INBOX_ID}", &inbox_id);
    let mut headers = axum::http::HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, "text/html; charset=utf-8".parse().unwrap());
    Ok((StatusCode::OK, headers, html).into_response())
}

/// Starts the HTTP server with an already-bound listener.
/// Binding is done by the caller (start_http_server) so that bind errors
/// di bind vengano restituiti sincronamente invece di essere nascosti nel task.
pub async fn start_server(state: Arc<HttpServerState>, listener: tokio::net::TcpListener) -> Result<(), String> {
    let addr = listener.local_addr()
        .map_err(|e| format!("Failed to read l'indirizzo del listener: {}", e))?;
    
    log::info!("🌐 HTTP server started at http://{}", addr);
    
    axum::serve(listener, create_router(state))
        .await
        .map_err(|e| e.to_string())
}
