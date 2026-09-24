# Future feature: offline LAN sharing

Status: **planned, not implemented**. Revisit after v1.1.0.

## Motivation

Peerino today requires an internet connection for every transfer, even
when both devices are on the same local network. This blocks a class of
use cases:

- Air-gapped networks (industrial, medical, defense)
- Off-grid locations (boats, cabins, construction sites)
- During internet outages (ISP down, mobile data exhausted)
- Privacy-focused setups where internet egress is undesirable
- Ad-hoc networks (two laptops connected directly via Ethernet cable,
  or via a phone hotspot with no cellular data)

These users need to share files locally without any external service.

## Why it doesn't work today

Three independent architectural dependencies require internet:

### 1. Signaling server (PeerJS cloud, 0.peerjs.com)

WebRTC requires an out-of-band channel to exchange SDP offers/answers
and ICE candidates before a peer connection can be established.
Peerino uses the public PeerJS cloud for this. Without internet, the
two peers never see each other's candidates, and no WebRTC connection
is created — even if both devices are on the same switch.

### 2. Receiver served from peerino.com

Share links point to `https://peerino.com/receiver?...`. Without
internet, the browser cannot load the receiver page at all, so the
user has nothing to interact with.

### 3. ICE provider fetch at link generation

When generating a link, the desktop app fetches ICE servers
(STUN/TURN credentials) from Cloudflare via a Worker. Without internet,
this fetch fails. The current code logs a warning and may still produce
a link, but the resulting `ice=` payload is empty or stale.

## What a proper implementation would look like

### Minimal viable feature

A separate UI action: **"Share on local network only"** (or "Offline
share"). It differs from the normal "Generate Link" in these ways:

1. **No ICE fetch.** The link is generated with no `ice=` parameter
   (or with an empty one). Only the LAN path will be used.

2. **No signaling.** The receiver connects directly to the desktop
   HTTP server at `http://<ip>:3000`, not via PeerJS.

3. **Link format:** `http://<ip>:3000/receiver?hash=<hash>&filename=<name>`
   — no `peerId=`, no `ice=`, no `turnMax=`, no `lan=` (it's the
   whole URL).

4. **Delivery of the URL:** the sender shows the URL prominently on
   screen, with a QR code for easy transfer to a phone. No clipboard
   sharing possible without another channel.

5. **The desktop HTTP server must already be running.** It should be
   started automatically before generating the offline link, and the
   UI should refuse to generate the link if the server is not up.

### Receiver side

The receiver page must detect that it's on a LAN HTTP origin and:

- Skip PeerJS initialization entirely
- Skip the "Detecting connection path..." state
- Go straight to "Download from local server" using the existing
  `/download/:hash` endpoint
- Handle the case where the sender is no longer reachable (server
  down) with a clear error, not a hang

This is mostly what `startDownloadFlow()` already does for the LAN
path, but it must become the *primary* flow when opened via LAN HTTP,
not a fallback that gets skipped.

### Security considerations

- The LAN HTTP server has no authentication (documented in the
  security audit, C-03). On an isolated network this is less of a
  concern, but on a shared network (café WiFi, hotel) any device can
  list and download files while the server is running.
- Mitigation options:
  - **Bind the server only to the specific LAN interface**, not 0.0.0.0
  - **Require a per-session token in the URL**: `http://<ip>:3000/s/<token>`
    where the token is generated per share and validated server-side
  - **Auto-shutdown the server** after N minutes of inactivity or after
    the share link is used once
- At minimum, the UI should warn: "This share is accessible to any
  device on this network."

### UX considerations

- The feature should be **opt-in**, not automatic. Users should not
  accidentally generate a non-encrypted link when they expect the
  normal flow.
- The label should be explicit: "Offline share (local network only)"
  or similar.
- The resulting page should be visually distinct (different badge,
  maybe a warning banner) so the user knows they're on a plain-HTTP,
  no-encryption path.
- The QR code generation should work offline. A small embedded
  library (like `qrcode.min.js`, ~10 KB) is enough.

## Test requirements

- Two devices on the same switch with WiFi/Ethernet, internet cable
  unplugged.
- One device runs Peerino, the other opens the URL.
- Verify: receiver loads, download completes, hash matches (if hash
  is included in the URL).
- Verify: no PeerJS connection attempt, no ICE fetch, no console
  errors related to network.
- Verify: server auto-shutdown works after the configured window.
- Verify: the shared token (if implemented) rejects unauthorized
  access.

## Estimated scope

- Rust: 40-80 lines (generate offline link, start server, token if
  added)
- TypeScript (app UI): 80-150 lines (new button, QR code, banner,
  server startup logic)
- Receiver (`receiver.html`): 60-100 lines (detect LAN origin,
  bypass PeerJS, adapt UI states)
- New asset: QR code library (~10 KB)
- Tests: unit + manual

Total: ~250-400 lines across three layers. Non-trivial but
contained.

## Alternatives considered

- **mDNS discovery** (Bonjour/Zeroconf): the sender advertises
  itself, the receiver browser finds it automatically without a
  manual URL. Browser support is inconsistent (Chrome has partial
  support behind flags; Firefox doesn't support it natively). Not
  viable today.
- **Direct Ethernet cable** between two machines: works with the
  same HTTP server, no changes needed beyond the current LAN path.
  Already possible manually today.
- **WebRTC without signaling via a shared QR code**: theoretically
  possible (offer SDP embedded in QR, answer via second QR), but
  extremely user-hostile. Not viable.
