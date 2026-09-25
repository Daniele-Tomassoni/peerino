#![allow(dead_code)]
// Modulo con risoluzione ICE multi-provider (Cloudflare, Metered, coturn).
// Attualmente usato solo per lettura SIGNALING_URL. Le funzioni fetch_*
// sono mantenute per: eventuale modello sender-embed, supporto provider
// alternativi, debug.

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
// Unified ICE servers provider: abstraction over multiple TURN/STUN sources.
// Allows switching providers without modifying the frontend code:
//
// - **Metered.ca REST API**: dynamic fetch of iceServers via API key.
//   Transient: use until you have a self-hosted VPS.
//   Env: `METERED_API_KEY` (required), `METERED_API_BASE` (optional,
//   default `https://peerino.metered.live/api/v1`).
//
// - **Coturn REST (HMAC-SHA1)**: ephemeral credentials computed locally.
//   Self-hosted: the final transition. Requires coturn with
//   `use-auth-secret` + `static-auth-secret`.
//   Env: `TURN_AUTH_SECRET` + `TURN_URLS` + optional `TURN_CRED_TTL_SECS`.
//
// - **Static STUN only**: safe fallback. No TURN.
//   Env: only `STUN_URLS` (optional, default Google public).
//
// The `fetch_ice_servers` method automatically selects the provider based on
// available env vars, with priority metered > coturn > static.

use crate::utils::turn_creds::{
    ice_link_config_from_env, ice_link_config_from_env_with_ttl, IceLinkConfig,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use serde::{Deserialize, Serialize};
use std::time::Duration;

const METERED_DEFAULT_BASE: &str = "https://peerino.metered.live/api/v1";
const FETCH_TIMEOUT_SECS: u64 = 10;

/// Default Cloudflare Worker TURN proxy endpoint.
///
/// This is a PUBLIC endpoint (not a secret). It is hardcoded as a fallback
/// so that the release binary works out-of-the-box even without a `.env` file.
/// Developers can override it via the `TURN_CREDENTIALS_ENDPOINT` env var.
const DEFAULT_TURN_CREDENTIALS_ENDPOINT: &str =
    "https://peerino-turn-proxy.shaft-bdc.workers.dev/api/turn-credentials";

/// Resolves the Cloudflare Worker TURN proxy endpoint.
///
/// Priority: `TURN_CREDENTIALS_ENDPOINT` env var (if set and non-empty) >
/// `DEFAULT_TURN_CREDENTIALS_ENDPOINT` (hardcoded public endpoint).
fn resolve_turn_credentials_endpoint() -> String {
    std::env::var("TURN_CREDENTIALS_ENDPOINT")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| DEFAULT_TURN_CREDENTIALS_ENDPOINT.to_string())
}

/// Explicit provider preference from ICE_PROVIDER env var.
/// Valori validi: "metered" | "coturn" | "static".
/// Se non impostato, comportamento auto (Metered → Coturn → Static).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum IceProviderPreference {
    #[default]
    Auto,
    Metered,
    Cloudflare,
    Coturn,
    Static,
}

impl IceProviderPreference {
    fn from_env() -> Self {
        match std::env::var("ICE_PROVIDER")
            .unwrap_or_default()
            .trim()
            .to_lowercase()
            .as_str()
        {
            "metered"    => IceProviderPreference::Metered,
            "cloudflare" => IceProviderPreference::Cloudflare,
            "coturn"     => IceProviderPreference::Coturn,
            "static"     => IceProviderPreference::Static,
            other => {
                if !other.is_empty() {
                    log::warn!("⚠️ ICE_PROVIDER={} non valido, uso auto", other);
                }
                IceProviderPreference::Auto
            }
        }
    }
}

/// Currently active provider, chosen based on env vars.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum IceProviderKind {
    /// Dynamic fetch from metered.ca REST API.
    Metered,
    /// Dynamic fetch from Cloudflare Worker TURN proxy.
    Cloudflare,
    /// Coturn REST schema with ephemeral HMAC-SHA1 credentials.
    CoturnRest,
    /// STUN only, no TURN (safe fallback).
    StaticOnly,
}

