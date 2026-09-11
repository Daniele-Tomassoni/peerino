/**
 * Vitest test for the "Cancel download" / "Cancel upload" button.
 *
 * Verifies:
 * - Click on cancel-btn with data-download-key calls ONLY cancel_download
 *   (Fix #4: no more double call that caused a race condition)
 * - Click on cancel-btn with data-upload-key calls cancel_upload
 * - Click on cancel-btn without a key is a no-op
 * - Cancelled state is set after the invoke
 * - Cancelling animation is applied before removal
 *
 * Setup: mocks window.__TAURI__ with only the invoke function
 * (no core/invoke needed for this test).
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';

// Mock Tauri invoke
const mockInvoke = vi.fn();
(window as any).__TAURI__ = {
    invoke: mockInvoke,
};

// Import the app module (loads the file with mocked Tauri globals)
// We need to mock the Tauri APIs used at module level
vi.mock('@tauri-apps/api/core', () => ({
    invoke: mockInvoke,
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
    listen: vi.fn(),
}));

// Mock PeerJS
vi.mock('peerjs', () => ({
    Peer: vi.fn().mockImplementation(() => ({
        on: vi.fn(),
        connect: vi.fn(),
        destroy: vi.fn(),
        id: null,
        open: false,
        connections: new Map(),
    })),
    DataConnection: vi.fn(),
}));

describe('Cancel button - download', () => {
    let invoke: ReturnType<typeof vi.fn>;

    beforeEach(() => {
        document.body.innerHTML = `
            <div id="download-progress-list"></div>
            <p id="footer-status"></p>
            <div id="download-progress-container" class="hidden"></div>
            <button id="upload-btn"></button>
            <button id="refresh-btn"></button>
            <button id="open-folder-btn"></button>
            <button id="upload-btn"></button>
            <div id="drop-zone"></div>
            <div id="drop-overlay"></div>
            <p id="upload-status"></p>
            <p id="server-status-text"></p>
            <span id="server-led"></span>
            <div id="download-progress-container"></div>
            <div id="files-list"></div>
            <button id="copy-local-link-btn"></button>
            <div id="upload-progress-container"></div>
            <div id="upload-progress-list"></div>
            <div id="conn-tooltip"></div>
        `;
        invoke = mockInvoke;
        invoke.mockClear();
    });

    it('click on cancel-btn download should call ONLY cancel_download', async () => {
        // Setup: simulate an active download row
        const row = document.createElement('div');
        row.className = 'download-item';
        row.setAttribute('data-download-id', 'test-hash-123');
        row.innerHTML = `
            <button class="cancel-btn" data-download-key="test-hash-123">✕</button>
        `;
        const list = document.getElementById('download-progress-list')!;
        list.appendChild(row);

        const btn = row.querySelector('.cancel-btn')!;
        btn.click();

        // Wait for the animation setTimeout to start
        await new Promise(r => setTimeout(r, 50));

        expect(invoke).toHaveBeenCalledWith('cancel_download', { hash: 'test-hash-123' });
        // Must NOT call cancel_upload for a download
        expect(invoke).not.toHaveBeenCalledWith('cancel_upload', expect.anything());
    });

    it('click on cancel-btn upload should call cancel_upload', async () => {
        const row = document.createElement('div');
        row.className = 'upload-item';
        row.setAttribute('data-upload-id', 'peer1-hash456');
        row.innerHTML = `
            <button class="cancel-btn" data-upload-key="peer1-hash456">✕</button>
        `;
        const list = document.getElementById('upload-progress-list')!;
        list.appendChild(row);

        const btn = row.querySelector('.cancel-btn')!;
        btn.click();

        await new Promise(r => setTimeout(r, 50));

        expect(invoke).toHaveBeenCalledWith('cancel_upload', { hash: 'hash456' });
        // Must NOT call cancel_download for an upload
        expect(invoke).not.toHaveBeenCalledWith('cancel_download', expect.anything());
    });

    it('click on cancel-btn without key calls nothing', async () => {
        const btn = document.createElement('button');
        btn.className = 'cancel-btn';
        // No data-download-key, no data-upload-key
        document.body.appendChild(btn);

        btn.click();
        await new Promise(r => setTimeout(r, 50));

        expect(invoke).not.toHaveBeenCalled();
    });

    it('click immediately disables the button (double-click prevention)', async () => {
        const row = document.createElement('div');
        row.className = 'download-item';
        row.setAttribute('data-download-id', 'test-hash');
        row.innerHTML = `
            <button class="cancel-btn" data-download-key="test-hash">✕</button>
        `;
        document.getElementById('download-progress-list')!.appendChild(row);

        const btn = row.querySelector('.cancel-btn')!;
        btn.click();

        // Synchronous: the button must be disabled immediately
        // (disabling happens before the 250ms setTimeout)
        expect(btn.disabled).toBe(true);
    });

    it('adds cancelling class for feedback animation', async () => {
        const row = document.createElement('div');
        row.className = 'download-item';
        row.setAttribute('data-download-id', 'test-hash');
        row.innerHTML = `
            <button class="cancel-btn" data-download-key="test-hash">✕</button>
        `;
        document.getElementById('download-progress-list')!.appendChild(row);

        const btn = row.querySelector('.cancel-btn')!;
        btn.click();

        await new Promise(r => setTimeout(r, 50));

        expect(row.classList.contains('cancelling')).toBe(true);
    });

    it('shows cancelled message in footer', async () => {
        const row = document.createElement('div');
        row.className = 'download-item';
        row.setAttribute('data-download-id', 'test-hash');
        row.innerHTML = `
            <button class="cancel-btn" data-download-key="test-hash">✕</button>
        `;
        document.getElementById('download-progress-list')!.appendChild(row);

        const btn = row.querySelector('.cancel-btn')!;
        btn.click();

        await new Promise(r => setTimeout(r, 50));

        const footer = document.getElementById('footer-status');
        expect(footer?.textContent).toContain('cancelled');
    });
});
