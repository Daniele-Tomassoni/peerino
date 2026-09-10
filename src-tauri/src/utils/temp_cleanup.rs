use anyhow::Result;
use chrono::{DateTime, Utc};
use std::path::Path;
use std::time::Duration;
use tokio::fs;
use tokio::time;

/// Pulisce i file nella cartella temp più vecchi di 1 ora.
/// Continua anche se un file non può essere rimosso.
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
                    log::warn!("Impossibile leggere metadata per {:?}: {}", path, e);
                    continue;
                }
            };

            let modified = match metadata.modified() {
                Ok(t) => DateTime::<Utc>::from(t),
                Err(e) => {
                    log::warn!("Impossibile leggere timestamp per {:?}: {}", path, e);
                    continue;
                }
            };

            if modified < one_hour_ago {
                if let Err(e) = fs::remove_file(&path).await {
                    log::warn!("Impossibile rimuovere {:?}: {}", path, e);
                } else {
                    log::info!("Rimosso file temporaneo: {:?}", path);
                }
            }
        }
    }

    Ok(())
}

/// Pulisce i file `.tmp` nella shared-folder più vecchi di 1 ora.
/// Questo rimuove i file temporanei orfani lasciati da upload interrottti
/// (prima della migrazione verso temp_folder).
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
                    log::warn!("Impossibile leggere metadata per {:?}: {}", path, e);
                    continue;
                }
            };

            let modified = match metadata.modified() {
                Ok(t) => DateTime::<Utc>::from(t),
                Err(e) => {
                    log::warn!("Impossibile leggere timestamp per {:?}: {}", path, e);
                    continue;
                }
            };

            if modified < one_hour_ago {
                if let Err(e) = fs::remove_file(&path).await {
                    log::warn!("Impossibile rimuovere {:?}: {}", path, e);
                } else {
                    log::info!("Rimosso file .tmp orfano: {:?}", path);
                }
            }
        }
    }

    Ok(())
}

/// Avvia un task asincrono che pulisce i file temp:
/// - Esegue una pulizia immediata all'avvio
/// - Poi pulisce ogni ora
pub async fn start_cleanup_task(temp_path: &str, shared_folder: &str) {
    let temp_path = temp_path.to_string();
    let shared_folder = shared_folder.to_string();

    // ✅ Pulizia immediata all'avvio
    if let Err(e) = cleanup_temp_files(&temp_path).await {
        log::error!("Errore nella pulizia iniziale temp: {}", e);
    }
    if let Err(e) = cleanup_shared_folder_tmp(&shared_folder).await {
        log::error!("Errore nella pulizia iniziale .tmp shared-folder: {}", e);
    }

    // Poi pulisce ogni ora
    let mut interval = time::interval(Duration::from_secs(3600));
    loop {
        interval.tick().await;
        if let Err(e) = cleanup_temp_files(&temp_path).await {
            log::error!("Errore nella pulizia temp: {}", e);
        }
        if let Err(e) = cleanup_shared_folder_tmp(&shared_folder).await {
            log::error!("Errore nella pulizia .tmp shared-folder: {}", e);
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

        // Directory vuota → OK
        let result = cleanup_temp_files(temp_path).await;
        assert!(result.is_ok());

        // Crea un file nuovo
        let file_path = temp_dir.path().join("new.txt");
        fs::write(&file_path, b"test").await.unwrap();

        // Pulizia → il file non viene rimosso (è nuovo)
        cleanup_temp_files(temp_path).await.unwrap();
        assert!(file_path.exists());
    }
}