/// Result of the ICE servers fetch: provider used + unified config.
#[derive(Debug, Clone, Serialize)]
pub struct IceResolution {
    pub provider: IceProviderKind,
    pub config: IceLinkConfig,
    /// Any non-fatal error (e.g. metered failed, fallback to coturn/static).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,
    /// Entries returned by the metered provider (ignored if not metered).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metered_entries: Option<Vec<IceServerEntry>>,
}

/// Serializable entry for the browser. Compatible with
/// WebRTC `RTCIceServer` (see moz:// RTCIceServer).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IceServerEntry {
    #[serde(default, deserialize_with = "deserialize_urls")]
    pub urls: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credential: Option<String>,
}

fn deserialize_urls<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where D: serde::Deserializer<'de> {
    let v = serde_json::Value::deserialize(deserializer)?;
    match v {
        serde_json::Value::String(s) => Ok(vec![s]),
        serde_json::Value::Array(arr) => arr
            .into_iter()
            .map(|x| x.as_str().map(String::from).ok_or_else(|| serde::de::Error::custom("not a string")))
            .collect(),
        _ => Err(serde::de::Error::custom("urls must be string or array")),
    }
}

/// Select and use the appropriate provider. Never blocking: in case of error
/// on metered network, automatic fallback to coturn or static STUN.
pub async fn fetch_ice_servers(ttl_secs: Option<u64>) -> IceResolution {
    let preference = IceProviderPreference::from_env();
    log::info!("[ICE] Provider preference: {:?}", preference);

    match preference {
        IceProviderPreference::Static => {
            log::info!("[ICE] StaticOnly richiesto");
            IceResolution {
                provider: IceProviderKind::StaticOnly,
                config: ice_link_config_from_env(),
                warning: None,
                metered_entries: None,
            }
        }

        IceProviderPreference::Metered => {
            if let Ok(api_key) = std::env::var("METERED_API_KEY") {
                if !api_key.trim().is_empty() {
                    match fetch_metered(&api_key).await {
                        Ok(entries) => {
                            log::info!(
                                "✅ ICE servers fetched from metered.ca ({} entries)",
                                entries.len()
                            );
                            return IceResolution {
                                provider: IceProviderKind::Metered,
                                config: IceLinkConfig {
                                    signaling_url: std::env::var("SIGNALING_URL")
                                        .ok()
                                        .filter(|s| !s.is_empty()),
                                    stun_urls: vec![],
                                    turn: None,
                                },
                                warning: None,
                                metered_entries: Some(entries),
                            };
                        }
                        Err(e) => {
                            log::warn!(
                                "⚠️ metered.ca fetch failed: {}. Fallback a StaticOnly (NON Coturn).",
                                e
                            );
                            return IceResolution {
                                provider: IceProviderKind::StaticOnly,
                                config: ice_link_config_from_env(),
                                warning: Some(format!("metered_unavailable: {}", e)),
                                metered_entries: None,
                            };
                        }
                    }
                }
            }
            log::warn!(
                "⚠️ ICE_PROVIDER=metered ma METERED_API_KEY non impostato. Fallback a StaticOnly."
            );
            IceResolution {
                provider: IceProviderKind::StaticOnly,
                config: ice_link_config_from_env(),
                warning: Some("metered_required_but_missing".to_string()),
                metered_entries: None,
            }
        }

        IceProviderPreference::Cloudflare => {
            let endpoint = resolve_turn_credentials_endpoint();
            log::info!("[TURN] Using credentials endpoint: {}", endpoint);
            if !endpoint.trim().is_empty() {
                match fetch_cloudflare_turn(&endpoint).await {
                        Ok(entries) => {
                            log::info!(
                                "✅ ICE servers fetched from Cloudflare Worker ({} entries)",
                                entries.len()
                            );
                            return IceResolution {
                                provider: IceProviderKind::Cloudflare,
                                config: IceLinkConfig {
                                    signaling_url: std::env::var("SIGNALING_URL")
                                        .ok()
                                        .filter(|s| !s.is_empty()),
                                    stun_urls: vec![],
                                    turn: None,
                                },
                                warning: None,
                                metered_entries: Some(entries),
                            };
                        }
                        Err(e) => {
                            log::warn!(
                                "⚠️ Cloudflare Worker fetch failed: {}. Fallback a StaticOnly.",
                                e
                            );
                            return IceResolution {
                                provider: IceProviderKind::StaticOnly,
                                config: ice_link_config_from_env(),
                                warning: Some(format!("cloudflare_unavailable: {}", e)),
                                metered_entries: None,
                            };
                        }
                    }
                }
            log::warn!(
                "⚠️ ICE_PROVIDER=cloudflare ma TURN_CREDENTIALS_ENDPOINT non impostato. Fallback a StaticOnly."
            );
            IceResolution {
                provider: IceProviderKind::StaticOnly,
                config: ice_link_config_from_env(),
                warning: Some("cloudflare_required_but_missing".to_string()),
                metered_entries: None,
            }
        }

        IceProviderPreference::Coturn => {
            let cfg = if let Some(ttl) = ttl_secs {
                ice_link_config_from_env_with_ttl(ttl)
            } else {
                ice_link_config_from_env()
            };
            if cfg.turn.is_none() {
                log::warn!(
                    "⚠️ ICE_PROVIDER=coturn ma TURN_AUTH_SECRET o TURN_URLS non impostati. Fallback a StaticOnly."
                );
                return IceResolution {
                    provider: IceProviderKind::StaticOnly,
                    config: ice_link_config_from_env(),
                    warning: Some("coturn_required_but_missing".to_string()),
                    metered_entries: None,
                };
            }
            IceResolution {
                provider: IceProviderKind::CoturnRest,
                config: cfg,
                warning: None,
                metered_entries: None,
            }
        }

        IceProviderPreference::Auto => {
            log::warn!("ICE_PROVIDER non impostato, uso fallback auto");
            // Provider 1: metered.ca REST API (if METERED_API_KEY is present)
            if let Ok(api_key) = std::env::var("METERED_API_KEY") {
                if !api_key.trim().is_empty() {
                    match fetch_metered(&api_key).await {
                        Ok(entries) => {
                            log::info!(
                                "✅ ICE servers fetched from metered.ca ({} entries)",
                                entries.len()
                            );
                            return IceResolution {
                                provider: IceProviderKind::Metered,
                                config: IceLinkConfig {
                                    signaling_url: std::env::var("SIGNALING_URL")
                                        .ok()
                                        .filter(|s| !s.is_empty()),
                                    stun_urls: vec![],
                                    turn: None,
                                },
                                warning: None,
                                metered_entries: Some(entries),
                            };
                        }
                        Err(e) => {
                            log::warn!(
                                "⚠️ metered.ca fetch failed: {}. Falling back to coturn/static STUN.",
                                e
                            );
                            let cfg = if let Some(ttl) = ttl_secs {
                                ice_link_config_from_env_with_ttl(ttl)
                            } else {
                                ice_link_config_from_env()
                            };
                            let provider = if cfg.turn.is_some() {
                                IceProviderKind::CoturnRest
                            } else {
                                IceProviderKind::StaticOnly
                            };
                            return IceResolution {
                                provider,
                                config: cfg,
                                warning: Some(format!("metered_unavailable: {}", e)),
                                metered_entries: None,
                            };
                        }
                    }
                }
            }

            // Provider 2: Cloudflare Worker TURN proxy
            let endpoint = resolve_turn_credentials_endpoint();
            log::info!("[TURN] Using credentials endpoint: {}", endpoint);
            if !endpoint.trim().is_empty() {
                match fetch_cloudflare_turn(&endpoint).await {
                        Ok(entries) => {
                            log::info!(
                                "✅ ICE servers fetched from Cloudflare Worker ({} entries)",
                                entries.len()
                            );
                            return IceResolution {
                                provider: IceProviderKind::Cloudflare,
                                config: IceLinkConfig {
                                    signaling_url: std::env::var("SIGNALING_URL")
                                        .ok()
                                        .filter(|s| !s.is_empty()),
                                    stun_urls: vec![],
                                    turn: None,
                                },
                                warning: None,
                                metered_entries: Some(entries),
                            };
                        }
                        Err(e) => {
                            log::warn!(
                                "⚠️ Cloudflare Worker fetch failed: {}. Falling back to coturn/static STUN.",
                                e
                            );
                            let cfg = if let Some(ttl) = ttl_secs {
                                ice_link_config_from_env_with_ttl(ttl)
                            } else {
                                ice_link_config_from_env()
                            };
                            let provider = if cfg.turn.is_some() {
                                IceProviderKind::CoturnRest
                            } else {
                                IceProviderKind::StaticOnly
                            };
                            return IceResolution {
                                provider,
                                config: cfg,
                                warning: Some(format!("cloudflare_unavailable: {}", e)),
                                metered_entries: None,
                            };
                        }
                    }
                }
            // Provider 3: coturn REST (if TURN_AUTH_SECRET + TURN_URLS are present)
            let cfg = if let Some(ttl) = ttl_secs {
                ice_link_config_from_env_with_ttl(ttl)
            } else {
                ice_link_config_from_env()
            };
            let provider = if cfg.turn.is_some() {
                IceProviderKind::CoturnRest
            } else {
                IceProviderKind::StaticOnly
            };
            IceResolution {
                provider,
                config: cfg,
                warning: None,
                metered_entries: None,
            }
        }
    }
}

