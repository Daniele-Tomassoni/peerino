// P2P Module for P2P Share - Phase 4
// Provides WebRTC-based P2P networking with DHT discovery

pub mod webrtc;
pub mod dht;
pub mod relay;

pub use dht::PeerInfo;
pub use relay::RelayManager;