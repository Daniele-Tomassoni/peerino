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
use tauri::State;
use std::path::PathBuf;

/// Returns the list of all registered files
/// - Loads files from the database (scan only runs at startup or on manual refresh)
/// - Updates the in-memory index
/// - Verifies that files actually exist on disk
/// - Returns JSON with all files: { filename, size, hash, uploaded_at }
/// - Sorted by upload date descending (newest first)
#[tauri::command]
pub async fn list_files(state: State<'_, AppState>) -> Result<Vec<FileInfo>, String> {
    let shared_folder = state.shared_folder.clone();
    let db = state.db.clone();
    let file_index = state.file_index.clone();

    // Load all files from the database (no per-request scan — P1)
    let repo = FileRepository::new(db);
    let files = repo.load_all().await?;

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

    Ok(files)
}

/// Manually refresh the file index by scanning the shared folder.
/// Use this when files are added/removed outside the app (e.g., via filesystem).
#[tauri::command]
pub async fn refresh_files(state: State<'_, AppState>) -> Result<Vec<FileInfo>, String> {
    let shared_folder = state.shared_folder.clone();
    let db = state.db.clone();
    let file_index = state.file_index.clone();

    // Scan the shared folder for new/removed files
    let repo = FileRepository::new(db);
    let _ = repo.scan_and_populate(&shared_folder).await;

    // Load all files from the database (includes any newly scanned files)
    let files = repo.load_all().await?;

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

    Ok(files)
}

#[cfg(test)]
mod tests {
    // Tests will run with integration
    // Tauri 2.0 does not support mock_state in tests
}