/// Builds the final `iceServers` array to pass to the browser, in compact
/// JSON format (`Vec<IceServerEntry>`) ready for `new RTCPeerConnection({iceServers})`.
pub fn build_browser_ice_servers(resolution: &IceResolution) -> Vec<IceServerEntry> {
    let mut out: Vec<IceServerEntry> = Vec::new();

    // 1) STUN entry. Use only the configured list; Cloudflare STUN is
    // already included by the active provider when applicable.
    let stun_urls: Vec<String> = resolution.config.stun_urls.clone();
    if !stun_urls.is_empty() {
        out.push(IceServerEntry {
            urls: stun_urls,
            username: None,
            credential: None,
        });
    }

    // 2) Metered/Cloudflare entries (STUN + TURN already paired with
    // credentials). Append and return.
    if let Some(extra) = &resolution.metered_entries {
        out.extend(extra.iter().cloned());
        return out;
    }

    // 3) Coturn TURN entry.
    if let Some(turn) = &resolution.config.turn {
        out.push(IceServerEntry {
            urls: turn.urls.clone(),
            username: Some(turn.username.clone()),
            credential: Some(turn.credential.clone()),
        });
    }

    out
}

/// Serializes `iceServers` for inclusion as a URL parameter.
/// Compact format: base64 URL-safe of `{"u":"<user>","c":"<cred>","s":["stun:..."],"t":["turn:..."]}`.
/// `u`/`c` are shared across all TURN entries; `s` = STUN list, `t` = TURN list.
pub fn encode_ice_servers_param(entries: &[IceServerEntry]) -> String {
    let mut stun_urls: Vec<String> = Vec::new();
    let mut turn_urls: Vec<String> = Vec::new();
    let mut username: Option<String> = None;
    let mut credential: Option<String> = None;

    for e in entries {
        if e.username.is_some() && e.credential.is_some() {
            turn_urls.extend(e.urls.iter().cloned());
            username = e.username.clone();
            credential = e.credential.clone();
        } else {
            stun_urls.extend(e.urls.iter().cloned());
        }
    }

    // Omit empty keys so the payload is compact. The receiver handles
    // missing keys gracefully (it only reads what it needs).
    let mut map = serde_json::Map::new();
    if let Some(u) = username {
        map.insert("u".into(), u.into());
    }
    if let Some(c) = credential {
        map.insert("c".into(), c.into());
    }
    if !stun_urls.is_empty() {
        map.insert("s".into(), stun_urls.into());
    }
    if !turn_urls.is_empty() {
        map.insert("t".into(), turn_urls.into());
    }
    let compact = serde_json::Value::Object(map);
    let json = compact.to_string();
    URL_SAFE_NO_PAD.encode(json.as_bytes())
}

