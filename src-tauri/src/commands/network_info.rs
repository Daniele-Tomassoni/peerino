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
use crate::utils::network::NetworkInfo;

/// Detects network information
/// - Detects the local IP (IPv4, non-loopback)
/// - Returns { ip: String, port: u16 }
/// - Uses TcpListener to find a valid IP
/// - If no IP is found, returns "127.0.0.1" as fallback
/// - The port is always 3000 (fixed)
#[tauri::command]
pub async fn get_network_info() -> Result<NetworkInfo, String> {
    crate::utils::network::get_local_ip()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_network_info() {
        let result = get_network_info().await;
        assert!(result.is_ok());
        let info = result.unwrap();

        // Verify non-empty IP
        assert!(!info.ip.is_empty());

        // Verify valid IP format (IPv4 with dots or IPv6 localhost)
        let is_valid_ip = info.ip.contains('.')
            || info.ip == "::1"
            || info.ip == "127.0.0.1";
        assert!(is_valid_ip, "Invalid IP: {}", info.ip);

        // Verify valid port (basic check only)
        assert!(info.port > 0, "Invalid port: {}", info.port);
    }
}