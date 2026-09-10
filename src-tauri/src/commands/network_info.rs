// Peerino - P2P file sharing senza cloud, senza account, senza intermediari.
// Copyright (C) 2025 Daniele
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

/// Rileva le informazioni di rete
/// - Rileva l'IP locale (IPv4, non loopback)
/// - Restituisce { ip: String, port: u16 }
/// - Usa TcpListener per trovare un IP valido
/// - Se non trova un IP, restituisce "127.0.0.1" come fallback
/// - La porta è sempre 3000 (fissa)
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
        
        // Verifica IP non vuoto
        assert!(!info.ip.is_empty());
        
        // Verifica formato IP valido (IPv4 con punti o IPv6 localhost)
        let is_valid_ip = info.ip.contains('.') 
            || info.ip == "::1" 
            || info.ip == "127.0.0.1";
        assert!(is_valid_ip, "IP non valido: {}", info.ip);
        
        // Verifica porta valida (solo controllo di base)
        assert!(info.port > 0, "Porta non valida: {}", info.port);
    }
}