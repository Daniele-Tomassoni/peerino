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
// Ephemeral TURN credentials following the coturn "REST API" authentication
// scheme (draft-ietf-ubiq-webrtc-turn-rest), the de-facto standard supported
// by coturn via `use-auth-secret` + `static-auth-secret`.
//
//   username   = "{unix_expiry_timestamp}"
//   credential = base64( HMAC-SHA1( shared_secret, username ) )
//
// Properties:
// - The shared secret NEVER travels in links: it lives only in the app .env
//   and in the coturn configuration on the VPS.
// - Credentials embedded in generated links stop working after the TTL,
//   bounding the damage of a leaked link.
// - No external calls: everything is computed locally (microseconds).
//
// The same scheme is used for both internet link generation and the internet
// inbox; only the TTL differs (short for downloads, aligned to inbox expiry).

use base64::{engine::general_purpose::STANDARD, Engine};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha1::Sha1;

type HmacSha1 = Hmac<Sha1>;

/// Default TTL for ephemeral credentials when TURN_CRED_TTL_SECS is not set.
pub const DEFAULT_CRED_TTL_SECS: u64 = 7200; // 2 hours

/// Fallback STUN servers used when STUN_URLS is not configured.
/// Ordered by regional relevance: China/Russia first, then Google/Cloudflare as global fallback.
/// WebRTC ICE tries each server in order and uses the first that responds.
pub const DEFAULT_STUN_URLS: &str =
    "stun:stun.miwifi.com:3478,stun:stun.chat.bilibili.com:3478,stun:stun.rtc.yandex.net:3478,stun:stun.l.google.com:19302,stun:stun.cloudflare.com:3478,stun:stun1.l.google.com:19302";

/// Ephemeral TURN credentials ready to be embedded in a link.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnCredentials {
    pub username: String,
    pub credential: String,
    /// TURN server URLs (from TURN_URLS, comma separated).
    pub urls: Vec<String>,
}

/// Full ICE/signaling configuration resolved from environment variables.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IceLinkConfig {
    /// Signaling host for PeerJS (SIGNALING_URL). None = default cloud (0.peerjs.com).
    pub signaling_url: Option<String>,
    /// STUN urls (STUN_URLS, comma separated). Defaults to Google public STUN.
    pub stun_urls: Vec<String>,
    /// Some(turn_credentials) if TURN_AUTH_SECRET + TURN_URLS are configured.
    pub turn: Option<TurnCredentials>,
}

/// Splits a comma-separated URL list from an env var, trimming empties.
fn split_urls(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Generates ephemeral credentials with the given secret and TTL.
pub fn generate_ephemeral_credentials(
    secret: &str,
    ttl_secs: u64,
    urls: Vec<String>,
) -> Result<TurnCredentials, String> {
    if secret.is_empty() {
        return Err("TURN_AUTH_SECRET is empty".to_string());
    }
    if urls.is_empty() {
        return Err("TURN_URLS is empty".to_string());
    }

    let expiry = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| format!("System time error: {}", e))?
        .as_secs()
        + ttl_secs;

    // coturn REST scheme: username is the unix expiry timestamp.
    let username = format!("{}", expiry);

    let mut mac = HmacSha1::new_from_slice(secret.as_bytes())
        .map_err(|e| format!("HMAC key error: {}", e))?;
    mac.update(username.as_bytes());
    let digest = mac.finalize().into_bytes();
    let credential = STANDARD.encode(digest);

    Ok(TurnCredentials {
        username,
        credential,
        urls,
    })
}

/// Reads the optional PeerJS signaling host from the environment.
/// None means the default cloud signaling host.
pub fn signaling_url_from_env() -> Option<String> {
    std::env::var("SIGNALING_URL")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

/// Resolves the ICE/signaling configuration from environment variables:
/// - SIGNALING_URL      (optional): self-hosted peerjs-server host
/// - STUN_URLS          (optional): comma separated, defaults to Google STUN
/// - TURN_URLS          (optional): comma separated; required to enable TURN
/// - TURN_AUTH_SECRET   (optional): shared secret with coturn
/// - TURN_CRED_TTL_SECS (optional): credential lifetime, default 7200
///
/// TURN is enabled ONLY if both TURN_AUTH_SECRET and TURN_URLS are set:
/// without them the links stay STUN-only, which is the safe default.
pub fn ice_link_config_from_env() -> IceLinkConfig {
    let ttl_secs = std::env::var("TURN_CRED_TTL_SECS")
        .ok()
        .and_then(|v| v.trim().parse::<u64>().ok())
        .filter(|&v| v > 0)
        .unwrap_or(DEFAULT_CRED_TTL_SECS);
    ice_link_config_from_env_with_ttl(ttl_secs)
}

/// Same as [`ice_link_config_from_env`] but with an explicit TTL. Used by the
/// internet inbox to align the credential lifetime with the inbox validity
/// (24h), so TURN does not die before the inbox itself.
pub fn ice_link_config_from_env_with_ttl(ttl_secs: u64) -> IceLinkConfig {
    let signaling_url = signaling_url_from_env();

    let stun_urls = split_urls(
        &std::env::var("STUN_URLS").unwrap_or_else(|_| DEFAULT_STUN_URLS.to_string()),
    );

    let secret = std::env::var("TURN_AUTH_SECRET")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_default();

    let urls = std::env::var("TURN_URLS")
        .ok()
        .map(|s| split_urls(&s))
        .unwrap_or_default();

    let turn = if !secret.is_empty() && !urls.is_empty() {
        match generate_ephemeral_credentials(&secret, ttl_secs, urls) {
            Ok(creds) => {
                log::info!(
                    "🔑 Ephemeral TURN credentials generated (TTL {}s)",
                    ttl_secs
                );
                Some(creds)
            }
            Err(e) => {
                log::warn!("⚠️ Failed to generate TURN credentials: {}", e);
                None
            }
        }
    } else {
        None
    };

    IceLinkConfig {
        signaling_url,
        stun_urls,
        turn,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ephemeral_credentials_shape() {
        let creds =
            generate_ephemeral_credentials("test-secret", 3600, vec!["turn:x:3478".into()])
                .expect("credentials");
        // Username must be a plain unix timestamp string
        assert!(creds.username.parse::<u64>().is_ok());
        // Credential must be valid base64 of a 20-byte HMAC-SHA1
        assert_eq!(STANDARD.decode(&creds.credential).unwrap().len(), 20);
        assert_eq!(creds.urls, vec!["turn:x:3478".to_string()]);
    }

    #[test]
    fn empty_secret_is_rejected() {
        assert!(generate_ephemeral_credentials("", 60, vec!["turn:x".into()]).is_err());
    }

    #[test]
    fn deterministic_for_same_inputs() {
        let a = generate_ephemeral_credentials("s", 100, vec!["u".into()]).unwrap();
        let b = generate_ephemeral_credentials("s", 100, vec!["u".into()]).unwrap();
        assert_eq!(a.username, b.username);
        assert_eq!(a.credential, b.credential);
    }
}
