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

/// Rileva l'IP locale della rete (IPv4, non loopback)
/// Usa TcpListener per trovare un IP valido
/// Se non trova un IP, restituisce "127.0.0.1" come fallback
pub fn get_local_ip() -> Result<NetworkInfo, String> {
    // Prova a bindare a 0.0.0.0:0 per ottenere l'IP locale
    let socket = TcpListener::bind("0.0.0.0:0")
        .map_err(|e| format!("Impossibile bindare socket: {}", e))?;
    
    let local_addr = socket.local_addr()
        .map_err(|e| format!("Impossibile ottenere indirizzo locale: {}", e))?;
    
    let ip = local_addr.ip();
    
    // Se l'IP è loopback o 0.0.0.0, proviamo metodi alternativi
    if ip.is_loopback() || ip.is_unspecified() {
        // Tentativi multipli con timeout
        let endpoints = ["8.8.8.8:80", "1.1.1.1:80", "9.9.9.9:80"];
        let mut detected_ip = None;
        
        for endpoint in endpoints {
            // Crea un socket UDP con timeout
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

        // Fallback finale a localhost
        log::warn!("Nessun IP di rete rilevato, fallback a 127.0.0.1");
        return Ok(NetworkInfo {
            ip: "127.0.0.1".to_string(),
            port: crate::server::DEFAULT_HTTP_PORT,
        });
    }
    
    // IP già valido
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