# P2P File Sharing App - Project Context
## Versione 3.2.2 — Aggiornata 2026-09-10

---

## 1. Panorama

**Nome**: Peerino — P2P File Sharing App  
**Stack**: Tauri 2.0 (Rust backend) + TypeScript Vanilla (frontend) + PeerJS (WebRTC)  
**Stato**: Fase 4 completata, Fase 5-6 pianificate  

### 4 Funzioni Principali
| Modalità | Funzione | Tecnologia |
|----------|----------|------------|
| Locale | Download link | `http://{IP}:3000/download/{hash}` |
| Locale | Inbox upload | `http://{IP}:3000/inbox/{id}` |
| Internet | P2P-to-Web download | Netlify + PeerJS |
| Internet | Inbox upload WebRTC | Netlify + PeerJS |

---

## 2. Architettura

### Componenti
- **Backend Rust** (`src-tauri/src/`): comandi Tauri, server HTTP (axum, porta 3000), database SQLite (rusqlite, WAL mode)
- **Frontend TypeScript** (`src/ui/`): app desktop (`app.ts`), pagina browser (`web-receiver.html`)
- **Deploy**: `netlify-deploy/index.html` (identico a `web-receiver.html`)
- **Signaling**: PeerJS cloud (0.peerjs.com) o self-hosted (SIGNALING_URL)

### Flusso di Connessione (Cascata)
1. **LAN** (host+host) → LED VERDE → HTTP diretto
2. **STUN** (srflx/prflx) → LED VERDE → WebRTC P2P
3. **TURN relay** (relay) → LED GIALLO (≤100MB) o ROSSO (>100MB) → blocco se >limite

---

## 3. Sistema di Limite TURN (v3.2.2)

### Panorama
Limite di dimensione file per connessioni TURN relay. Default **100 MB**, configurabile via env `TURN_MAX_FILE_SIZE`. Coerenza browser/backend tramite `&turnMax=` nei link.

### Dove è applicato
| Flusso | File | Meccanismo |
|--------|------|------------|
| Download P2P (app→browser) | `app.ts:1141-1217` | Retry loop 10 tentativi su `detectConnectionPath` |
| Upload inbox (browser→app) | `app.ts:1506-1543` | Polling unificato + check `detectedPath2` |
| Difesa backend | `init_incoming_upload.rs:29` | Parametro `path` opzionale (turn/direct) |
| Browser | `web-receiver.html:609-670` | `effectiveSize` con fallback a globale |

### Coerenza browser/backend
- **Backend**: `&turnMax={valore}` nei link (`generate_web_link.rs:142-150`, `create_inbox.rs:84-92`)
- **Browser**: legge `turnMax` dal URL, fallback a `104857600` (`web-receiver.html:521-533`)
- **App desktop**: cache `__turnMaxFileSize` caricata all'avvio (`app.ts:2237-2238`)

### Fix Completati (11/11)
| # | Severità | Problema | Soluzione |
|---|----------|----------|-----------|
| #1 | CRITICO | `detectConnectionPath` chiamata una sola volta | Loop 10 tentativi (1s intervallo) |
| #1b | CRITICO | Loop esaurito senza risultato | Default a `'turn'` (massima prudenza) |
| #2 | CRITICO | `detectAndMarkPath` ignora secondo argomento | `effectiveSize` con fallback |
| #3 | CRITICO | `init_incoming_upload` senza `path` | `path` passato dal frontend |
| #4 | ALTO | Doppio polling parallelo | Unificato in un solo polling |
| #5 | ALTO | Limite hardcoded nel browser | `&turnMax=` nel link |
| #5b | MEDIO | Doppio `upload_complete` | Flag `uploadCompleteSent` + helper |
| #6 | MEDIO | Delete prima di finalize | Spostata dopo `finalizeIncomingUpload` |
| #11 | BASSO | IPC ridondanti | Cache `__turnMaxFileSize` |

### File Chiave
- `src/ui/app.ts` — Logica frontend (streamFileToConnection, processIncomingMessage, auto-finalize)
- `src/ui/web-receiver.html` — Pagina browser (detectAndMarkPath, getTurnSizeLimitMessage)
- `netlify-deploy/index.html` — Versione deploy della pagina browser
- `src-tauri/src/commands/p2p/generate_web_link.rs` — Generazione link download
- `src-tauri/src/commands/p2p/create_inbox.rs` — Generazione link inbox
- `src-tauri/src/commands/p2p/init_incoming_upload.rs` — Difesa backend (path opzionale)
- `src-tauri/src/commands/turn_limits.rs` — Comandi Tauri per il limite
- `src-tauri/src/commands/mod.rs` — Funzioni centralizzate `get_turn_max_file_size()`

---

## 4. Prossimi Passi Consigliati

1. **Fix #8 (MEDIO)**: base64 → Tauri Channels in `p2p_file_transfer.rs` (rimandare a release futura)
2. **Test manuale**: matrice di 6 scenari (LAN, STUN, TURN con file piccoli/grandi)
3. **Commit**: `fix: resolve 11 critical/high/medium TURN limit bugs`

---

## 5. Regole Progetto (da .roo/rules/rules.md)

- Configurazioni sicurezza in `src-tauri/capabilities/default.json` (NON in `tauri.conf.json`)
- Niente `Vec<u8>` per gestione file → streaming asincrono con `tokio::fs` e buffer 64KB
- Database SQLite in `tokio::task::spawn_blocking`
- Modalità WAL per database
- Crediti come numeri interi (1 credito = 100 punti)
- Selezione file con `@tauri-apps/plugin-dialog`
- Codifica base64 con `base64::{engine::general_purpose::STANDARD, Engine}`
- Streaming file grandi con buffer limitati (`tokio::sync::mpsc::channel(32)`)
