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
use crate::FileInfo;
use rusqlite::{params, OptionalExtension, Connection};
use std::sync::Arc;

/// File repository for managing files in the database
pub struct FileRepository {
    conn: Arc<tokio::sync::Mutex<Connection>>,
}

impl FileRepository {
    /// Creates a new repository
    pub fn new(conn: Arc<tokio::sync::Mutex<Connection>>) -> Self {
        Self { conn }
    }

    /// Initialize the table if it does not exist
    pub async fn init_table(&self) -> Result<(), String> {
        let conn = self.conn.clone();

        tokio::task::spawn_blocking(move || {
            let db = conn.blocking_lock();
            db.execute(
                "CREATE TABLE IF NOT EXISTS files (
                    hash TEXT PRIMARY KEY,
                    filename TEXT NOT NULL,
                    size INTEGER NOT NULL,
                    uploaded_at TEXT NOT NULL
                )",
                [],
            )
            .map_err(|e| e.to_string())?;
            Ok::<_, String>(())
        })
        .await
        .map_err(|e| e.to_string())?
    }

    /// Saves a file to the index
    /// Uses spawn_blocking to avoid blocking the async runtime
    pub async fn save(&self, file_info: &FileInfo) -> Result<(), String> {
        let file_info = file_info.clone();
        let conn = self.conn.clone();

        tokio::task::spawn_blocking(move || {
            let db = conn.blocking_lock();
            db.execute(
                "INSERT OR REPLACE INTO files (hash, filename, size, uploaded_at) VALUES (?1, ?2, ?3, ?4)",
                params![file_info.hash, file_info.filename, file_info.size, file_info.uploaded_at],
            )
            .map_err(|e| e.to_string())?;
            Ok::<_, String>(())
        })
        .await
        .map_err(|e| e.to_string())?
    }

    /// Loads all files from the index
    /// Uses spawn_blocking to avoid blocking the async runtime
    pub async fn load_all(&self) -> Result<Vec<FileInfo>, String> {
        let conn = self.conn.clone();

        tokio::task::spawn_blocking(move || {
            let db = conn.blocking_lock();
            let mut stmt = db
                .prepare("SELECT hash, filename, size, uploaded_at FROM files ORDER BY uploaded_at DESC")
                .map_err(|e| e.to_string())?;

            let files = stmt
                .query_map([], |row| {
                    Ok(FileInfo {
                        hash: row.get(0)?,
                        filename: row.get(1)?,
                        size: row.get(2)?,
                        uploaded_at: row.get(3)?,
                    })
                })
                .map_err(|e| e.to_string())?
                .filter_map(|f| f.ok())
                .collect::<Vec<FileInfo>>();

            Ok(files)
        })
        .await
        .map_err(|e| e.to_string())?
    }

    /// Finds a file by hash
    /// Uses spawn_blocking to avoid blocking the async runtime
    #[allow(dead_code)]
    pub async fn find_by_hash(&self, hash: &str) -> Result<Option<FileInfo>, String> {
        let conn = self.conn.clone();
        let hash = hash.to_string();

        tokio::task::spawn_blocking(move || {
            let db = conn.blocking_lock();
            let mut stmt = db
                .prepare("SELECT hash, filename, size, uploaded_at FROM files WHERE hash = ?1")
                .map_err(|e| e.to_string())?;

            let result = stmt
                .query_row([hash], |row| {
                    Ok(FileInfo {
                        hash: row.get(0)?,
                        filename: row.get(1)?,
                        size: row.get(2)?,
                        uploaded_at: row.get(3)?,
                    })
                })
                .optional()
                .map_err(|e| e.to_string())?;

            Ok(result)
        })
        .await
        .map_err(|e| e.to_string())?
    }

    /// Removes a file from the index
    /// Uses spawn_blocking to avoid blocking the async runtime
    #[allow(dead_code)]
    pub async fn remove(&self, hash: &str) -> Result<(), String> {
        let conn = self.conn.clone();
        let hash = hash.to_string();

        tokio::task::spawn_blocking(move || {
            let db = conn.blocking_lock();
            db.execute("DELETE FROM files WHERE hash = ?1", [hash])
                .map_err(|e| e.to_string())?;
            Ok::<_, String>(())
        })
        .await
        .map_err(|e| e.to_string())?
    }

    /// Counts the number of files in the index
    /// Uses spawn_blocking to avoid blocking the async runtime
    #[allow(dead_code)]
    pub async fn count(&self) -> Result<usize, String> {
        let conn = self.conn.clone();

        tokio::task::spawn_blocking(move || {
            let db = conn.blocking_lock();
            let count: usize = db
                .query_row("SELECT COUNT(*) FROM files", [], |row| row.get(0))
                .map_err(|e| e.to_string())?;
            Ok(count)
        })
        .await
        .map_err(|e| e.to_string())?
    }

    /// Scans the shared-folder and adds missing files to the database
    /// Optimized: checks by file name before computing SHA-256 hash
    pub async fn scan_and_populate(&self, shared_folder: &str) -> Result<usize, String> {
        let conn = self.conn.clone();
        let shared_folder = shared_folder.to_string();

        tokio::task::spawn_blocking(move || {
            let db = conn.blocking_lock();

            // Read filename -> (hash, size) metadata from the database.
            let existing: std::collections::HashMap<String, (String, u64)> = {
                let mut stmt = db
                    .prepare("SELECT filename, hash, size FROM files")
                    .map_err(|e| e.to_string())?;
                let metadata = stmt
                    .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, u64>(2)?)))
                    .map_err(|e| e.to_string())?
                    .filter_map(|r| r.ok())
                    .map(|(filename, hash, size)| (filename, (hash, size)))
                    .collect();
                metadata
            };

            // Scan the folder
            let mut added = 0;
            let mut disk_filenames = std::collections::HashSet::new();
            if let Ok(entries) = std::fs::read_dir(&shared_folder) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() {
                        let filename = path.file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("unknown")
                            .to_string();

                        // Skip dotfiles (e.g., .tmp files, .DS_Store)
                        if filename.starts_with('.') {
                            continue;
                        }
                        disk_filenames.insert(filename.clone());
                        let metadata = match std::fs::metadata(&path) {
                            Ok(metadata) => metadata,
                            Err(_) => continue,
                        };
                        if let Some((_, db_size)) = existing.get(&filename) {
                            if *db_size == metadata.len() {
                                continue;
                            }
                        }

                        // Compute SHA-256 only for new files
                        let hash = {
                            use sha2::{Digest, Sha256};
                            let mut hasher = Sha256::new();
                            let mut file = std::fs::File::open(&path)
                                .map_err(|e| format!("Open error on {}: {}", filename, e))?;
                            let mut buffer = vec![0u8; 64 * 1024];
                            loop {
                                let bytes = std::io::Read::read(&mut file, &mut buffer)
                                    .map_err(|e| format!("Read error on {}: {}", filename, e))?;
                                if bytes == 0 { break; }
                                hasher.update(&buffer[..bytes]);
                            }
                            hex::encode(hasher.finalize())
                        };

                        // Add or refresh the database record.
                        let uploaded_at = chrono::Utc::now().to_rfc3339();
                        db.execute(
                            "INSERT OR REPLACE INTO files (hash, filename, size, uploaded_at) VALUES (?1, ?2, ?3, ?4)",
                            params![hash, filename, metadata.len(), uploaded_at],
                        )
                        .map_err(|e| e.to_string())?;
                        added += 1;
                    }
                }
            }

            // Remove database records whose files no longer exist.
            let mut stmt = db.prepare("SELECT filename FROM files").map_err(|e| e.to_string())?;
            let db_filenames: Vec<String> = stmt.query_map([], |row| row.get::<_, String>(0))
                .map_err(|e| e.to_string())?.filter_map(|r| r.ok()).collect();
            for filename in db_filenames {
                if !disk_filenames.contains(&filename) {
                    db.execute("DELETE FROM files WHERE filename = ?1", [&filename])
                        .map_err(|e| e.to_string())?;
                }
            }

            Ok(added)
        })
        .await
        .map_err(|e| e.to_string())?
    }
}
