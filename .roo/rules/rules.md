# Istruzioni di Comportamento dell'Agente (P2P Share)

- Stai sviluppando un'applicazione P2P File Sharing usando Tauri 2.0 (Rust backend) e TypeScript Vanilla (Frontend).
- Prima di eseguire qualsiasi operazione, analizzare la struttura del progetto e leggere attentamente il file `project-context.md` per comprendere i vincoli architetturali generali.

## Regole Architetturali Tassative (Tauri 2.0 & Rust)
1. Le configurazioni di sicurezza e i permessi DEVONO essere definiti in `src-tauri/capabilities/default.json` (mai in `tauri.conf.json`).
2. È severamente VIETATO l'uso di `Vec<u8>` per la gestione e il passaggio dei file. Usa esclusivamente lo streaming asincrono con `tokio::fs` e buffer da 64KB.
3. Per le interazioni IPC ad alta frequenza o lo streaming di dati binari verso la UI, utilizza i **Tauri Channels** (`tauri::ipc::Channel<Vec<u8>>`).
4. Qualsiasi chiamata al database SQLite tramite `rusqlite` deve essere racchiusa in `tokio::task::spawn_blocking` (o `tauri::async_runtime::spawn_blocking`).
5. Il database deve essere inizializzato con la modalità WAL attiva (`conn.pragma_update(None, "journal_mode", &"WAL")?;`).
6. Gestisci la tokenomics dei crediti usando solo numeri interi (1 credito = 100 punti) per prevenire bug di precisione float.
7. Per la selezione file, usa `@tauri-apps/plugin-dialog` (NON `@tauri-apps/api/dialog`).
8. Per la codifica base64 delle chiavi AES (Fase 5), usa `base64::{engine::general_purpose::STANDARD, Engine}` (NON `base64::encode`).
9. Per lo streaming di file grandi usa buffer limitati (es. `tokio::sync::mpsc::channel(32)`) per prevenire OOM.

## Linee Guida per lo Sviluppo Incrementale
- Rispetta rigorosamente la roadmap suddivisa in 6 Fasi delineata nel contesto.
- Non implementare o anticipare feature di fasi successive a meno che non sia richiesto esplicitamente dal programmatore.
- Prima di considerare conclusa una fase, verifica mentalmente il rispetto dei relativi "Test di Accettazione" presenti nel documento di contesto.

## Test e Mock
- Per i test Vitest, crea un file `tests/setup.ts` con il mock di `window.__TAURI__` e dei plugin per evitare fallimenti fuori contesto Tauri.