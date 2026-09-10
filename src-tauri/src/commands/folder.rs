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

/// Apre la cartella condivisa nel file manager del sistema
/// Usa il crate `open` per cross-platform compatibility
#[tauri::command]
pub async fn open_shared_folder(state: State<'_, AppState>) -> Result<(), String> {
    log::info!("open_shared_folder chiamato");
    
    let shared_folder = state.shared_folder.clone();
    
    // Risolvi il percorso assoluto
    let path = if Path::new(&shared_folder).is_absolute() {
        Path::new(&shared_folder).to_path_buf()
    } else {
        // Se è relativo, risolvi rispetto alla directory corrente
        std::env::current_dir()
            .map(|cwd| cwd.join(&shared_folder))
            .unwrap_or_else(|_| Path::new(&shared_folder).to_path_buf())
    };
    
    log::info!("Apertura cartella: {:?}", path);
    
    // Usa il crate `open` per aprire la cartella nel file manager
    open::that(&path)
        .map_err(|e| {
            log::error!("Errore apertura cartella: {}", e);
            format!("Impossibile aprire la cartella: {}", e)
        })?;
    
    log::info!("Cartella aperta con successo");
    Ok(())
}

/// Verifica se la cartella condivisa esiste
#[tauri::command]
pub async fn check_shared_folder(state: State<'_, AppState>) -> Result<bool, String> {
    let shared_folder = state.shared_folder.clone();
    let exists = Path::new(&shared_folder).exists();
    Ok(exists)
}

#[cfg(test)]
mod tests {
    // I test verranno eseguiti con integrazione
}