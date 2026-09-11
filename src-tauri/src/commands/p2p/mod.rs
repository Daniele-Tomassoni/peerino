// Peerino - P2P file sharing without cloud, without accounts, without intermediaries.
// Copyright (C) 2026 Daniele Tomassoni
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
