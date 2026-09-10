# CHECKPOINT — Fix Limite TURN (2026-09-10)

> Data: 2026-09-10T08:48:00Z
> Agente: Zoo
> Scopo: snapshot dello stato del codice DOPO tutti i fix del limite TURN.

## Fase 1: Fix Critici (completati)

### Fix #1 (CRITICO) — Retry loop in streamFileToConnection
- **File**: `src/ui/app.ts` righe 1141-1217
- **Problema**: `detectConnectionPath` chiamata una sola volta → se ritorna `null`, il limite non viene applicato.
- **Fix**: Loop `for` fino a 10 tentativi (1s intervallo).

### Fix #1b (CRITICO) — Loop esaurito senza risultato: default a TURN
- **File**: `src/ui/app.ts` righe 1163-1167
- **Problema**: Se il loop si esaurisce senza determinare il path, il codice procedeva senza applicare il limite.
- **Fix**: In caso di esaurimento, `detectedPath` viene imposto a `'turn'` (massima prudenza).

### Fix #2 (CRITICO) — effectiveSize in detectAndMarkPath
- **File**: `src/ui/web-receiver.html` righe 609-670
- **Problema**: Il secondo argomento `fileSizeOverride` era accettato ma ignorato; la variabile globale `fileSize` in modalità upload non viene mai impostata.
- **Fix**: Introdotta variabile `effectiveSize` che preferisce il argomento passato con fallback alla globale.

### Fix #3 (CRITICO) — path a init_incoming_upload
- **File**: `src/ui/app.ts` righe 1606-1610
- **Problema**: L'invoke non passava il parametro `path` → nel backend `is_turn` era sempre `false`.
- **Fix**: Aggiunto `path: detectedPath2 === 'turn' ? 'turn' : 'direct'`.

### Fix #4 (ALTO) — Polling unificato
- **File**: `src/ui/app.ts` righe 1506-1543
- **Problema**: Due polling paralleli sulla stessa connessione → race condition su `connectionPaths`.
- **Fix**: Unificati in un unico polling che condivide `detectedPath2`.

### Fix #5 (MEDIO) — Race condition upload_end
- **File**: `src/ui/app.ts` righe 1658-1667
- **Problema**: Se `upload_end` arrivava prima dell'auto-finalizzazione, nessuna risposta al browser.
- **Fix**: Aggiunto `else` branch con log dell'attesa.

### Fix #5b (MEDIO) — Doppio upload_complete (idempotenza)
- **File**: `src/ui/app.ts` righe 181-202, 1452, 1662
- **Problema**: L'handler `upload_end` e l'auto-finalizzazione potevano inviare entrambi `upload_complete`.
- **Fix**: Campo `uploadCompleteSent: boolean` + helper `sendUploadComplete()`.

### Fix #6 (MEDIO) — Delete dopo finalize
- **File**: `src/ui/app.ts` riga 1711
- **Problema**: `incomingUploads.delete` prima di `finalizeIncomingUpload` → entry orfana in caso di fallimento.
- **Fix**: Spostata la delete DOPO `await finalizeIncomingUpload(...)`.

## Fase 2: Fix #5 (ALTO) — Allineamento limite hardcoded

### Fix #5 (ALTO) — &turnMax= nel link
- **File**: `src-tauri/src/commands/p2p/generate_web_link.rs` righe 142-150
- **File**: `src-tauri/src/commands/p2p/create_inbox.rs` righe 84-92
- **File**: `src/ui/web-receiver.html` righe 521-533, 658, 1095
- **File**: `netlify-deploy/index.html` righe 521-533, 650, 1087
- **Problema**: Il browser usava il valore hardcoded `104857600` (100MB) per il controllo overlimit. Se il limite veniva cambiato via env (`TURN_MAX_FILE_SIZE`), il browser non lo saprebbe mai.
- **Fix**:
  1. Backend: aggiunto `&turnMax={turn_max}` ai link generati da `generate_web_link.rs` e `create_inbox.rs`
  2. Browser: letta la variabile `TURN_MAX_FILE_SIZE` dal URL con fallback a `104857600` se assente o non valido
  3. Sostituiti tutti gli usi hardcoded di `104857600` con `TURN_MAX_FILE_SIZE` in entrambi i file HTML

### Retrocompatibilità
- Link già generati senza `&turnMax=` funzionano grazie al fallback a `104857600`
- Nessuna migrazione necessaria
- Nessuna modifica alle capabilities o ai comandi Tauri

## Fase 3: Fix #11 (BASSO) — Cache di get_turn_limits

### Fix #11 (BASSO) — Cache IPC ridondanti
- **File**: `src/ui/app.ts` righe 1147-1154, 1538-1540
- **Problema**: `get_turn_limits` veniva chiamato 3 volte: all'avvio (corretto), in `streamFileToConnection` (ridondante), in `processIncomingMessage` (ridondante).
- **Fix**: Sostituite le due chiamate ridondanti con la cache `__turnMaxFileSize` già caricata all'avvio. Fallback a un'unica chiamata IPC solo se la cache non è ancora pronta.

## Stato dei File Modificati
- `src/ui/app.ts` — 7 blocchi modificati
- `src/ui/web-receiver.html` — 2 blocchi modificati (detectAndMarkPath + parametri URL)
- `netlify-deploy/index.html` — 3 blocchi modificati (parametri URL + detectAndMarkPath + getTurnSizeLimitMessage)
- `src-tauri/src/commands/p2p/generate_web_link.rs` — 1 blocco modificato
- `src-tauri/src/commands/p2p/create_inbox.rs` — 1 blocco modificato
- `CHECKPOINT.md` — questo file

## Note
- Nessun file Rust di infrastruttura modificato (solo 2 file di generazione link).
- `netlify-deploy/index.html` è stato sincronizzato con `src/ui/web-receiver.html`.
- I due file HTML sono ora identici per le parti modificate.
- Chiamate IPC ridondanti eliminate: `get_turn_limits` ora chiamata una sola volta all'avvio.
