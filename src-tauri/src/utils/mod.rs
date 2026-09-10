pub mod network;
pub mod temp_cleanup;
pub mod peer_id;
pub mod turn_creds;
pub mod ice_provider;

/// Verifica che un filename sia sicuro (no path traversal).
/// Rifiuta nomi vuoti o contenenti `..`, `/` o `\`.
/// Condiviso tra i vari handler per evitare duplicazioni e mantenere
/// coerente la validazione dei nomi file in tutto il backend.
pub fn is_safe_filename(filename: &str) -> bool {
    !filename.is_empty()
        && !filename.contains("..")
        && !filename.contains('/')
        && !filename.contains('\\')
}

/// Restituisce la directory in cui si trova l'eseguibile dell'app.
/// Questo garantisce che le cartelle `shared-folder`, `temp` e `config`
/// vengano create nella stessa directory di `peerino.exe`, indipendentemente
/// dalla directory di lavoro corrente.
pub fn get_app_dir() -> String {
    std::env::current_exe()
        .ok()
        .and_then(|exe_path| exe_path.parent().map(|p| p.to_path_buf()))
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| ".".to_string())
}
