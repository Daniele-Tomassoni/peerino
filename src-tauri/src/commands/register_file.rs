use crate::{AppState, FileInfo};
use crate::database::files::FileRepository;
use sha2::{Digest, Sha256};
use std::path::Path;
use tauri::State;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const MAX_FILES: usize = 1000;
const BUFFER_SIZE: usize = 64 * 1024; // 64KB

/// Registra un file nella shared folder
/// - Riceve il percorso assoluto del file
/// - Apre il file in streaming con tokio::fs
/// - Calcola SHA-256 in streaming durante la copia
/// - Copia il file in shared-folder/ gestendo conflitti di nome
/// - Inserisce i metadati in un indice locale e nel database
/// - Restituisce l'hash calcolato
#[tauri::command]
pub async fn register_file(
    file_path: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    log::info!("register_file chiamato con path: {}", file_path);
    
    let source_path = Path::new(&file_path);
    
    // Verifica che il file esista
    if !source_path.exists() {
        log::error!("File non trovato: {}", file_path);
        return Err(format!("File non trovato: {}", file_path));
    }

    // Ottieni il nome del file
    let filename = source_path
        .file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "unknown".to_string());
    
    log::info!("Nome file estratto: {}", filename);

    // Gestisci il conflitto di nomi
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

    // Controllo iniziale del limite (lock breve)
    {
        let file_index = state.file_index.lock().await;
        if file_index.len() >= MAX_FILES {
            log::error!("Limite massimo di 1000 file raggiunto");
            return Err("Limite massimo di 1000 file raggiunto".to_string());
        }
    }

    // ✅ Hash + Copia in un unico passaggio
    let mut hasher = Sha256::new();
    let mut source_file = File::open(source_path)
        .await
        .map_err(|e| {
            log::error!("Errore apertura file sorgente: {}", e);
            e.to_string()
        })?;
    let mut target_file = File::create(&target_path)
        .await
        .map_err(|e| {
            log::error!("Errore creazione file target: {}", e);
            e.to_string()
        })?;
    
    log::info!("File aperti, inizio streaming...");
    
    let mut buffer = vec![0u8; BUFFER_SIZE];

    loop {
        let bytes_read = source_file
            .read(&mut buffer)
            .await
            .map_err(|e| {
                log::error!("Errore lettura file: {}", e);
                e.to_string()
            })?;
        
        if bytes_read == 0 {
            break;
        }
        
        // Aggiorna hash
        hasher.update(&buffer[..bytes_read]);
        
        // Scrivi target
        target_file
            .write_all(&buffer[..bytes_read])
            .await
            .map_err(|e| {
                log::error!("Errore scrittura file: {}", e);
                e.to_string()
            })?;
    }
    
    // Flush e chiudi i file
    target_file
        .flush()
        .await
        .map_err(|e| {
            log::error!("Errore flush file: {}", e);
            e.to_string()
        })?;
    
    log::info!("Streaming completato, calcolo hash...");

    let hash = hex::encode(hasher.finalize());
    
    // Usa tokio per ottenere i metadati in modo asincrono
    let metadata = tokio::fs::metadata(source_path)
        .await
        .map_err(|e| {
            log::error!("Errore metadati file: {}", e);
            e.to_string()
        })?;

    // Inserisci i metadati nell'indice (lock separato)
    {
        let mut file_index = state.file_index.lock().await;
        // Controllo secondario: se altri thread hanno superato il limite
        if file_index.len() >= MAX_FILES {
            return Err("Limite massimo di 1000 file raggiunto".to_string());
        }
        
        let file_info = FileInfo {
            filename: target_path.file_name().unwrap().to_str().unwrap().to_string(),
            size: metadata.len(),
            hash: hash.clone(),
            uploaded_at: chrono::Utc::now().to_rfc3339(),
        };
        
        file_index.insert(hash.clone(), file_info.clone());
        
        // Salva anche nel database per persistenza
        let db = state.db.clone();
        tauri::async_runtime::spawn(async move {
            let repo = FileRepository::new(db);
            if let Err(e) = repo.save(&file_info).await {
                log::error!("Errore salvataggio database: {}", e);
            }
        });
    }

    log::info!("File registrato con successo: {} (hash: {})", filename, hash);
    
    Ok(hash)
}

#[cfg(test)]
mod tests {
    // I test verranno eseguiti con integrazione
    // Tauri 2.0 non supporta mock_state in test
}