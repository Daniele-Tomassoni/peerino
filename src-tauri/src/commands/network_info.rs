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