async fn fetch_metered(api_key: &str) -> Result<Vec<IceServerEntry>, String> {
    let base = std::env::var("METERED_API_BASE")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| METERED_DEFAULT_BASE.to_string())
        .trim()
        .trim_end_matches('/')
        .to_string();

    if !base.starts_with("http://") && !base.starts_with("https://") {
        return Err(format!(
            "METERED_API_BASE malformata (manca http:// o https://): {:?}",
            base
        ));
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(FETCH_TIMEOUT_SECS))
        .build()
        .map_err(|e| format!("reqwest build error: {}", e))?;

    let resp = client
        .get(format!("{}/turn/credentials", base))
        .query(&[("apiKey", api_key.trim())])
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("metered API returned status {}", resp.status()));
    }

    let body_text = resp
        .text()
        .await
        .map_err(|e| format!("read body: {}", e))?;

    // Debug: log response body (first 200 chars) for diagnosis
    log::debug!("Metered response body (first 200 chars): {}",
        if body_text.len() > 200 { &body_text[..200] } else { &body_text });

    // The Metered API returns a top-level JSON array of IceServerEntry,
    // not an object with an "iceServers" field. Example:
    // [
    //   {"urls":"stun:stun.relay.metered.ca:80"},
    //   {"urls":"turn:global.relay.metered.ca:80","username":"...","credential":"..."},
    //   ...
    // ]
    let entries: Vec<IceServerEntry> = serde_json::from_str(&body_text)
        .map_err(|e| {
            let preview: String = body_text.chars().take(300).collect();
            format!("metered API: failed to parse ice servers array: {}. Body (first 300 chars): {}", e, preview)
        })?;

    if entries.is_empty() {
        return Err("metered API: empty ice servers array".to_string());
    }

    log::info!("[Metered] Parsed {} ice servers successfully", entries.len());
    Ok(entries)
}

