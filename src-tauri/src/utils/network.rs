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
use std::net::TcpListener;
use std::time::Duration;

/// Network detection result
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NetworkInfo {
    pub ip: String,
    pub port: u16,
}

/// Detects the local network IP (IPv4, non-loopback)
/// Uses TcpListener to find a valid IP
/// If no IP is found, returns "127.0.0.1" as fallback
pub fn get_local_ip() -> Result<NetworkInfo, String> {
    // Try binding to 0.0.0.0:0 to get the local IP
    let socket = TcpListener::bind("0.0.0.0:0")
        .map_err(|e| format!("Failed to bind socket: {}", e))?;

    let local_addr = socket.local_addr()
        .map_err(|e| format!("Failed to get local address: {}", e))?;

    let ip = local_addr.ip();

    // If the IP is loopback or 0.0.0.0, try alternative methods
    if ip.is_loopback() || ip.is_unspecified() {
        // Multiple attempts with timeout
        let endpoints = ["8.8.8.8:80", "1.1.1.1:80", "9.9.9.9:80"];
        let mut detected_ip = None;

        for endpoint in endpoints {
            // Create a UDP socket with timeout
            if let Ok(socket) = std::net::UdpSocket::bind("0.0.0.0:0") {
                if let Ok(()) = socket.set_read_timeout(Some(Duration::from_millis(500))) {
                    if let Ok(()) = socket.connect(endpoint) {
                        if let Ok(addr) = socket.local_addr() {
                            let ip = addr.ip();
                            if !ip.is_loopback() && !ip.is_unspecified() {
                                detected_ip = Some(ip);
                                break;
                            }
                        }
                    }
                }
            }
        }

        if let Some(ip) = detected_ip {
            return Ok(NetworkInfo {
                ip: ip.to_string(),
                port: crate::server::DEFAULT_HTTP_PORT,
            });
        }

        // Final fallback to localhost
        log::warn!("No network IP detected, falling back to 127.0.0.1");
        return Ok(NetworkInfo {
            ip: "127.0.0.1".to_string(),
            port: crate::server::DEFAULT_HTTP_PORT,
        });
    }

    // IP already valid
    Ok(NetworkInfo {
        ip: ip.to_string(),
        port: crate::server::DEFAULT_HTTP_PORT,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_local_ip() {
        let result = get_local_ip();
        assert!(result.is_ok());
        let info = result.unwrap();
        assert!(!info.ip.is_empty());
        assert_eq!(info.port, crate::server::DEFAULT_HTTP_PORT);
    }
}