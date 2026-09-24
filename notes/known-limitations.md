# Known Limitations

## Self-hosted signaling and dynamic ICE (C-10 from security audit)

The backend can generate share links with a custom `SIGNALING_URL` and
dynamic ICE servers (Cloudflare, coturn, Metered). The desktop app,
however, always uses `0.peerjs.com` and Google STUN for its own
PeerJS connection.

Consequence: if you configure a self-hosted signaling server, the
browser will use it but the desktop will not, causing connection
failures.

Additionally, the CSP in `tauri.conf.json` allows only `wss://*.peerjs.com`
and `wss://*.metered.ca`. A custom signaling host requires editing the
CSP and rebuilding the app.

Status: not fixed. Planned for v1.1.0 when self-hosted deployments
become a real use case.

## LAN fallback is not usable from peerino.com (mixed-content blocking)

The receiver is served over HTTPS from `peerino.com`. Share links include
a `lan=http://<ip>:3000` parameter pointing to the desktop's local HTTP
server.

When the receiver is opened from `https://peerino.com`, the browser blocks
the HTTP request to the LAN server as mixed content. The LAN transfer path
therefore never fires in the normal flow, and the transfer falls through
to WebRTC (STUN or TURN).

Consequences:
- The LAN path only works when the receiver page is served directly from
  the LAN HTTP server (`http://<ip>:3000`), which the user must open
  manually.
- TURN is used only as a diagnostic signal when a direct STUN/WebRTC
  connection is not possible. Peerino does not transfer files through TURN.
  If the direct path is unavailable, the user sees a message asking them
  to try from a different network.

Possible future fixes (not scheduled):
- Serve the LAN receiver over HTTPS with a self-signed certificate and
  a clear browser-warning flow.
- Or remove the LAN path entirely and rely on WebRTC only.

Status: documented, not fixed. Revisit in v1.1.0.

## Cancellation flag shared across concurrent downloads of the same hash

The download tracker keys cancellation flags by file hash. If two
downloads of the same file run concurrently, cancelling one cancels both.

Rare edge case (same file, two receivers, simultaneous cancel). Not
fixed. The uniform cleanup added in v1.0.8 prevents the flag from
persisting after a transfer ends.

Status: documented, not fixed. Full fix would require a per-transfer
UUID instead of the hash.
