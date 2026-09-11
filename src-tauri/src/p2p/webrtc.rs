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
// WebRTC configuration for P2P networking
// Provides ICE server configuration for frontend PeerJS

use std::time::Duration;

/// ICE server configuration
#[allow(dead_code)]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IceServer {
    pub urls: Vec<String>,
    pub username: Option<String>,
    pub credential: Option<String>,
}

/// WebRTC configuration for P2P connections
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct WebRtcConfig {
    pub ice_servers: Vec<IceServer>,
    pub connection_timeout: Duration,
}

#[allow(dead_code)]
impl Default for WebRtcConfig {
    fn default() -> Self {
        Self {
            // STUN servers: Cina/Russia first, Google/Cloudflare as global fallback.
            // WebRTC ICE tries each server in order and uses the first that responds.
            ice_servers: vec![
                IceServer {
                    urls: vec!["stun:stun.miwifi.com:3478".to_string()],
                    username: None,
                    credential: None,
                },
                IceServer {
                    urls: vec!["stun:stun.chat.bilibili.com:3478".to_string()],
                    username: None,
                    credential: None,
                },
                IceServer {
                    urls: vec!["stun:stun.rtc.yandex.net:3478".to_string()],
                    username: None,
                    credential: None,
                },
                IceServer {
                    urls: vec!["stun:stun.l.google.com:19302".to_string()],
                    username: None,
                    credential: None,
                },
                IceServer {
                    urls: vec!["stun:stun.cloudflare.com:3478".to_string()],
                    username: None,
                    credential: None,
                },
                IceServer {
                    urls: vec!["stun:stun1.l.google.com:19302".to_string()],
                    username: None,
                    credential: None,
                },
            ],
            connection_timeout: Duration::from_secs(30),
        }
    }
}

#[allow(dead_code)]
impl WebRtcConfig {
    /// Create configuration with TURN servers (for production/mobile)
    pub fn with_turn(username: String, password: String) -> Self {
        let mut config = Self::default();
        config.ice_servers.push(IceServer {
            urls: vec![
                "turn:global.turn.metered.ca:80?transport=udp".to_string(),
                "turn:global.turn.metered.ca:443?transport=tcp".to_string(),
                "turns:global.turn.metered.ca:443?transport=tcp".to_string(),
            ],
            username: Some(username),
            credential: Some(password),
        });
        config
    }
}

/// Get the default ICE servers configuration (for frontend use)
#[allow(dead_code)]
pub fn get_ice_servers_json() -> serde_json::Value {
    serde_json::json!([
        {
            "urls": ["stun:stun.l.google.com:19302"]
        },
        {
            "urls": ["stun:stun1.l.google.com:19302"]
        }
    ])
}