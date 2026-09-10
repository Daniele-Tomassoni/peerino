/**
 * Test Vitest per il pulsante "Annulla download" / "Annulla upload".
 *
 * Verifica:
 * - Click su cancel-btn con data-download-key chiama SOLO cancel_download
 *   (Fix #4: niente più doppia chiamata che causava race condition)
 * - Click su cancel-btn con data-upload-key chiama cancel_upload
 * - Click su cancel-btn senza chiave è no-op
 * - Stato cancelled viene impostato dopo l'invoke
 * - Animazione cancelling viene applicata prima della rimozione
 *
 * Esegui: `npm test -- cancel-button`
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

// Mock Tauri API
vi.mock('@tauri-apps/api/core', () => ({
    invoke: vi.fn().mockResolvedValue(null),
}));

vi.mock('@tauri-apps/plugin-dialog', () => ({
    open: vi.fn(),
}));

vi.mock('@tauri-apps/plugin-clipboard-manager', () => ({
    writeText: vi.fn(),
}));

vi.mock('@tauri-apps/plugin-notification', () => ({
    sendNotification: vi.fn(),
}));

vi.mock('@tauri-apps/api/event', () => ({
    listen: vi.fn().mockResolvedValue(() => {}),
}));

vi.mock('peerjs', () => ({
    Peer: vi.fn().mockImplementation(() => ({
        on: vi.fn(),
        destroy: vi.fn(),
        open: true,
        id: 'mock-peer-id',
    })),
}));

describe('Cancel button - download', () => {
    let invoke: ReturnType<typeof vi.fn>;

    beforeEach(async () => {
        // Reset DOM
        document.body.innerHTML = `
            <div id="download-progress-list"></div>
            <p id="footer-status"></p>
            <div id="download-progress-container" class="hidden"></div>
            <button id="upload-btn"></button>
            <button id="refresh-btn"></button>
            <button id="open-folder-btn"></button>
            <select id="column-select"></select>
            <select id="sort-select"></select>
            <div id="drop-zone"></div>
            <div id="drop-overlay"></div>
            <p id="upload-status"></p>
            <p id="server-status-text"></p>
            <span id="server-led"></span>
            <span id="server-led-small"></span>
            <p id="server-status-small"></p>
            <span id="local-ip"></span>
            <span id="my-peer-id"></span>
            <button id="copy-peer-id-btn"></button>
            <span id="relay-status"></span>
            <input id="remote-peer-id" />
            <button id="connect-peer-btn"></button>
            <button id="disconnect-peer-btn"></button>
            <div id="connection-status"></div>
            <span id="connection-status-text"></span>
            <ul id="peers-list"></ul>
            <button id="generate-web-link-btn"></button>
            <div id="web-link-container"></div>
            <span id="web-link-display"></span>
            <button id="copy-web-link-btn"></button>
            <input type="checkbox" id="streaming-toggle" />
            <button id="create-inbox-local-btn"></button>
            <div id="inbox-local-link-container"></div>
            <span id="inbox-local-link"></span>
            <button id="copy-inbox-local-link-btn"></button>
            <button id="create-inbox-internet-btn"></button>
            <div id="inbox-internet-link-container"></div>
            <span id="inbox-internet-link"></span>
            <button id="copy-inbox-internet-link-btn"></button>
            <button id="generate-local-link-btn"></button>
            <div id="local-link-container"></div>
            <span id="local-link-display"></span>
            <button id="copy-local-link-btn"></button>
            <div id="upload-progress-container"></div>
            <div id="upload-progress-list"></div>
            <div id="conn-tooltip"></div>
            <div id="files-list"></div>
        `;
        // Mock Tauri global
        (window as any).__TAURI__ = { invoke: vi.fn().mockResolvedValue(null) };
        const tauri = await import('@tauri-apps/api/core');
        invoke = vi.mocked(tauri.invoke);
        invoke.mockClear();
    });

    afterEach(() => {
        vi.clearAllTimers();
    });

    it('click su cancel-btn download deve chiamare SOLO cancel_download', async () => {
        // Setup: simula una riga di download attiva
        const row = document.createElement('div');
        row.className = 'download-item';
        row.setAttribute('data-download-id', 'test-hash-123');
        row.innerHTML = `
            <button class="cancel-btn" data-download-key="test-hash-123">✕</button>
        `;
        const list = document.getElementById('download-progress-list')!;
        list.appendChild(row);

        // Simula click
        const btn = row.querySelector('.cancel-btn') as HTMLButtonElement;
        btn.click();

        // Aspetta che il setTimeout dell'animazione parta
        await new Promise(r => setTimeout(r, 50));

        // Fix #4: SOLO cancel_download, NON cancel_upload
        expect(invoke).toHaveBeenCalledWith('cancel_download', { hash: 'test-hash-123' });
        expect(invoke).not.toHaveBeenCalledWith('cancel_upload', expect.anything());
    });

    it('click su cancel-btn upload deve chiamare cancel_upload', async () => {
        const row = document.createElement('div');
        row.className = 'upload-item';
        row.setAttribute('data-upload-id', 'peer1-hash456');
        row.innerHTML = `
            <button class="cancel-btn" data-upload-key="peer1-hash456">✕</button>
        `;
        const list = document.getElementById('upload-progress-list')!;
        list.appendChild(row);

        const btn = row.querySelector('.cancel-btn') as HTMLButtonElement;
        btn.click();

        await new Promise(r => setTimeout(r, 50));

        expect(invoke).toHaveBeenCalledWith('cancel_upload', { hash: 'hash456' });
        // Non deve chiamare cancel_download per un upload
        expect(invoke).not.toHaveBeenCalledWith('cancel_download', expect.anything());
    });

    it('click su cancel-btn senza chiave non chiama nulla', async () => {
        const btn = document.createElement('button');
        btn.className = 'cancel-btn';
        // Nessun data-download-key, nessun data-upload-key
        document.body.appendChild(btn);

        btn.click();

        await new Promise(r => setTimeout(r, 10));

        expect(invoke).not.toHaveBeenCalled();
    });

    it('click disabilita immediatamente il pulsante (prevenzione doppio click)', async () => {
        const row = document.createElement('div');
        row.className = 'download-item';
        row.setAttribute('data-download-id', 'test-hash');
        row.innerHTML = `
            <button class="cancel-btn" data-download-key="test-hash">✕</button>
        `;
        document.getElementById('download-progress-list')!.appendChild(row);

        const btn = row.querySelector('.cancel-btn') as HTMLButtonElement;
        btn.click();

        // Sincrono: il pulsante deve essere disabilitato subito
        // (la disabilitazione avviene prima del setTimeout di 250ms)
        expect(btn.disabled).toBe(true);
    });

    it('aggiunge classe cancelling per animazione di feedback', async () => {
        const row = document.createElement('div');
        row.className = 'download-item';
        row.setAttribute('data-download-id', 'test-hash');
        row.innerHTML = `
            <button class="cancel-btn" data-download-key="test-hash">✕</button>
        `;
        document.getElementById('download-progress-list')!.appendChild(row);

        const btn = row.querySelector('.cancel-btn') as HTMLButtonElement;
        btn.click();

        // La classe cancelling deve essere applicata immediatamente
        expect(row.classList.contains('cancelling')).toBe(true);
    });

    it('footer status mostra feedback post-cancellazione', async () => {
        const row = document.createElement('div');
        row.className = 'download-item';
        row.setAttribute('data-download-id', 'test-hash');
        row.innerHTML = `
            <button class="cancel-btn" data-download-key="test-hash">✕</button>
        `;
        document.getElementById('download-progress-list')!.appendChild(row);

        const btn = row.querySelector('.cancel-btn') as HTMLButtonElement;
        btn.click();

        await new Promise(r => setTimeout(r, 10));

        const footer = document.getElementById('footer-status');
        expect(footer?.textContent).toContain('annullato');
    });
});