/// Fetch TURN credentials from a Cloudflare Worker proxy.
///
/// The Worker is expected to return a JSON object with an `iceServers` field
/// containing an array of `IceServerEntry` (already in WebRTC standard format).
/// Example response:
///   {"iceServers":[{"urls":["turn:turn.cloudflare.com:3478"],"username":"...","credential":"..."}]}
async fn fetch_cloudflare_turn(endpoint: &str) -> Result<Vec<IceServerEntry>, String> {
    let url = endpoint.trim();
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err(format!(
            "TURN_CREDENTIALS_ENDPOINT malformato (manca http:// o https://): {:?}",
            url
        ));
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(FETCH_TIMEOUT_SECS))
        .build()
        .map_err(|e| format!("reqwest build error: {}", e))?;

    let resp = client
        .get(url)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!(
            "Cloudflare Worker returned status {}",
            resp.status()
        ));
    }

    let body_text = resp
        .text()
        .await
        .map_err(|e| format!("read body: {}", e))?;

    #[derive(Deserialize)]
    struct WorkerResponse {
        #[serde(rename = "iceServers")]
        ice_servers: Vec<IceServerEntry>,
    }

    let parsed: WorkerResponse = serde_json::from_str(&body_text).map_err(|e| {
        let preview: String = body_text.chars().take(300).collect();
        format!(
            "Cloudflare Worker: failed to parse response: {}. Body (first 300 chars): {}",
            e, preview
        )
    })?;

    if parsed.ice_servers.is_empty() {
        return Err("Cloudflare Worker returned empty iceServers array".to_string());
    }

    log::info!(
        "[Cloudflare TURN] Parsed {} ice servers",
        parsed.ice_servers.len()
    );
    Ok(parsed.ice_servers)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_format_is_compact() {
        use base64::{engine::general_purpose::STANDARD, Engine};

        let entries = vec![
            IceServerEntry {
                urls: vec!["turn:global.relay.metered.ca:80".to_string()],
                username: Some("user".to_string()),
                credential: Some("pass".to_string()),
            },
            IceServerEntry {
                urls: vec!["stun:stun.l.google.com:19302".to_string()],
                username: None,
                credential: None,
            },
        ];
        let encoded = encode_ice_servers_param(&entries);

        // URL_SAFE_NO_PAD → STANDARD per decodificare
        let std_b64 = encoded.replace("-", "+").replace("_", "/");
        let pad = std_b64.len() % 4;
        let std_b64 = if pad > 0 {
            format!("{}{}", std_b64, "=".repeat(4 - pad))
        } else { std_b64 };
        let json_bytes = STANDARD.decode(std_b64.as_bytes()).unwrap();
        let json_str = String::from_utf8(json_bytes).unwrap();
        let v: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        assert_eq!(v["u"], "user");
        assert_eq!(v["c"], "pass");
        assert_eq!(v["s"][0], "stun:stun.l.google.com:19302");
        assert_eq!(v["t"][0], "turn:global.relay.metered.ca:80");
    }
}
