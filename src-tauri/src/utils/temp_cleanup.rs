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
use anyhow::Result;
use chrono::{DateTime, Utc};
use std::path::Path;
use std::time::Duration;
use tokio::fs;
use tokio::time;

/// Cleans files in the temp folder older than 1 hour.
/// Continues even if a file cannot be removed.
pub async fn cleanup_temp_files(temp_path: &str) -> Result<()> {
    let temp_dir = Path::new(temp_path);

    if !temp_dir.exists() {
        return Ok(());
    }

    let one_hour_ago = Utc::now() - chrono::Duration::hours(1);
    let mut entries = fs::read_dir(temp_dir).await?;

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();

        if path.is_file() {
            let metadata = match entry.metadata().await {
                Ok(m) => m,
                Err(e) => {
                    log::warn!("Unable to read metadata per {:?}: {}", path, e);
                    continue;
                }
            };

            let modified = match metadata.modified() {
                Ok(t) => DateTime::<Utc>::from(t),
                Err(e) => {
                    log::warn!("Unable to read timestamp per {:?}: {}", path, e);
                    continue;
                }
            };

            if modified < one_hour_ago {
                if let Err(e) = fs::remove_file(&path).await {
                    log::warn!("Unable to remove {:?}: {}", path, e);
                } else {
                    log::info!("Removed temporary file: {:?}", path);
                }
            }
        }
    }

    Ok(())
}

/// Cleans `.tmp` files in the shared-folder older than 1 hour.
/// This removes orphaned temp files left behind by interrupted uploads
/// (before the migration to temp_folder).
pub async fn cleanup_shared_folder_tmp(shared_folder: &str) -> Result<()> {
    let dir = Path::new(shared_folder);

    if !dir.exists() {
        return Ok(());
    }

    let one_hour_ago = Utc::now() - chrono::Duration::hours(1);
    let mut entries = fs::read_dir(dir).await?;

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();

        if path.is_file() {
            // Only clean .tmp files
            if path.extension().and_then(|e| e.to_str()) != Some("tmp") {
                continue;
            }

            let metadata = match entry.metadata().await {
                Ok(m) => m,
                Err(e) => {
                    log::warn!("Unable to read metadata per {:?}: {}", path, e);
                    continue;
                }
            };

            let modified = match metadata.modified() {
                Ok(t) => DateTime::<Utc>::from(t),
                Err(e) => {
                    log::warn!("Unable to read timestamp per {:?}: {}", path, e);
                    continue;
                }
            };

            if modified < one_hour_ago {
                if let Err(e) = fs::remove_file(&path).await {
                    log::warn!("Unable to remove {:?}: {}", path, e);
                } else {
                    log::info!("Removed orphan .tmp file: {:?}", path);
                }
            }
        }
    }

    Ok(())
}

/// Starts an async task that cleans temp files:
/// - Performs an immediate cleanup on startup
/// - Then cleans every hour
pub async fn start_cleanup_task(temp_path: &str, shared_folder: &str) {
    let temp_path = temp_path.to_string();
    let shared_folder = shared_folder.to_string();

    // ✅ Immediate cleanup on startup
    if let Err(e) = cleanup_temp_files(&temp_path).await {
        log::error!("Error in initial temp cleanup: {}", e);
    }
    if let Err(e) = cleanup_shared_folder_tmp(&shared_folder).await {
        log::error!("Error in initial .tmp shared-folder cleanup: {}", e);
    }

    // Then clean every hour
    let mut interval = time::interval(Duration::from_secs(3600));
    loop {
        interval.tick().await;
        if let Err(e) = cleanup_temp_files(&temp_path).await {
            log::error!("Error in temp cleanup: {}", e);
        }
        if let Err(e) = cleanup_shared_folder_tmp(&shared_folder).await {
            log::error!("Error in .tmp shared-folder cleanup: {}", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::fs;

    #[tokio::test]
    async fn test_cleanup_temp_files() {
        let temp_dir = tempfile::tempdir().unwrap();
        let temp_path = temp_dir.path().to_str().unwrap();

        // Empty directory → OK
        let result = cleanup_temp_files(temp_path).await;
        assert!(result.is_ok());

        // Create a new file
        let file_path = temp_dir.path().join("new.txt");
        fs::write(&file_path, b"test").await.unwrap();

        // Cleanup → the file is not removed (it is new)
        cleanup_temp_files(temp_path).await.unwrap();
        assert!(file_path.exists());
    }
}
