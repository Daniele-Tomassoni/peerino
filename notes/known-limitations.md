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
- All internet shares between different networks go through the TURN
  relay when a direct STUN connection isn't possible, and are subject to
  the 10 MB per-file relay limit.

Possible future fixes (not scheduled):
- Serve the LAN receiver over HTTPS with a self-signed certificate and
  a clear browser-warning flow.
- Or remove the LAN path entirely and rely on WebRTC only.

Status: documented, not fixed. Revisit in v1.1.0.
