/// Apre un URL nel browser predefinito
#[tauri::command]
pub async fn open_url(url: String) -> Result<(), String> {
    tauri::async_runtime::spawn(async move {
        // Apri l'URL nel browser
        if let Err(e) = open::that(&url) {
            log::error!("Errore apertura URL: {}", e);
        }
    });
    
    Ok(())
}

#[cfg(test)]
mod tests {
    // I test verranno eseguiti con integrazione
}