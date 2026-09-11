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
// ICE servers provider unificato: astrazione sopra più fonti TURN/STUN.
// Permette di switchare provider senza modificare il codice frontend:
//
// - **Metered.ca REST API**: fetch dinamico di iceServers via API key.
//   Transient: da usare finché non si ha un VPS self-hosted.
//   Env: `METERED_API_KEY` (obbligatoria), `METERED_API_BASE` (opzionale,
//   default `https://peerino.metered.live/api/v1`).
//
// - **Coturn REST (HMAC-SHA1)**: credenziali effimere calcolate localmente.
//   Self-hosted: la transizione finale. Richiede coturn con
//   `use-auth-secret` + `static-auth-secret`.
//   Env: `TURN_AUTH_SECRET` + `TURN_URLS` + opzionale `TURN_CRED_TTL_SECS`.
//
// - **Static STUN only**: fallback sicuro. Nessun TURN.
//   Env: solo `STUN_URLS` (opzionale, default Google public).
//
// Il metodo `fetch_ice_servers` seleziona automaticamente il provider in base
// alle env disponibili, con priorità metered > coturn > static.

use crate::utils::turn_creds::{
    ice_link_config_from_env, ice_link_config_from_env_with_ttl, IceLinkConfig,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use serde::{Deserialize, Serialize};
use std::time::Duration;

const METERED_DEFAULT_BASE: &str = "https://peerino.metered.live/api/v1";
const FETCH_TIMEOUT_SECS: u64 = 10;

/// Currently active provider, chosen based on env vars.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum IceProviderKind {
    /// Dynamic fetch from metered.ca REST API.
    Metered,
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
    #[serde(rename = "urls")]
    pub urls: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credential: Option<String>,
}

/// JSON response from the metered API (documented format).
#[derive(Debug, Deserialize)]
struct MeteredResponse {
    #[serde(default)]
    ice_servers: Option<Vec<IceServerEntry>>,
}

/// Select and use the appropriate provider. Never blocking: in case of error
/// on metered network, automatic fallback to coturn or static STUN.
pub async fn fetch_ice_servers(ttl_secs: Option<u64>) -> IceResolution {
    // Provider 1: metered.ca REST API (se METERED_API_KEY è presente)
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
                            // I campi turn non sono usati dal browser quando
                            // arrivano via metered_entries; lasciamo placeholder.
                            turn: None,
                        },
                        warning: None,
                        metered_entries: Some(entries),
                    };
                }
                Err(e) => {
                    log::warn!(
                        "⚠️ metered.ca fetch fallito: {}. Fallback a coturn/static STUN.",
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

    // Provider 2: coturn REST (se TURN_AUTH_SECRET + TURN_URLS sono presenti)
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

/// Costruisce l'array `iceServers` finale da passare al browser, in formato
/// JSON compatto (`Vec<IceServerEntry>`) pronto per `new RTCPeerConnection({iceServers})`.
pub fn build_browser_ice_servers(resolution: &IceResolution) -> Vec<IceServerEntry> {
    let mut out: Vec<IceServerEntry> = Vec::new();

    // 1) Se metered: usa le entries ritornate dalla REST API (già pronte).
    if let Some(extra) = &resolution.metered_entries {
        out.extend(extra.iter().cloned());
        return out;
    }

    // 2) Altrimenti: STUN da env (o default Google).
    if !resolution.config.stun_urls.is_empty() {
        out.push(IceServerEntry {
            urls: resolution.config.stun_urls.clone(),
            username: None,
            credential: None,
        });
    }

    // 3) TURN (coturn effimere): un'unica entry con urls/username/credential.
    if let Some(turn) = &resolution.config.turn {
        out.push(IceServerEntry {
            urls: turn.urls.clone(),
            username: Some(turn.username.clone()),
            credential: Some(turn.credential.clone()),
        });
    }

    out
}

/// Serializza `iceServers` per inclusione come parametro URL (base64 di JSON).
pub fn encode_ice_servers_param(entries: &[IceServerEntry]) -> String {
    let json = serde_json::to_string(entries).unwrap_or_else(|_| "[]".to_string());
    BASE64.encode(json.as_bytes())
}

/// Decodifica l'array `iceServers` dal parametro URL (lato browser o test).
pub fn decode_ice_servers_param(encoded: &str) -> Vec<IceServerEntry> {
    let bytes = match BASE64.decode(encoded.as_bytes()) {
        Ok(b) => b,
        Err(_) => return vec![],
    };
    serde_json::from_slice::<Vec<IceServerEntry>>(&bytes).unwrap_or_default()
}

async fn fetch_metered(api_key: &str) -> Result<Vec<IceServerEntry>, String> {
    let base = std::env::var("METERED_API_BASE")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| METERED_DEFAULT_BASE.to_string());
    let url = format!("{}/turn/credentials?apiKey={}", base, api_key);

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(FETCH_TIMEOUT_SECS))
        .build()
        .map_err(|e| format!("reqwest build error: {}", e))?;

    let resp = client
        .get(&url)
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

    // La risposta metered può avere due forme:
    //   A) { "iceServers": [...] }
    //   B) [...]  (array diretto, documentazione alternativa)
    if let Ok(parsed) = serde_json::from_str::<MeteredResponse>(&body_text) {
        if let Some(entries) = parsed.ice_servers {
            if !entries.is_empty() {
                return Ok(entries);
            }
        }
    }
    if let Ok(entries) = serde_json::from_str::<Vec<IceServerEntry>>(&body_text) {
        if !entries.is_empty() {
            return Ok(entries);
        }
    }

    // Diagnostics: the response does not contain a valid ice servers array.
    // Log the body (first 500 chars) to aid debugging (e.g. free plan
    // metered returns an error message, not an array).
    let preview: String = body_text.chars().take(500).collect();
    log::warn!("metered API: invalid response (body preview): {}", preview);
    Err(format!(
        "metered API: response does not contain ice servers. Body: {}",
        preview
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_decode_roundtrip() {
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
        let decoded = decode_ice_servers_param(&encoded);
        assert_eq!(decoded.len(), 2);
        assert_eq!(decoded[0].username.as_deref(), Some("user"));
    }

    #[test]
    fn decode_invalid_returns_empty() {
        let decoded = decode_ice_servers_param("not_base64!!!");
        assert!(decoded.is_empty());
    }
}
