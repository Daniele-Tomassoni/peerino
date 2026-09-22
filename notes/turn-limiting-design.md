# TURN Rate Limiting Design — Peerino

Stato: PROGETTAZIONE (non implementato)
Ultimo aggiornamento: 22 settembre 2026

## 1. Sommario esecutivo

Questo documento definisce l'architettura di rate limiting per il servizio
Cloudflare TURN usato da Peerino. L'obiettivo è proteggere il budget
Cloudflare (1 TB/mese egress, $0.05/GB oltre) senza degradare
l'esperienza utente per i trasferimenti legit.

Abbiamo verificato: la documentazione Cloudflare supporta `customIdentifier`
nella generazione credenziali, la GraphQL Analytics API fornisce metriche di
egress per `customIdentifier`, e Turnstile offre autenticazione gratis fino a
1M challenge/mese.

La prossima azione quando servirà: misurare il consumo attuale (sezione 9),
decidere il TTL (sezione 7.1), poi implementare i layer in ordine.

## 2. Requisiti

| Requisito | Valore | Fonte |
|-----------|--------|-------|
| Limite per utente (peer ID) | 1 GB/mese di traffico TURN | App |
| Limite globale | 1 TB/mese egress totale | Cloudflare (free tier) |
| Prezzo oltre free tier | $0.05/GB | Cloudflare |
| Turn max file size | 100 MB | App (non Cloudflare) |
| TTL credenziali | 24h (attuale), 5-15 min (raccomandato) | wrangler.toml |
| Permesso token API | "Account Analytics: Read" | Cloudflare |

## 3. Stato attuale (settembre 2026)

- Worker `peerino-turn-proxy`: stateless, no auth, no rate limit.
- TTL default: 24h (`TURN_CRED_TTL = 86400` in `wrangler.toml`).
- Client Rust (`ice_provider.rs`): passa solo `Accept: application/json`,
  nessun identificatore utente.
- Nessun KV / DO / D1 configurato nel Worker.
- Consumo attuale: **DA MISURARE** (vedi sezione 9).

## 4. Cosa abbiamo verificato sulla documentazione Cloudflare

Citazioni verificabili:

- `customIdentifier` supportato alla generazione credenziali
  (fonte: Cloudflare TURN generate-credentials).
- GraphQL Analytics API: `https://api.cloudflare.com/client/v4/graphql`.
- Dataset: `callsTurnUsageAdaptiveGroups`.
- Metriche: `egressBytes`, `ingressBytes`, `concurrentConnections`.
- Dimensioni: `keyId`, `customIdentifier`, `username`.
- Filtri: `customIdentifier`, `keyId`, `date_geq`, `date_leq`.
- Permesso token API: "Account Analytics: Read".
- Esempio query "Usage for a specific custom identifier" nella doc.

Esempio di query GraphQL per `customIdentifier`:

```graphql
{
  viewer {
    accounts(filter: { accountTag: "<CF_ACCOUNT_ID>" }) {
      callsTurnUsageAdaptiveGroups(
        limit: 10000
        filter: {
          date_geq: "<inizio-mese>"
          date_leq: "<fine-mese>"
          customIdentifier: "<peerId>"
        }
      ) {
        sum {
          egressBytes
        }
      }
    }
  }
}
```

## 5. Errori e approcci scartati

Questa sezione serve a non ripetere gli stessi errori.

### 5.1 TTL non limita i byte totali — ma limita il BURST

Correzione importante: il TTL non riduce il traffico complessivo
(una credenziale può relayare byte illimitati per tutto il TTL).
MA il TTL è **l'unica variabile che limita il danno di un burst**.

Tabella (danno max per burst, assumendo 1 Gbps sostenuto):

| TTL    | Danno max per burst |
|--------|---------------------|
| 24h    | ~10 TB              |
| 1h     | ~450 GB             |
| 15 min | ~112 GB             |
| 5 min  | ~37 GB              |

Conclusione: TTL deve scendere a 5-15 min.

### 5.2 Autenticazione client→Worker è obbligatoria

L'endpoint `/api/turn-credentials` è pubblico. Chiunque con `curl` può
chiedere credenziali. Senza auth:

- Il rate limit per peerId è aggirabile (attaccante genera 100 peer ID).
- Il check GraphQL è aggirabile (attaccante non è utente legittimo).
- Qualsiasi threshold è aggirabile.

Opzioni auth:

- **Cloudflare Turnstile** (consigliato): challenge + token monouso,
  gratis fino a 1M challenge/mese.
- **Shared secret embedded nel binario**: semplice ma estraibile con
  `strings`, richiede rotazione, sconsigliato.
- **Rate limit per IP**: utile solo come difesa secondaria (NAT condiviso).

### 5.3 KV non è atomico — non usarlo per contatori

Cloudflare KV non ha increment atomico. Due richieste concorrenti con
`read → +1 → write` possono perdere un incremento. Per contatori
affidabili serve **Durable Object** o **D1 con transazione**.

### 5.4 Strada "credenziali emesse × 100 MB" è sbagliata

Assumere "1 credenziale = 1 file = 100 MB" è falso di ~1000×.
Vedi 5.1. Non implementare questa strada.

### 5.5 Strada "rate limit per IP" è insufficiente da sola

NAT condiviso → falsi positivi. Non contabilizza byte. Va bene solo come
difesa secondaria.

