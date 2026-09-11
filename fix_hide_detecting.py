#!/usr/bin/env python3
"""Batch translate remaining Italian strings in peerino.md."""
import os

path = 'peerino.md'
with open(path, 'r', encoding='utf-8') as f:
    content = f.read()

replacements = [
    ('# Peerino — Documento di Sintesi del Progetto', '# Peerino — Project Summary Document'),
    ('> Documento di analisi tecnica del progetto **Peerino** (P2P File Sharing App), versione 3.2.0.',
     '> Technical analysis document of the **Peerino** project (P2P File Sharing App), version 3.2.0.'),
    ('> Tutte le affermazioni sono supportate da evidenze rinvenute nel codice sorgente; in caso di dubbio, l\'informazione è segnalata esplicitamente nella sezione *Limitazioni*.',
     '> All claims are supported by evidence found in the source code; in case of doubt, the information is explicitly flagged in the *Limitations* section.'),
    ('## 1. Panoramica del Progetto', '## 1. Project Overview'),
    ('### 1.1 Scopo e Dominio', '### 1.1 Purpose and Domain'),
    ('**Peerino** è un\'applicazione desktop cross-platform per la **condivisione di file peer-to-peer** (P2P) costruita con **Tauri 2.0** (Rust backend + WebView frontend). L\'obiettivo dichiarato nel file',
     '**Peerino** is a cross-platform desktop application for **peer-to-peer file sharing** (P2P) built with **Tauri 2.0** (Rust backend + WebView frontend). The goal stated in file'),
    ('è realizzare un sistema di condivisione file *decentralizzato*, *senza server centrali* e con un\'architettura incrementale articolata in sei fasi di sviluppo.',
     'is to build a *decentralized* file sharing system, *without central servers*, with an incremental architecture organized into six development phases.'),
    ('Il dominio applicativo spazia dalla condivisione file in rete locale (LAN/WiFi) alla distribuzione di file su Internet via WebRTC, con supporto per il download diretto da browser (P2P-to-Web) e per la ricezione file da browser (Scatola di Consegna Inversa). È prevista l\'introduzione di un sistema di crediti interni (Fase 5) e di un\'app mobile (Fase 6), entrambe non ancora implementate ma documentate come roadmap.',
     'The application domain spans from file sharing on local networks (LAN/WiFi) to file distribution over the Internet via WebRTC, with support for direct browser downloads (P2P-to-Web) and file reception from browsers (Reverse Delivery Box). The introduction of an internal credit system (Phase 5) and a mobile app (Phase 6) is planned, both not yet implemented but documented in the roadmap.'),
    ('### 1.2 Pubblico Target', '### 1.2 Target Audience'),
    ('- **Utenti finali desktop** che necessitano di condividere file in rete locale o via Internet senza infrastrutture server dedicate.',
     '- **Desktop end users** who need to share files on local networks or over the Internet without dedicated server infrastructure.'),
    ('- **Destinatari remoti** (browser o altre istanze Peerino) che ricevono link pubblici auto-generati.',
     '- **Remote recipients** (browser or other Peerino instances) who receive auto-generated public links.'),
    ('- **Sviluppatori** che intendono estendere l\'app secondo la roadmap documentata in',
     '- **Developers** who intend to extend the app according to the roadmap documented in'),
    ('### 1.3 Stato del Progetto', '### 1.3 Project Status'),
    ('- **Versione corrente**: `3.2.0` (vedi',
     '- **Current version**: `3.2.0` (see'),
    ('- **Fase completata**: Fase 4 (P2P su Internet) con funzionalità avanzate. Il codice contiene già il sistema di cancellazione robusto (P0–P3) documentato in',
     '- **Completed phase**: Phase 4 (P2P over Internet) with advanced features. The code already contains the robust cancellation system (P0–P3) documented in'),
    ('- **Fase pianificata**: Fase 5 (P2P locale con crediti) e Fase 6 (mobile).',
     '- **Planned phase**: Phase 5 (local P2P with credits) and Phase 6 (mobile).'),
    ('- **Modalità di sviluppo**: il P2P diretto app↔app via PeerJS è implementato ma **nascosto nell\'UI** (la UI di connessione manuale è stata rimossa; il motore PeerJS gira *headless* per generare il PeerID necessario ai link web). Vedi',
     '- **Development mode**: direct app↔app P2P via PeerJS is implemented but **hidden in the UI** (the manual connection UI was removed; the PeerJS engine runs *headless* to generate the PeerID needed for web links). See'),
    ('- **Tipo di repository**: non è un repository Git (vedi',
     '- **Repository type**: not a Git repository (see'),
    ('### 1.4 Caratteristiche Distintive', '### 1.4 Distinctive Features'),
    ('- **Streaming obbligatorio**: nessun `Vec<u8>` viene passato per i file (regola architetturale esplicita, vedi',
     '- **Mandatory streaming**: no `Vec<u8>` is passed for files (explicit architectural rule, see'),
    ('I file sono gestiti con `tokio::fs` e buffer da 64KB.',
     'Files are handled with `tokio::fs` and 64KB buffers.'),
    ('- **Tauri Channels** per lo streaming binario ad alta frequenza (`tauri::ipc::Channel<Vec<u8>>`).',
     '- **Tauri Channels** for high-frequency binary streaming (`tauri::ipc::Channel<Vec<u8>>`).'),
    ('- **Capabilities Tauri 2.0** dichiarate in file dedicati',
     '- **Tauri 2.0 capabilities** declared in dedicated files'),
    ('mai in `tauri.conf.json`.',
     'never in `tauri.conf.json`.'),
    ('- **Database SQLite con WAL mode** e tutte le operazioni incapsulate in `tokio::task::spawn_blocking`.',
     '- **SQLite database with WAL mode** and all operations encapsulated in `tokio::task::spawn_blocking`.'),
    ('- **TURN credentials effimere** calcolate localmente via HMAC-SHA1 (schema coturn REST) per i link P2P-to-Web.',
     '- **Ephemeral TURN credentials** computed locally via HMAC-SHA1 (coturn REST schema) for P2P-to-Web links.'),
    ('- **Sistema di cancellazione robusta** per download e upload, con flag atomici (`AtomicBool`) e telemetria.',
     '- **Robust cancellation system** for downloads and uploads, with atomic flags (`AtomicBool`) and telemetry.'),
]

for old, new in replacements:
    if old in content:
        content = content.replace(old, new)
    else:
        print(f'NOT FOUND: {old[:80]}')

with open(path, 'w', encoding='utf-8') as f:
    f.write(content)
print('Done phase 1')