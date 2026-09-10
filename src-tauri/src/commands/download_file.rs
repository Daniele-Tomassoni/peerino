use crate::AppState;
use crate::utils::is_safe_filename;
use std::path::{Path, PathBuf};
use std::time::Instant;
use tauri::State;
use tauri::Emitter;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const BUFFER_SIZE: usize = 64 * 1024; // 64KB

/// Valida il percorso di destinazione fornito dal frontend prima di scrivere.
/// Previene la scrittura arbitraria su disco (path traversal / sovrascritture
/// accidentali) richiedendo un percorso assoluto la cui cartella padre esista
/// e il cui nome file sia sicuro.
fn validate_target_path(target_path: &str) -> Result<PathBuf, String> {
    let path = Path::new(target_path);
    if !path.is_absolute() {
        return Err("Il percorso di destinazione deve essere assoluto".to_string());
    }
    let parent = path
        .parent()
        .ok_or_else(|| "Percorso di destinazione non valido".to_string())?;
    if !parent.exists() {
        return Err("La cartella di destinazione non esiste".to_string());
    }
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| "Nome file di destinazione non valido".to_string())?;
    if !is_safe_filename(file_name) {
        return Err("Nome file di destinazione non valido".to_string());
    }
    Ok(path.to_path_buf())
}

/// Scarica un file dalla shared folder
/// - Cerca il file nell'indice tramite hash
/// - Copia il file in streaming verso target_path (validato)
/// - Traccia il progresso del download in tempo reale
/// - Restituisce il nome originale del file
#[tauri::command]
pub async fn download_file(
    hash: String,
    target_path: String,
    state: State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<String, String> {
    // Trova il file nell'indice
    let file_info = {
        let file_index = state.file_index.lock().await;
        file_index
            .get(&hash)
            .cloned()
            .ok_or_else(|| "File non trovato nell'indice".to_string())?
    };

    // ✅ Validazione filename sorgente (difesa in profondità)
    if !is_safe_filename(&file_info.filename) {
        return Err("Nome file non valido".to_string());
    }

    // Costruisci il percorso del file sorgente
    let shared_folder = state.shared_folder.clone();
    let source_path = Path::new(&shared_folder).join(&file_info.filename);

    // Verifica che il file esista
    if !source_path.exists() {
        return Err(format!("File non trovato sul disco: {}", file_info.filename));
    }

    // ✅ Valida il percorso di destinazione prima di aprire il file
    let target_path = validate_target_path(&target_path)?;

    // Register cancellation flag for this download
    let cancelled_flag = state.download_tracker.register_cancellation_flag(&hash).await;

    // Copia il file in streaming
    let mut source_file = File::open(&source_path)
        .await
        .map_err(|e| e.to_string())?;
    let mut target_file = File::create(&target_path)
        .await
        .map_err(|e| e.to_string())?;

    let mut buffer = vec![0u8; BUFFER_SIZE];
    let total_size = file_info.size;
    let mut downloaded_bytes: u64 = 0;
    let start_time = Instant::now();

    loop {
        // Check cancellation flag
        if cancelled_flag.load(std::sync::atomic::Ordering::SeqCst) {
            log::info!("Download cancelled: {}", hash);
            let _ = target_file.flush().await;
            let _ = tokio::fs::remove_file(&target_path).await;
            let _ = state.download_tracker.remove_download(&hash).await;
            return Err("Download annullato".to_string());
        }

        let bytes_read = source_file
            .read(&mut buffer)
            .await
            .map_err(|e| e.to_string())?;

        if bytes_read == 0 {
            break;
        }

        // ✅ write_all garantisce scrittura completa
        target_file
            .write_all(&buffer[..bytes_read])
            .await
            .map_err(|e| e.to_string())?;

        downloaded_bytes += bytes_read as u64;

        // Update download progress tracker
        let elapsed_secs = start_time.elapsed().as_secs_f64();
        let speed_mbps = if elapsed_secs > 0.0 {
            (downloaded_bytes as f64 / (1024.0 * 1024.0)) / elapsed_secs
        } else {
            0.0
        };
        let _ = state.download_tracker.update_progress(
            &hash,
            downloaded_bytes,
            total_size,
            speed_mbps,
            "local",
            &file_info.filename,
        ).await;

        // Emit Tauri event for real-time download progress
        let progress = if total_size > 0 {
            (downloaded_bytes as f64 / total_size as f64 * 100.0).min(100.0).round() as u32
        } else {
            0
        };
        let _ = app_handle.emit("download-progress", crate::commands::download_progress::DownloadProgress {
            hash: hash.clone(),
            filename: file_info.filename.clone(),
            total_bytes: total_size,
            downloaded_bytes,
            speed_mbps,
            peer_ip: "local".to_string(),
            progress,
            cancelled: false,
        });
    }

    // Sincronizza per assicurarsi che i dati siano scritti su disco
    target_file.sync_all().await.map_err(|e| e.to_string())?;

    // Update final progress (100%) and remove from tracker
    let _ = state.download_tracker.update_progress(
        &hash,
        total_size,
        total_size,
        0.0,
        "local",
        &file_info.filename,
    ).await;
    let _ = state.download_tracker.remove_download(&hash).await;

    log::info!("File scaricato: {} (hash: {})", file_info.filename, hash);

    Ok(file_info.filename)
}

#[cfg(test)]
mod tests {
    // I test verranno eseguiti con integrazione
    // Tauri 2.0 non supporta mock_state in test
}
