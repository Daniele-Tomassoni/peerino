# Peerino

**P2P file sharing senza cloud, senza account, senza intermediari.**

Peerino è un'applicazione desktop open source per la condivisione di file
peer-to-peer. Invia un file a chiunque tramite un semplice link — il
destinatario non ha bisogno di installare nulla. Oppure ricevi file da
chiunque, direttamente sul tuo computer.

---

## ✨ Caratteristiche

- 🔗 **Condivisione tramite link**: genera un link, condividilo, il
  destinatario scarica il file dal browser
- 📥 **Inbox**: ricevi file da chiunque tramite un link, senza che debbano
  installare Peerino
- 🔒 **Privacy**: connessione WebRTC crittografata end-to-end (DTLS)
- 🏠 **Fallback LAN**: trasferimento diretto nella stessa rete, senza
  passare da Internet
- ⚡ **WebRTC P2P**: connessione diretta tra peer quando possibile
- ✅ **Verifica integrità**: hash SHA-256 su ogni file
- 🆓 **Open source**: licenza MIT, gratuito, senza account
- 💻 **Solo Windows** (per ora; macOS e Linux in roadmap)

---

## 🚀 Come funziona

### Inviare un file a chi non ha Peerino

1. Apri Peerino
2. Seleziona il file che vuoi condividere
3. Clicca **Share** — il link viene copiato automaticamente
4. Incollalo in un'email, chat, o qualsiasi altro canale
5. Il destinatario apre il link nel browser e scarica il file

### Ricevere un file da chi non ha Peerino

1. Apri Peerino
2. Clicca **Inbox** — viene generato un link
3. Copia il link e invialo a chi vuole mandarti un file
4. Il destinatario apre il link nel browser e carica il file
5. Il file arriva direttamente nel tuo computer

---

## 📦 Installazione

### Requisiti

- Windows 10 o 11 (64-bit)
- WebView2 (preinstallato su Windows 10/11 recenti)

### Download

Scarica l'ultima versione da [GitHub Releases](https://github.com/your-username/peerino/releases).

1. Scarica il file `.msi` o `.exe`
2. Esegui l'installer
3. Avvia Peerino dal menu Start

---

## 🛠️ Build da sorgente

### Prerequisiti

- [Node.js](https://nodejs.org/) (LTS)
- [Rust](https://www.rust-lang.org/tools/install) (toolchain stabile)
- [Tauri CLI](https://tauri.app/v1/guides/getting-started/prerequisites)

### Comandi

```bash
# Installa le dipendenze
npm install

# Avvia in modalità sviluppo (hot reload)
npm run tauri dev

# Build di produzione (genera installer)
npm run tauri build
```

L'installer viene generato in `src-tauri/target/release/bundle/`.

---

## ⚙️ Configurazione

Copia `.env.example` in `.env` e compila le variabili:

```bash
cp .env.example .env
```

### Variabili

| Variabile | Descrizione | Default |
|-----------|-------------|---------|
| `TURN_URLS` | URL del server TURN (Coturn) | `turn:peerino.com:3478` |
| `TURN_AUTH_SECRET` | Secret condiviso con Coturn (schema REST) | *(vuoto fino a configurazione)* |
| `TURN_CRED_TTL_SECS` | TTL credenziali effimere | `7200` (2 ore) |
| `STUN_URLS` | Lista server STUN (separati da virgola) | Xiaomi, Bilibili, Yandex, Google, Cloudflare |
| `SIGNALING_URL` | URL del server di signaling | `0.peerjs.com` |
| `P2P_WEB_URL` | URL della pagina web per il browser destinatario | `https://peerino.com/receiver` |
| `HTTP_PORT` | Porta del server HTTP locale | `3000` |
| `TURN_MAX_FILE_SIZE` | Limite dimensione file su TURN (byte) | `104857600` (100 MB) |

**Nota**: `TURN_AUTH_SECRET` va popolato solo dopo aver configurato Coturn
sul VPS. Senza questo valore, Peerino usa solo STUN (funziona nella
maggior parte dei casi ma non su NAT simmetrici).

---

## 🔐 Privacy e sicurezza

- **Crittografia end-to-end**: tutte le connessioni WebRTC usano DTLS
- **Nessun server centrale**: i file passano direttamente tra peer
- **Nessun account**: nessuna registrazione, nessun login
- **Metadati minimi**: nome file e hash sono nel link (chi ha il link può
  scaricare il file)
- **Scadenza link**: i link scadono dopo 24 ore
- **Limite download**: ogni link ha un numero massimo di download
- **Limite TURN**: 100 MB per proteggere la banda del relay TURN condiviso

---

## ⚠️ Limitazioni attuali

- **Solo Windows**: macOS e Linux in roadmap
- **App mobile**: in roadmap
- **Sistema crediti**: in roadmap
- **DHT**: scoperta peer decentralizzata in roadmap
- **Nessuna autenticazione**: chi ha il link può scaricare il file

---

## 🤝 Contribuire

Peerino è open source e i contributi sono benvenuti.

1. Fai un fork del repository
2. Crea un branch per la tua feature (`git checkout -b feature/amazing`)
3. Committa le modifiche (`git commit -m 'Add amazing feature'`)
4. Push sul branch (`git push origin feature/amazing`)
5. Apri una Pull Request

Per bug e richieste, apri una [Issue](https://github.com/your-username/peerino/issues).

---

## 📄 Licenza

Distribuito sotto licenza **MIT**. Vedi [`LICENSE`](LICENSE) per i dettagli.

---

## 📬 Contatti

- **Email**: support@peerino.com
- **GitHub**: [github.com/your-username/peerino](https://github.com/your-username/peerino)
- **Issues**: [github.com/your-username/peerino/issues](https://github.com/your-username/peerino/issues)

---

**Peerino** — Condividi file. Senza cloud. Senza account. Senza intermediari.