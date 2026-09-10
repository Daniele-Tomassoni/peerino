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