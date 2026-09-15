# Peerino

**P2P file sharing without cloud, without accounts, without intermediaries.**

Peerino is an open-source desktop application for peer-to-peer file sharing. Send a file to anyone via a simple link — the recipient doesn't need to install anything. Or receive files from anyone, directly on your computer.

---

## ✨ Features

- 🔗 **Share via link**: generate a link, share it, and the recipient downloads the file from their browser
- 📥 **Inbox**: receive files from anyone via a link, without them needing to install Peerino
- 🔒 **Privacy**: end-to-end encrypted WebRTC connections (DTLS)
- 🏠 **LAN fallback**: direct transfer on the same network, without going through the internet
- ⚡ **WebRTC P2P**: direct peer-to-peer connection when possible
- ✅ **Integrity verification**: SHA-256 hash on every file
- 🆓 **Open source**: AGPLv3 licensed, free, no account required
- 💻 **Windows only** (for now; macOS and Linux in roadmap)

---

## 🚀 How it works

### Send a file to someone who doesn't have Peerino

1. Open Peerino
2. Select the file you want to share
3. Click **Share** — the link is copied automatically
4. Paste it in an email, chat, or any other channel
5. The recipient opens the link in their browser and downloads the file

### Receive a file from someone who doesn't have Peerino

1. Open Peerino
2. Click **Inbox** — a link is generated
3. Copy the link and send it to whoever wants to send you a file
4. The recipient opens the link in their browser and uploads the file
5. The file arrives directly on your computer

---

## 📦 Installation

### Requirements

- Windows 10 or 11 (64-bit)
- WebView2 (preinstalled on recent Windows 10/11)

### Download

Download the latest version from [GitHub Releases](https://github.com/your-username/peerino/releases).

1. Download the `.msi` or `.exe` file
2. Run the installer
3. Launch Peerino from the Start menu

---

## 🛠️ Build from source

### Prerequisites

- [Node.js](https://nodejs.org/) (LTS)
- [Rust](https://www.rust-lang.org/tools/install) (stable toolchain)
- [Tauri CLI](https://tauri.app/v2/guides/getting-started/prerequisites)

### Commands

```bash
# Install dependencies
npm install

# Start in development mode (hot reload)
npm run tauri dev

# Production build (generates installer)
npm run tauri build
```

The installer is generated in `src-tauri/target/release/bundle/`.

### ⚠️ Windows: Race condition with Windows Defender

On Windows, `npm run tauri dev` can fail with `STATUS_ENTRYPOINT_NOT_FOUND` due to a race condition with Windows Defender. When `cargo run` compiles and immediately executes the binary, Defender can block it during scanning.

**Solutions**:

**Option 1 — Defender exclusion (recommended)**:

```powershell
Add-MpPreference -ExclusionPath "$PWD\src-tauri\target"
```

**Option 2 — Manual development script**:

```powershell
.\dev.ps1
```

The script starts Vite, waits, compiles the Rust backend, waits for Defender's scan, then launches the executable.

---

## ⚙️ Configuration

Copy `.env.example` to `.env` and fill in the variables:

```bash
cp .env.example .env
```

### Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `TURN_URLS` | TURN server URL (Coturn) | `turn:peerino.com:3478` |
| `TURN_AUTH_SECRET` | Shared secret with Coturn (REST scheme) | *(empty until configured)* |
| `TURN_CRED_TTL_SECS` | Ephemeral credential TTL | `7200` (2 hours) |
| `STUN_URLS` | Comma-separated list of STUN servers | Xiaomi, Bilibili, Yandex, Google, Cloudflare |
| `SIGNALING_URL` | Signaling server URL | `0.peerjs.com` |
| `P2P_WEB_URL` | Web page URL for the browser recipient | `https://peerino.com/receiver` |
| `HTTP_PORT` | Local HTTP server port | `3000` |
| `TURN_MAX_FILE_SIZE` | Max file size over TURN (bytes) | `104857600` (100 MB) |

**Note**: `TURN_AUTH_SECRET` only needs to be set after configuring Coturn on your VPS. Without this value, Peerino uses STUN only (works in most cases but not on symmetric NAT).

---

## 🔐 Privacy & Security

- **End-to-end encryption**: all WebRTC connections use DTLS
- **No central server**: files pass directly between peers
- **No account**: no registration, no login
- **Minimal metadata**: filename and hash are in the link (whoever has the link can download the file)
- **Link expiration**: links expire after 24 hours
- **Download limit**: each link has a maximum number of downloads
- **TURN limit**: 100 MB to protect shared TURN relay bandwidth

---

## ⚠️ Current Limitations

- **Windows only**: macOS and Linux in roadmap
- **Mobile app**: in roadmap
- **Credit system**: in roadmap
- **DHT**: decentralized peer discovery in roadmap
- **No authentication**: anyone with the link can download the file

---

## 🤝 Contributing

Peerino is open source and contributions are welcome.

1. Fork the repository
2. Create a branch for your feature (`git checkout -b feature/amazing`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing`)
5. Open a Pull Request

For bugs and requests, open an [Issue](https://github.com/your-username/peerino/issues).

---

## 📄 License

Distributed under the **GNU Affero General Public License v3.0 (AGPLv3)**.

This project is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License, version 3, as published by the Free Software Foundation.

See [`LICENSE`](LICENSE) for the full text.

**Note**: if you modify Peerino and offer it as a network service, you must release the source code of your modifications under the same license (this is the main requirement of AGPLv3).

---

## 📬 Contact

- **Email**: support@peerino.com
- **GitHub**: [github.com/your-username/peerino](https://github.com/your-username/peerino)
- **Issues**: [github.com/your-username/peerino/issues](https://github.com/your-username/peerino/issues)

---

**Peerino** — Share files. Without cloud. Without accounts. Without intermediaries.