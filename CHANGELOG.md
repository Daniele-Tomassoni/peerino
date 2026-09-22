# Changelog

All notable changes to Peerino are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.0.3] - 2026-09-22

### Changed
- Unified browser receiver: the LAN HTTP server now serves the same
  `peerino-website/receiver.html` used by peerino.com. Eliminates the
  drift between the two files that caused the LAN receiver to miss TURN
  support and several fixes (hash verification, cancellation, inactivity
  timeout, log masking).

### Removed
- `src/ui/web-receiver.html` (redundant; unified with the deployed receiver).

## [1.0.2] - 2026-09-22

### Fixed
- Async channel callback reordering under backpressure caused file
  corruption on TURN relay downloads. Downloads completed with the same
  file size but a different SHA-256 hash than the original. The channel
  callback is now serialized through a Promise queue, preserving the
  order of `conn.send()` calls even when multiple callbacks suspend on
  `bufferedamountlow`.

## [1.0.1] - 2026-09-22

### Added
- Hardcoded default TURN endpoint (`DEFAULT_TURN_CREDENTIALS_ENDPOINT`) so the
  released binary works out-of-the-box without a `.env` file
- SHA-256 integrity verification on browser receiver downloads
- Bundle configuration for MSI and NSIS installers
- `CONTRIBUTING.md` with setup, testing, and PR process
- `notes/turn-limiting-design.md` documenting the TURN rate limiting design
  (not implemented, informational)

### Changed
- `TURN_MAX_FILE_SIZE` default from 1024 bytes (bug) to 104,857,600 bytes (100 MB)
- Landing page redesign with 4 download CTAs, feature cards, How-it-works section
- Desktop layout optimized for 16:9 (1080px container)
- README terminology aligned with UI ("Generate Link", "Create Inbox")
- Privacy Policy and Terms updated to reflect Cloudflare TURN (was Metered)
- Node.js minimum version in CONTRIBUTING from 18 to 20

### Fixed
- Download button on landing page now points to `/releases/latest` instead of
  a pinned version
- `.status-box.info` CSS class missing in browser receiver
- `finalizeStarted` variable not declared (ReferenceError under strict mode)
- Deploy marker added to `receiver.html` for cache verification

### Removed
- (nothing)

## [1.0.0] - 2026-09-21

### Added

#### Core features
- P2P file sharing via link (browser receiver, no install required)
- Inbox mode: receive files from anyone via a link
- LAN fallback: direct transfer on the same network
- WebRTC P2P with STUN and TURN relay support
- Cloudflare TURN integration via Worker proxy (TURN key never in client)
- Automatic connection path detection (LAN → STUN → TURN)
- SHA-256 integrity verification on incoming transfers (browser → app)
- Recent transfers log (in-memory, 50 entries)
- Dark/blue themed UI with unified design system

#### Desktop app (Tauri)
- Windows installer (MSI + NSIS) and portable executable
- Persistent Peer ID (survives app restarts)
- Auto-reconnect to signaling server on connection loss
- Drag & drop file upload
- Shared folder management with sort and column layout
- Inbox links (local + internet)
- Local link generation (LAN-only HTTP)
- System tray / notification support
- Auto-restore connections after system hibernation

#### Browser receiver
- Direct download from link, no app required
- LAN fallback (HTTP) when available
- Progress bar with speed and ETA
- Connection path stepper (LAN/STUN/TURN)
- Turn bandwidth warning
- Collapsible technical log

### Changed
- Migrated from Metered TURN to Cloudflare TURN (1 TB/month free tier)
- `TURN_MAX_FILE_SIZE` default raised from 1 KB to 100 MB
- Link format compacted from ~840 to ~340 chars (-60%)
- ICE parameter now uses compact `{u,c,s,t}` format with base64 URL-safe
- `signal=` parameter omitted when using default PeerJS cloud
- Status messages moved from footer to header (fixed layout bug)
- Scrollbar styling: thin (6px) with theme color across all panels
- README updated with Cloudflare TURN instructions

### Fixed
- TURN credentials not loaded in dev mode (`.env` path resolution)
- `METERED_API_BASE` malformed URL caused reqwest builder error
- DataChannel was not reliable+ordered (PeerJS default), causing corrupted files and hangs at 99% on lossy networks
- Download receiver: path-detection gate added 1-5s latency for files <100MB
- Download receiver: stuck at "Receiving... X%" if sender stopped sending
- Inbox cancellation: temp file remained on disk after cancel
- Inbox cancellation: progress bar reappeared after cancel (residual chunks)
- Download P2P cancellation: app kept sending chunks after cancel
- Browser download cancellation: browser kept buffering after cancel
- `decoded ice` error when ICE parameter was URL-safe base64
- CTA animation triggered on page load instead of on reveal
- Status messages overlapped help button in header
- Footer status messages did not auto-dismiss (layout bug)
- Windows TaskDialogIndirect error (missing Common Controls v6 manifest)
- Dead code causing TS2367 in streamFileToConnection
- Dead code causing 'msg is not defined' error in website deploy
- Malformed `.gitignore` line for `.continue/` and `peerino-turn-proxy/`
- Metered API key reference removed from public documentation

### Security
- TURN key kept server-side in Cloudflare Worker, never in client binary
- Peer ID masked in user-visible logs (only first 12 chars shown)
- File hash masked in browser logs
- WebRTC DataChannel with DTLS encryption (self-signed certs)
- No file storage on any server: files are never stored remotely. When relayed via TURN, encrypted packets transit through Cloudflare (not stored, not decryptable).

### Performance
- Channel-based streaming for P2P download (replaces per-chunk IPC + base64)
- SHA-256 hash computed with `crypto.subtle` in browser (50-100x faster than js-sha256 when available)
- TURN credentials fetched from Cloudflare Worker (no client-side key)
- Rust `stream_file` uses `Channel<Vec<u8>>` with 64KB chunks and ack-based flow control (K=8 chunks per ack)
- Inactivity timeout for download receiver (30s) to free resources on stalled transfers

### Documentation
- README with features, installation, build instructions, configuration
- Privacy Policy and Terms of Service
- AGPLv3 license
- GitHub Actions CI/CD (frontend + backend + auto-deploy website)
- CI badge in README
- `.env.example` translated to English with `TURN_MAX_FILE_SIZE` documentation

### Known limitations
- Windows only (macOS and Linux in roadmap)
- No mobile app yet (roadmap)
- Files >200MB may fail on mobile browsers (memory limit)
- No code signing: Windows SmartScreen warning on first install
- HTTP (LAN) cancellation: browser relies on fetch failure (no explicit signal)