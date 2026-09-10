// Kademlia DHT for peer discovery
// Provides global peer discovery for P2P Internet connections

/// Information about a peer
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PeerInfo {
    pub peer_id: String,
    pub addresses: Vec<String>,
    pub last_seen: u64,
}