## 6. Design raccomandato

Architettura a strati:

**Layer 1 — TTL breve (5-15 min)**
- Riduce il danno massimo di burst.
- Richiede **credential refresh lato client** a metà trasferimento
  (feature nuova, non implementata oggi).

**Layer 2 — Autenticazione client→Worker**
- Turnstile (consigliato) o shared secret.
- Obbligatoria prima di qualsiasi altro layer.

**Layer 3 — Rate limit credenziali per peerId**
- Formula: N_creds/h ≈ (60 / TTL_min) + margine.
  - TTL 5 min  → ~12 creds/h + margine → limite ~30/h
  - TTL 15 min → ~4 creds/h + margine → limite ~15/h
  - TTL 1h     → ~1 cred/h + margine → limite ~5/h
- Il numero preciso si fissa dopo aver deciso il TTL (sezione 7.1).
- Solo dopo Layer 2 (altrimenti è teatro).
- Implementare con **Durable Object**, non KV.

**Layer 4 — Check GraphQL usage per customIdentifier**
- Worker riceve `peerId` dal client.
- Lo passa come `customIdentifier` a Cloudflare.
- Prima di emettere: query GraphQL per usage del peerId nel mese.
- Soglia consigliata iniziale: **500 MB** (margine per latenza analytics).
- Se > soglia: 429.

**Layer 5 — Check GraphQL usage globale**
- Stessa query senza filtro `customIdentifier`.
- Cache KV 15 min (query costosa).
- Se > 900 GB: 429 globale.

**Layer 6 — Monitoring + alert**
- Worker cron separato (`turn-usage-monitor`).
- Ogni 6 ore legge l'egress totale.
- Se > 800 GB: notifica Telegram.
- Non tocca l'app né il Worker TURN.

## 7. Ordine di decisione (prima di implementare)

1. **TTL**: 5-15 min con refresh client? o 1h accettando burst ~450 GB?
2. **Auth**: Turnstile / shared secret / nessuna (con conseguenze)?
3. **Rate limit credenziali**: sì/no (dipende da 2).
4. **Threshold byte**: 500 MB iniziale, poi calibra con dato latenza.
5. **Client-side refresh**: da progettare se TTL < durata trasferimento.
6. **Latenza analytics**: da misurare (vedi sezione 9).

## 8. Domande aperte (da risolvere prima di implementare)

- [ ] **Latenza GraphQL analytics**: quanto tempo passa tra consumo reale
      e visibilità? Da 5 min a 30 min? Determina il margine di sicurezza.
- [ ] **Rate limit query GraphQL**: numero riportato "300 query/5min" in
      analisi precedente ma FONTE NON CONFERMATA. Cercare nella doc
      https://developers.cloudflare.com/analytics/graphql-api/limits/ e
      linkare qui la fonte esatta, oppure rimuovere il numero.
- [ ] **Consumo attuale TURN**: da misurare (vedi 9).
- [ ] **Credential refresh in Peerino**: come implementarlo senza rompere
      trasferimenti in corso?

## 9. Cosa misurare e come

**TASK A — Latenza analytics**
Script/istruzioni in `peerino-turn-proxy/test/latency-test.md`
(da creare quando si affronta il task).

**TASK C — Consumo attuale (5 min, curl)**

```bash
curl https://api.cloudflare.com/client/v4/graphql \
  -H "Authorization: Bearer $CF_ANALYTICS_TOKEN" \
  -H "Content-Type: application/json" \
  --data-raw @- <<EOF
{"query":"{ viewer { accounts(filter: {accountTag: \"$CF_ACCOUNT_ID\"}) { callsTurnUsageAdaptiveGroups(limit: 10000, filter: {date_geq: \"<inizio-mese>\", date_leq: \"<fine-mese>\"}) { sum { egressBytes } } } } }"}
EOF
```

Sostituisci <inizio-mese> e <fine-mese> con date ISO 8601 in UTC,
es. 2026-09-01T00:00:00Z e 2026-09-30T23:59:59Z.

Interpretazione:

- < 100 GB/mese → archiviare questo design, ripetere check tra 1-2 mesi.
- 100-500 GB → implementare monitoring (Layer 6) con calma.
- 500-800 GB → implementare Layer 1+2+6 subito.
- > 800 GB → implementare tutto.

## 10. Stato della decisione (aggiornare man mano)

- [ ] Consumo attuale misurato
- [ ] TTL deciso
- [ ] Auth deciso
- [ ] Rate limit deciso
- [ ] Implementazione iniziata
- [ ] Test end-to-end
- [ ] Deploy in produzione

## 11. Riferimenti

### Documentazione Cloudflare
- TURN generate-credentials: https://developers.cloudflare.com/realtime/turn/generate-credentials/
- TURN analytics (GraphQL): https://developers.cloudflare.com/realtime/turn/analytics/
- Turnstile: https://developers.cloudflare.com/turnstile/
- GraphQL API endpoint: https://api.cloudflare.com/client/v4/graphql
- GraphQL Analytics API (generale): https://developers.cloudflare.com/analytics/graphql-api/

### File del repo
- `peerino-turn-proxy/src/index.ts` — Worker TURN proxy
- `peerino-turn-proxy/wrangler.toml` — config Worker
- `src-tauri/src/utils/ice_provider.rs` — client Rust che chiama il Worker