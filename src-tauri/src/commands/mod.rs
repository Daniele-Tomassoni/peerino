pub mod clipboard;
pub mod download_file;
pub mod download_progress;
pub mod folder;
pub mod hash_integrity;
pub mod list_files;
pub mod local_link;
pub mod network_info;
pub mod open_url;
pub mod register_file;
pub mod start_server;
pub mod turn_limits;
pub mod p2p;

// ---------------------------------------------------------------------
// Limite dimensione file su connessione TURN (post-Fase 2)
// Lettura lazy: env var letta al primo accesso, poi cachata.
// ---------------------------------------------------------------------
pub fn get_turn_max_file_size() -> u64 {
    static CACHE: std::sync::OnceLock<u64> = std::sync::OnceLock::new();
    *CACHE.get_or_init(|| {
        std::env::var("TURN_MAX_FILE_SIZE")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(104_857_600) // 100 MB default
    })
}

pub fn get_turn_max_file_size_buffer() -> u64 {
    get_turn_max_file_size() + (get_turn_max_file_size() / 10) // 110 MB (+10%)
}