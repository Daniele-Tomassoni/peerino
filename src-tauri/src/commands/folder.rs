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
use crate::AppState;
use std::path::Path;
use tauri::State;

/// Opens the shared folder in the system file manager
/// Uses the `open` crate for cross-platform compatibility
#[tauri::command]
pub async fn open_shared_folder(state: State<'_, AppState>) -> Result<(), String> {
    log::info!("open_shared_folder called");

    let shared_folder = state.shared_folder.clone();

    // Resolve the absolute path
    let path = if Path::new(&shared_folder).is_absolute() {
        Path::new(&shared_folder).to_path_buf()
    } else {
        // If relative, resolve against the current directory
        std::env::current_dir()
            .map(|cwd| cwd.join(&shared_folder))
            .unwrap_or_else(|_| Path::new(&shared_folder).to_path_buf())
    };

    log::info!("Opening folder: {:?}", path);

    // Use the `open` crate to open the folder in the file manager
    open::that(&path)
        .map_err(|e| {
            log::error!("Error opening folder: {}", e);
            format!("Failed to open folder: {}", e)
        })?;

    log::info!("Folder opened successfully");
    Ok(())
}

/// Check if the shared folder exists
#[tauri::command]
pub async fn check_shared_folder(state: State<'_, AppState>) -> Result<bool, String> {
    let shared_folder = state.shared_folder.clone();
    let exists = Path::new(&shared_folder).exists();
    Ok(exists)
}

#[cfg(test)]
mod tests {
    // Tests will run with integration
}