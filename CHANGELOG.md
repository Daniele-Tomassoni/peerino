# Changelog

All notable changes to Peerino are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed
- TURN relay is diagnostic-only: no file passes through Cloudflare
  TURN. When a direct connection isn't possible, Peerino shows a
  clear message asking to try a less restrictive network.
- Receiver now fetches TURN credentials from the Worker when the
  transfer starts, instead of embedding them in the share link.
  Share links are ~70% shorter and contain no credentials.
- TURN rate limit raised to 40 credentials/hour per IP.
- New Worker `peerino-turn-monitor` tracks monthly TURN egress
  and triggers a global shutdown flag when usage approaches 100 GB.
- TURN credential TTL set to 15 minutes.

### Removed
- File size cap for TURN transfers (no longer applicable).
- `ice=` and `turnMax=` parameters from new P2P share links.

## [1.0.10] - 2026-09-24

### Changed
- TURN file-size limit reduced from 10 MB to 5 MB. The relay is only
  used as a fallback when a direct connection isn't possible, and
  represents a small fraction of transfers. The lower limit protects
  the shared Cloudflare free-tier budget.
- Relay size-limit message rewritten in English with clearer guidance:
  retry first (direct connection is probabilistic), then local network,
  then different network, then smaller file.

### Infrastructure (not tracked in this repo)
- The Cloudflare Worker `peerino-turn-proxy` now includes:
  - IP-based rate limit (4 credentials per hour per IP) via Durable
    Object with SQLite-backed storage
  - Credential TTL reduced from 24h to 1h
  - `customIdentifier` field passed to Cloudflare TURN API for future
    usage analytics (currently hardcoded to "unknown"; will be populated
    in a follow-up release)

## [1.0.9] - 2026-09-24

### Fixed
- The frontend fallback for the TURN file-size limit was still set to
  100 MB in three places. If the backend call to `get_turn_limits`
  failed, the UI would use 100 MB while the backend enforced 10 MB.
  Now both use 10 MB consistently.

### Changed
- Share links no longer include the `lan=http://<ip>:3000` parameter.
  The parameter was blocked by mixed-content protection whenever the
  receiver was loaded over HTTPS from peerino.com, so it never fired
  in the normal flow. Opening the receiver directly from the LAN HTTP
  server (`http://<ip>:3000/receiver?...`) continues to work. Links
  are ~35-45 characters shorter.

### Documentation
- Updated stale "100 MB" comments in `turn_limits.rs`.
- Added `notes/future-offline-lan-sharing.md` describing a planned
  feature: file sharing on local networks without internet access.
  The document covers the three current architectural dependencies
  (signaling, receiver hosting, ICE fetch), a minimal implementation
  design, security and UX considerations, test requirements, and
  estimated scope (~250-400 lines). Not implemented.

## [1.0.8] - 2026-09-24

### Fixed
- The HTTP server now reads `HTTP_PORT` from the environment instead of
  always using port 3000, waits for the Axum task to shut down before
  returning from `stop_http_server`, and resets `server_running` when
  the task exits with an error. (C-18)
- File hashing during the startup scan now runs outside the SQLite lock.
  Previously, a large file could block `list_files` and other queries
  for the entire duration of the hash computation. (C-17)
- The cancellation flag registered by `stream_file` is now removed on
  every exit path (success, error, cancellation). Previously the flag
  could persist in the tracker after the stream ended. (C-11 follow-up)

### Documentation
- README: added a "How file size limits work" section explaining the
  10 MB TURN relay limit, why it exists, and how to bypass it by using
  a direct connection.
- Landing page: added a short note about the relay limit.
- `notes/known-limitations.md`: documented the LAN mixed-content issue
  and the shared cancellation flag limitation for concurrent downloads
  of the same hash.

## [1.0.7] - 2026-09-24

### Fixed
- The file scanner now propagates read errors instead of silently
  saving a partial hash for files that fail mid-read. (C-07 follow-up)
- The startup probe now waits for the async setup task to finish before
  reading the error state, eliminating a race where the UI could start
  before the writability check completed. (C-08 follow-up)
