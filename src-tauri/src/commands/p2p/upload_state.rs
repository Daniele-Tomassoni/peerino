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
// Shared state for incremental incoming file uploads (browser → app)
// Used by init_incoming_upload, append_incoming_chunk, finalize_incoming_file

use std::path::PathBuf;
use sha2::{Sha256, Digest};
use tokio::fs::File;

/// State for an in-progress incoming upload
pub struct UploadState {
    pub file: File,
    pub filename: String,
    pub written: u64,
    pub hasher: Sha256,
    pub temp_path: PathBuf,
    pub expected_hash: String,
}

impl UploadState {
    pub async fn new(temp_folder: &str, filename: String, expected_hash: String) -> Result<Self, String> {
        let temp_path = PathBuf::from(temp_folder)
            .join(format!(".{}_{}.tmp",
                uuid::Uuid::new_v4().simple(),
                filename.replace(|c: char| !c.is_alphanumeric(), "_")
            ));

        let file = tokio::fs::File::create(&temp_path)
            .await
            .map_err(|e| format!("Impossibile creare file temporaneo: {}", e))?;

        Ok(Self {
            file,
            filename,
            written: 0,
            hasher: Sha256::new(),
            temp_path,
            expected_hash,
        })
    }
}
