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
use crate::{AppState, FileInfo};
use crate::database::files::FileRepository;
use tauri::State;
use std::path::PathBuf;

/// Restituisce la lista di tutti i file registrati
/// - Carica i file dal database (scansione avviene solo all'avvio o su refresh manuale)
/// - Aggiorna l'indice in memoria
/// - Verifica che i file esistano effettivamente su disco
/// - Restituisce JSON con tutti i file: { filename, size, hash, uploaded_at }
/// - Ordinato per data di caricamento decrescente (più recenti prima)
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
    // I test verranno eseguiti con integrazione
    // Tauri 2.0 non supporta mock_state in test
}