- The upload cleanup task releases the global lock before touching the
  filesystem, so new chunks arriving during cleanup are no longer
  blocked. (C-05 follow-up)
- Atomic renames now use an application-level lock on all platforms.
  On Unix, `rename()` silently overwrites an existing target, which
  could cause two concurrent uploads with the same filename to lose
  one file. The lock prevents this. (C-06 follow-up)

## [1.0.6] - 2026-09-23

### Fixed
- Abandoned P2P uploads are now cleaned up on connection close and
  after 5 minutes of inactivity. Previously, interrupted uploads left
  orphan temp files and file handles. (C-05)
- File writes are now atomic (temp file + rename). Partial files are
  removed on I/O error, and concurrent writes with the same filename
  no longer overwrite each other. (C-06)
- The file index now recalculates hashes when a file's size changes,
  removes records of files deleted from disk, and never caches stale
  entries. (C-07)
- Startup now surfaces an explicit error when the data folders
  (shared-folder, temp, config) are not writable, instead of silently
  falling back to in-memory storage. (C-08)
- Unicode filenames no longer panic the HTTP server when building the
  Content-Disposition header. RFC 5987 `filename*` is used for
  non-ASCII names. (C-13)
- Expired inboxes are rejected immediately by `get_inbox`, instead of
  remaining valid until the next hourly cleanup. (C-14)
- The LAN receiver no longer buffers the entire file in JavaScript
  memory. LAN downloads now use the browser's native streaming via
  redirect. (C-15)
- PeerJS and js-sha256 are now self-hosted instead of loaded from
  unpkg, removing a runtime CDN dependency. (C-19)

## [1.0.5] - 2026-09-23

### Security
- Require a valid, non-expired inbox_id for P2P uploads. Previously,
  anyone with a PeerID from any shared link could push files into the
  shared folder without the inbox link. (C-01)
- Remove the public `/files` HTTP endpoint. The endpoint exposed name,
  size, and hash of every shared file to any device on the local
  network. The server now uses a strict CORS allow-list. (C-03)
- Reject drive-relative and Windows device filenames (e.g. `C:evil.txt`,
  `CON`, `LPT1`) in uploaded files. Previously such names could escape
  the shared folder on Windows. (C-02)
- Enforce byte limits on incoming transfers: HTTP uploads are capped at
  1 GB (configurable via `HTTP_UPLOAD_MAX_SIZE`), P2P uploads are capped
  at the declared size plus 1 KB. Finalization now rejects size
  mismatches. (C-04)
- Check `conn.open` before sending chunks and abort on connection
  close/error. Previously the sender could report "File sent" even when
  the channel was already closed. (C-12)

### Changed
- Remove regional STUN servers (miwifi, bilibili, yandex). They provided
  no practical benefit: Cloudflare TURN is throttled in Russia and
  unreliable in China, so relayed transfers don't work there anyway.
  Share links are ~80 characters shorter.
- Reduce the TURN relay per-file limit from 100 MB to 10 MB, with a
  clearer user message showing the file size, the limit, the reason,
  and concrete steps to fix (connect both devices to the same network).
- Always include `turnMax` in share links. The v1.0.4 optimization of
  omitting it when equal to the default is removed, to avoid drift
  between app and browser defaults.

## [1.0.4] - 2026-09-23

### Fixed
- Regional STUN servers (China/Russia) were silently dropped when the
  active TURN provider was Cloudflare, because `build_browser_ice_servers`
  returned early before appending the default STUN list. Users behind
  blocked Google/Cloudflare STUN had no reachable fallback.
- Inbox cancellation was not notified to the browser sender. When the
  app cancelled an incoming upload, the browser kept showing
  "Sending file..." with no indication the transfer had stopped. The app
  now sends `transfer_cancelled` to the browser, which resets the UI and
  stops the send loop.

### Changed
- Share links are shorter: `turnMax` is omitted when it equals the
  receiver's default (100 MB), and empty `ice` fields are no longer
  serialized.

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