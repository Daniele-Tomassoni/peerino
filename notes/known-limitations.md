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
