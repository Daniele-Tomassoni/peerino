#!/usr/bin/env python3
"""Batch translate remaining Italian strings in Rust files."""
import os

replacements = {
    'src-tauri/src/commands/p2p/generate_web_link.rs': [
        ('//    Risolve automaticamente quale provider usare in base alle env, con',
         '//    Automatically resolves which provider to use based on env, with'),
        ('//    fetch asincrono per metered.ca. Restituisce un IceResolution che include',
         '//    async fetch for metered.ca. Returns an IceResolution that includes'),
        ('//    l\'array iceServers pronto per il browser (formato WebRTC standard).',
         '//    the iceServers array ready for the browser (standard WebRTC format).'),
        ('// Override manuale del signaling (parametro esplicito del comando Tauri).',
         '// Manual signaling override (explicit Tauri command parameter).'),
        ('// NOTA: STUN e TURN sono ora inclusi SOLO nel parametro `&ice=<base64>` sotto.',
         '// NOTE: STUN and TURN are now included ONLY in the `&ice=<base64>` parameter below.'),
        ('// Rimossi i parametri ridondanti `&stunUrls=` e `&turnUrls/turnUser/turnPass=`',
         '// Removed redundant `&stunUrls=` and `&turnUrls/turnUser/turnPass=` params'),
        ('// perché il parametro `&ice` (formato WebRTC standard) li contiene già tutti.',
         '// because the `&ice` param (standard WebRTC format) already contains them all.'),
        ('// Questo evita config duplicata/conflict nel browser.',
         '// This avoids duplicate/conflicting config in the browser.'),
        ('// TURN: credenziali statiche legacy (backward compat). Se il chiamante passa',
         '// TURN: legacy static credentials (backward compat). If the caller passes'),
        ('// esplicitamente turn_username + turn_password e NON c\'è un TURN provider',
         '// explicit turn_username + turn_password and there is NO TURN provider'),
        ('// configurato, li aggiungiamo come fallback legacy (parametri separati).',
         '// configured, we add them as a legacy fallback (separate params).'),
        ('// Provider TURN unificato: passa l\'intero array iceServers come parametro',
         '// Unified TURN provider: pass the entire iceServers array as a parameter'),
        ('// base64. Il browser lo deserializza e lo passa direttamente a RTCPeerConnection.',
         '// base64. The browser deserializes it and passes it directly to RTCPeerConnection.'),
        ('// Formato: &ice=<base64(JSON)>. Questo è il modo raccomandato per il browser',
         '// Format: &ice=<base64(JSON)>. This is the recommended approach for the browser'),
        ('// perché supporta QUALSIASI provider (metered, coturn self-hosted, misto).',
         '// because it supports ANY provider (metered, self-hosted coturn, mixed).'),
        ('// Diagnostica dettagliata: quanti STUN, quanti TURN, quale provider.',
         '// Detailed diagnostics: how many STUN, how many TURN, which provider.'),
        ('// Warning esplicito quando il link è solo-STUN: l\'utente (e lo sviluppatore',
         '// Explicit warning when the link is STUN-only: the user (and the developer'),
        ('// nei log) capisce subito perché la connessione potrebbe fallire su NAT',
         '// in the logs) immediately understands why the connection might fail on'),
        ('// simmetrico o CGNAT. Suggerisce anche la remediation concreta.',
         '// symmetric NAT or CGNAT. It also suggests the concrete remediation.'),
        ('"File non trovato nell\'indice"', '"File not found in index"'),
        ('"File non presente su disco: {}"', '"File not present on disk: {}"'),
        ('"PeerID non disponibile. Assicurati che PeerJS sia connesso."',
         '"PeerID unavailable. Ensure PeerJS is connected."'),
        ('// Usa P2P_WEB_URL (Netlify) come base URL: raggiungibile da internet.',
         '// Use P2P_WEB_URL (Netlify) as the base URL: reachable from the internet.'),
        ('// Il fallback LAN viene tentato tramite il parametro \'lan=\' aggiunto sotto.',
         '// The LAN fallback is attempted via the \'lan=\' parameter added below.'),
        ('"❌ Link SENZA TURN servers: solo STUN. Connessioni P2P su"',
         '"❌  Link WITHOUT TURN servers: STUN only. P2P connections on"'),
        ('"   NAT simmetrico o dietro CGNAT (es. Iliad/Ho.Mobile) falliranno."',
         '"   symmetric NAT or behind CGNAT (e.g. Iliad/Ho.Mobile) will fail."'),
        ('"   Remediation: upgrade metered a piano paid OPPURE configura"',
         '"   Remediation: upgrade metered to a paid plan OR configure"'),
        ('"   un VPS con coturn self-hosted (TURN_URLS + TURN_AUTH_SECRET)."',
         '"   a VPS with self-hosted coturn (TURN_URLS + TURN_AUTH_SECRET)."'),
    ],
}

for filepath, pairs in replacements.items():
    if not os.path.exists(filepath):
        print(f'SKIP (not found): {filepath}')
        continue
    with open(filepath, 'r') as f:
        content = f.read()
    modified = False
    for old, new in pairs:
        if old in content:
            content = content.replace(old, new)
            modified = True
        else:
            print(f'  NOT FOUND in {filepath}: {old[:80]}')
    if modified:
        with open(filepath, 'w') as f:
            f.write(content)
        print(f'UPDATED: {filepath}')
    else:
        print(f'NO CHANGES: {filepath}')

print('Done')