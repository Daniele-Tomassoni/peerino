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
// P2P-to-Web Relay with backpressure
// Allows browsers to download files without installing the app

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;
use uuid::Uuid;

/// Configuration for the relay
#[derive(Debug, Clone)]
pub struct RelayConfig {
    pub buffer_size: usize,
    pub link_expiry_seconds: u64,
    pub max_downloads: u32,
}

impl Default for RelayConfig {
    fn default() -> Self {
        Self {
            buffer_size: 32,
            link_expiry_seconds: 24 * 60 * 60, // 24 hours
            max_downloads: u32::MAX, // No download limit (only expiry applies)
        }
    }
}

/// Public link information
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PublicLink {
    pub id: String,
    pub file_hash: String,
    pub expires_at: u64,
    pub max_downloads: u32,
    pub downloads_count: u32,
    pub created_at: u64,
}

/// Inbox for reverse file delivery
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Inbox {
    pub id: String,
    pub created_at: u64,
    pub expires_at: u64,
}

/// Relay manager for P2P-to-Web file sharing
pub struct RelayManager {
    config: RelayConfig,
    links: Arc<Mutex<HashMap<String, PublicLink>>>,
    inboxes: Arc<Mutex<HashMap<String, Inbox>>>,
}

impl RelayManager {
    pub fn new(config: RelayConfig) -> Self {
        Self {
            config,
            links: Arc::new(Mutex::new(HashMap::new())),
            inboxes: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Generate a public link for a file
    pub async fn generate_public_link(
        &self,
        file_hash: String,
        expires_in: Option<u64>,
        max_downloads: Option<u32>,
    ) -> Result<String, String> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let link = PublicLink {
            id: Uuid::new_v4().to_string(),
            file_hash,
            expires_at: now + expires_in.unwrap_or(self.config.link_expiry_seconds),
            max_downloads: max_downloads.unwrap_or(self.config.max_downloads),
            downloads_count: 0,
            created_at: now,
        };

        let link_id = link.id.clone();

        {
            let mut links = self.links.lock().await;
            links.insert(link_id.clone(), link);
        }

        Ok(link_id)
    }

    /// Get link information
    pub async fn get_link(&self, link_id: &str) -> Option<PublicLink> {
        let links = self.links.lock().await;
        links.get(link_id).cloned()
    }

    /// Validate and consume a link (returns file hash if valid)
    /// Uses a single lock to prevent race conditions
    pub async fn validate_and_consume_link(&self, link_id: &str) -> Option<String> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Single atomic operation with one lock
        let mut links = self.links.lock().await;

        if let Some(link) = links.get_mut(link_id) {
            // Check expiry
            if link.expires_at < now {
                links.remove(link_id);
                return None;
            }

            // Check download limit
            if link.downloads_count >= link.max_downloads {
                links.remove(link_id);
                return None;
            }

            // Increment download count
            link.downloads_count += 1;
            let file_hash = link.file_hash.clone();

            // Remove if reached limit
            if link.downloads_count >= link.max_downloads {
                links.remove(link_id);
            }

            Some(file_hash)
        } else {
            None
        }
    }

    /// Create an inbox for reverse file delivery
    pub async fn create_inbox(&self) -> Result<String, String> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let inbox = Inbox {
            id: Uuid::new_v4().to_string(),
            created_at: now,
            expires_at: now + self.config.link_expiry_seconds,
        };

        let inbox_id = inbox.id.clone();

        {
            let mut inboxes = self.inboxes.lock().await;
            inboxes.insert(inbox_id.clone(), inbox);
        }

        Ok(inbox_id)
    }

    /// Get inbox information
    pub async fn get_inbox(&self, inbox_id: &str) -> Option<Inbox> {
        let inboxes = self.inboxes.lock().await;
        inboxes.get(inbox_id).cloned()
    }

    /// Clean up expired links and inboxes
    pub async fn cleanup_expired(&self) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        {
            let mut links = self.links.lock().await;
            links.retain(|_, link| link.expires_at > now);
        }

        {
            let mut inboxes = self.inboxes.lock().await;
            inboxes.retain(|_, inbox| inbox.expires_at > now);
        }
    }
}

impl Default for RelayManager {
    fn default() -> Self {
        Self::new(RelayConfig::default())
    }
}