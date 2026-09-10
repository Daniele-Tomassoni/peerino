// P2P Commands for Phase 4 (Internet) and Phase 5 (Local)
// Provides Tauri commands for P2P networking functionality

pub mod connect;
pub mod disconnect;
pub mod peers;
pub mod generate_link;
pub mod stream_file;
pub mod create_inbox;
pub mod create_inbox_local;
pub mod generate_web_link;
pub mod set_peer_id;
pub mod p2p_file_transfer;
pub mod upload_state;
pub mod upload_progress;
pub mod init_incoming_upload;
pub mod append_incoming_chunk;
pub mod finalize_incoming_file;
