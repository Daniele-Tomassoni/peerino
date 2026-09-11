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
use crate::{AppState, FileInfo};
use crate::database::files::FileRepository;
use sha2::{Digest, Sha256};
use std::path::Path;
use tauri::State;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const MAX_FILES: usize = 1000;
const BUFFER_SIZE: usize = 64 * 1024; // 64KB

/// Registers a file in the shared folder
/// - Receives the absolute file path
/// - Opens the file via streaming with tokio::fs
/// - Computes SHA-256 in streaming during the copy
/// - Copies the file into shared-folder/ handling name conflicts
/// - Inserts metadata into a local index and the database
/// - Returns the computed hash
#[tauri::command]
pub async fn register_file(
    file_path: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    log::info!("register_file called with path: {}", file_path);

    let source_path = Path::new(&file_path);

    // Verify that the file exists
    if !source_path.exists() {
        log::error!("File not found: {}", file_path);
        return Err(format!("File not found: {}", file_path));
    }

    // Get the file name
    let filename = source_path
        .file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "unknown".to_string());

    log::info!("Extracted file name: {}", filename);

    // Handle name conflicts
    let shared_folder = state.shared_folder.clone();
    let mut target_path = Path::new(&shared_folder).join(&filename);
    let mut counter = 1;
    while target_path.exists() {
        let stem = target_path.file_stem().unwrap_or_default().to_str().unwrap_or("file");
        let ext = target_path.extension().and_then(|e| e.to_str()).unwrap_or("");
        let new_name = if ext.is_empty() {
            format!("{}_{}", stem, counter)
        } else {
            format!("{}_{}.{}", stem, counter, ext)
        };
        target_path = Path::new(&shared_folder).join(new_name);
        counter += 1;
    }

    log::info!("Target path: {:?}", target_path);

    // Initial limit check (short lock)
    {
        let file_index = state.file_index.lock().await;
        if file_index.len() >= MAX_FILES {
            log::error!("Maximum limit of 1000 files reached");
            return Err("Maximum limit of 1000 files reached".to_string());
        }
    }

    // ✅ Hash + Copy in a single pass
    let mut hasher = Sha256::new();
    let mut source_file = File::open(source_path)
        .await
        .map_err(|e| {
            log::error!("Error opening source file: {}", e);
            e.to_string()
        })?;
    let mut target_file = File::create(&target_path)
        .await
        .map_err(|e| {
            log::error!("Error creating target file: {}", e);
            e.to_string()
        })?;

    log::info!("Files opened, starting streaming...");

    let mut buffer = vec![0u8; BUFFER_SIZE];

    loop {
        let bytes_read = source_file
            .read(&mut buffer)
            .await
            .map_err(|e| {
                log::error!("Error reading file: {}", e);
                e.to_string()
            })?;

        if bytes_read == 0 {
            break;
        }

        // Update hash
        hasher.update(&buffer[..bytes_read]);

        // Write to target
        target_file
            .write_all(&buffer[..bytes_read])
            .await
            .map_err(|e| {
                log::error!("Error writing file: {}", e);
                e.to_string()
            })?;
    }

    // Flush and close the files
    target_file
        .flush()
        .await
        .map_err(|e| {
            log::error!("Error flushing file: {}", e);
            e.to_string()
        })?;

    log::info!("Streaming completed, calculating hash...");

    let hash = hex::encode(hasher.finalize());

    // Use tokio to get metadata asynchronously
    let metadata = tokio::fs::metadata(source_path)
        .await
        .map_err(|e| {
            log::error!("Error reading file metadata: {}", e);
            e.to_string()
        })?;

    // Insert metadata into the index (separate lock)
    {
        let mut file_index = state.file_index.lock().await;
        // Secondary check: if other threads exceeded the limit
        if file_index.len() >= MAX_FILES {
            return Err("Maximum limit of 1000 files reached".to_string());
        }

        let file_info = FileInfo {
            filename: target_path.file_name().unwrap().to_str().unwrap().to_string(),
            size: metadata.len(),
            hash: hash.clone(),
            uploaded_at: chrono::Utc::now().to_rfc3339(),
        };

        file_index.insert(hash.clone(), file_info.clone());

        // Also save to the database for persistence
        let db = state.db.clone();
        tauri::async_runtime::spawn(async move {
            let repo = FileRepository::new(db);
            if let Err(e) = repo.save(&file_info).await {
                log::error!("Error saving to database: {}", e);
            }
        });
    }

    log::info!("File registered successfully: {} (hash: {})", filename, hash);

    Ok(hash)
}

#[cfg(test)]
mod tests {
    // Tests will be run with integration
    // Tauri 2.0 does not support mock_state in tests
}