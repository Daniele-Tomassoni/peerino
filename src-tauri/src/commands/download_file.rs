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
use crate::AppState;
use crate::utils::is_safe_filename;
use std::path::{Path, PathBuf};
use std::time::Instant;
use tauri::State;
use tauri::Emitter;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const BUFFER_SIZE: usize = 64 * 1024; // 64KB

/// Validates the destination path provided by the frontend before writing.
/// Prevents arbitrary disk writes (path traversal / accidental overwrites)
/// by requiring an absolute path whose parent folder exists and whose
/// filename is safe.
fn validate_target_path(target_path: &str) -> Result<PathBuf, String> {
    let path = Path::new(target_path);
    if !path.is_absolute() {
        return Err("The destination path must be absolute".to_string());
    }
    let parent = path
        .parent()
        .ok_or_else(|| "Invalid destination path".to_string())?;
    if !parent.exists() {
        return Err("The destination folder does not exist".to_string());
    }
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| "Invalid destination filename".to_string())?;
    if !is_safe_filename(file_name) {
        return Err("Invalid destination filename".to_string());
    }
    Ok(path.to_path_buf())
}

/// Download a file from the shared folder
/// - Looks up the file in the index by hash
/// - Copies the file via streaming to target_path (validated)
/// - Tracks download progress in real time
/// - Returns the original filename
#[tauri::command]
pub async fn download_file(
    hash: String,
    target_path: String,
    state: State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<String, String> {
    // Look up the file in the index
    let file_info = {
        let file_index = state.file_index.lock().await;
        file_index
            .get(&hash)
            .cloned()
            .ok_or_else(|| "File not found in index".to_string())?
    };

    // ✅ Source filename validation (defense in depth)
    if !is_safe_filename(&file_info.filename) {
        return Err("Invalid filename".to_string());
    }

    // Build the source file path
    let shared_folder = state.shared_folder.clone();
    let source_path = Path::new(&shared_folder).join(&file_info.filename);

    // Verify the file exists
    if !source_path.exists() {
        return Err(format!("File not found on disk: {}", file_info.filename));
    }

    // ✅ Validate the destination path before opening the file
    let target_path = validate_target_path(&target_path)?;

    // Register cancellation flag for this download
    let cancelled_flag = state.download_tracker.register_cancellation_flag(&hash).await;

    // Copy the file via streaming
    let mut source_file = File::open(&source_path)
        .await
        .map_err(|e| e.to_string())?;
    let mut target_file = File::create(&target_path)
        .await
        .map_err(|e| e.to_string())?;

    let mut buffer = vec![0u8; BUFFER_SIZE];
    let total_size = file_info.size;
    let mut downloaded_bytes: u64 = 0;
    let start_time = Instant::now();

    loop {
        // Check cancellation flag
        if cancelled_flag.load(std::sync::atomic::Ordering::SeqCst) {
            log::info!("Download cancelled: {}", hash);
            let _ = target_file.flush().await;
            let _ = tokio::fs::remove_file(&target_path).await;
            let _ = state.download_tracker.remove_download(&hash).await;
            return Err("Download cancelled".to_string());
        }

        let bytes_read = source_file
            .read(&mut buffer)
            .await
            .map_err(|e| e.to_string())?;

        if bytes_read == 0 {
            break;
        }

        // ✅ write_all guarantees complete write
        target_file
            .write_all(&buffer[..bytes_read])
            .await
            .map_err(|e| e.to_string())?;

        downloaded_bytes += bytes_read as u64;

        // Update download progress tracker
        let elapsed_secs = start_time.elapsed().as_secs_f64();
        let speed_mbps = if elapsed_secs > 0.0 {
            (downloaded_bytes as f64 / (1024.0 * 1024.0)) / elapsed_secs
        } else {
            0.0
        };
        let _ = state.download_tracker.update_progress(
            &hash,
            downloaded_bytes,
            total_size,
            speed_mbps,
            "local",
            &file_info.filename,
        ).await;

        // Emit Tauri event for real-time download progress
        let progress = if total_size > 0 {
            (downloaded_bytes as f64 / total_size as f64 * 100.0).min(100.0).round() as u32
        } else {
            0
        };
        let _ = app_handle.emit("download-progress", crate::commands::download_progress::DownloadProgress {
            hash: hash.clone(),
            filename: file_info.filename.clone(),
            total_bytes: total_size,
            downloaded_bytes,
            speed_mbps,
            peer_ip: "local".to_string(),
            progress,
            cancelled: false,
        });
    }

    // Sync to ensure data is written to disk
    target_file.sync_all().await.map_err(|e| e.to_string())?;

    // Update final progress (100%) and remove from tracker
    let _ = state.download_tracker.update_progress(
        &hash,
        total_size,
        total_size,
        0.0,
        "local",
        &file_info.filename,
    ).await;
    let _ = state.download_tracker.remove_download(&hash).await;

    log::info!("File downloaded: {} (hash: {})", file_info.filename, hash);

    Ok(file_info.filename)
}

#[cfg(test)]
mod tests {
    // Tests will run with integration
    // Tauri 2.0 does not support mock_state in tests
}
