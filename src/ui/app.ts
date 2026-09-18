// Peerino - P2P file sharing without cloud, without accounts, without intermediaries.
// Copyright (C) 2026 Daniele Tomassoni
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { writeText } from '@tauri-apps/plugin-clipboard-manager';
import { sendNotification } from '@tauri-apps/plugin-notification';
import { listen } from '@tauri-apps/api/event';
import { Peer, DataConnection } from 'peerjs';

// Global error handler
window.addEventListener('error', (event) => {
    console.error('🔥 GLOBAL ERROR:', event.error);
});

window.addEventListener('unhandledrejection', (event) => {
    console.error('🔥 PROMISE REJECTED:', event.reason);
});

// Interfaces
interface FileInfo {
    filename: string;
    size: number;
    hash: string;
    uploaded_at: string;
}

interface NetworkInfo {
    ip: string;
    port: number;
}

interface DownloadProgress {
    hash: string;
    filename: string;
    total_bytes: number;
    downloaded_bytes: number;
    speed_mbps: number;
    peer_ip: string;
    progress: number;
    cancelled: boolean;
}

interface UploadProgress {
    hash?: string;
    filename: string;
    bytes_processed: number;
    total_bytes: number;
    progress: number;
    speed_mbps?: number;
    peer_id?: string;
    cancelled: boolean;
}

interface PeerInfo {
    peer_id: string;
}

interface P2pConfig {
    signalingUrl?: string;
    turnUsername?: string;
    turnPassword?: string;
}

declare global {
    interface Window {
        fileMap: Map<string, FileInfo>;
    }
}
window.fileMap = new Map<string, FileInfo>();

// DOM Elements - Header
const serverLed = document.getElementById('server-led') as HTMLSpanElement;
const serverStatusText = document.getElementById('server-status-text') as HTMLParagraphElement;

// DOM Elements - Upload (left column)
const uploadBtn = document.getElementById('upload-btn') as HTMLButtonElement;
const uploadStatus = document.getElementById('upload-status') as HTMLParagraphElement;
const dropZone = document.getElementById('drop-zone') as HTMLDivElement;
const dropOverlay = document.getElementById('drop-overlay') as HTMLDivElement;

// Progress bar download (left column - shows inbox downloads)
const downloadProgressContainerLeft = document.getElementById('download-progress-container') as HTMLDivElement;

// DOM Elements - File (center column)
const filesList = document.getElementById('files-list') as HTMLDivElement;
const refreshBtn = document.getElementById('refresh-btn') as HTMLButtonElement;
const openFolderBtn = document.getElementById('open-folder-btn') as HTMLButtonElement;
const columnSelect = document.getElementById('column-select') as HTMLSelectElement;
const sortSelect = document.getElementById('sort-select') as HTMLSelectElement;

// DOM Elements - Sharing (right column)
const localIpEl = document.getElementById('local-ip') as HTMLSpanElement;
const serverLedSmall = document.getElementById('server-led-small') as HTMLSpanElement;
const serverStatusSmall = document.getElementById('server-status-small') as HTMLParagraphElement;
const startServerBtn = document.getElementById('start-server-btn') as HTMLButtonElement;
const myPeerIdEl = document.getElementById('my-peer-id') as HTMLSpanElement;
const copyPeerIdBtn = document.getElementById('copy-peer-id-btn') as HTMLButtonElement;
const relayStatusEl = document.getElementById('relay-status') as HTMLSpanElement;
const remotePeerIdInput = document.getElementById('remote-peer-id') as HTMLInputElement;
const connectPeerBtn = document.getElementById('connect-peer-btn') as HTMLButtonElement;
const disconnectPeerBtn = document.getElementById('disconnect-peer-btn') as HTMLButtonElement;
const connectionStatusEl = document.getElementById('connection-status') as HTMLDivElement;
const connectionStatusText = document.getElementById('connection-status-text') as HTMLSpanElement;
const peersListEl = document.getElementById('peers-list') as HTMLUListElement;

// P2P-to-Web Link elements
const generateWebLinkBtn = document.getElementById('generate-web-link-btn') as HTMLButtonElement;
const webLinkContainer = document.getElementById('web-link-container') as HTMLDivElement;
const webLinkDisplay = document.getElementById('web-link-display') as HTMLSpanElement;
const copyWebLinkBtn = document.getElementById('copy-web-link-btn') as HTMLButtonElement;

const streamingToggle = document.getElementById('streaming-toggle') as HTMLInputElement;

// Local Inbox
const createInboxLocalBtn = document.getElementById('create-inbox-local-btn') as HTMLButtonElement;
const inboxLocalLinkContainer = document.getElementById('inbox-local-link-container') as HTMLDivElement;
const inboxLocalLinkEl = document.getElementById('inbox-local-link') as HTMLSpanElement;
const copyInboxLocalLinkBtn = document.getElementById('copy-inbox-local-link-btn') as HTMLButtonElement;

// Internet Inbox
const createInboxInternetBtn = document.getElementById('create-inbox-internet-btn') as HTMLButtonElement;
const inboxInternetLinkContainer = document.getElementById('inbox-internet-link-container') as HTMLDivElement;
const inboxInternetLinkEl = document.getElementById('inbox-internet-link') as HTMLSpanElement;
const copyInboxInternetLinkBtn = document.getElementById('copy-inbox-internet-link-btn') as HTMLButtonElement;

// Generate Local Link
const generateLocalLinkBtn = document.getElementById('generate-local-link-btn') as HTMLButtonElement;
const localLinkContainer = document.getElementById('local-link-container') as HTMLDivElement;
const localLinkDisplay = document.getElementById('local-link-display') as HTMLSpanElement;
const copyLocalLinkBtn = document.getElementById('copy-local-link-btn') as HTMLButtonElement;

// Download progress list (ALWAYS VISIBLE) - left column (inbox downloads)
const downloadProgressListLeft = document.getElementById('download-progress-list') as HTMLDivElement;

// Upload progress list (right column - shows P2P/HTTP uploads)
const uploadProgressContainerRight = document.getElementById('upload-progress-container') as HTMLDivElement;
const uploadProgressListRight = document.getElementById('upload-progress-list') as HTMLDivElement;

// State
let serverRunning = false;
let currentNetworkInfo: NetworkInfo | null = null;
let currentContext: 'local' | 'internet' = 'local';
const copiedLinkFiles = new Map<string, number>();
let turnLimitWarningShown = false;
let headerStatusTimers: { left?: number; right?: number } = {};

// P2P
let p2pConfig: P2pConfig = {
    signalingUrl: '0.peerjs.com',
};
let peer: Peer | null = null;
let currentPeerId: string | null = null;
let connections: Map<string, DataConnection> = new Map();
let reconnectAttempts = 0;
let persistentIdRetryAttempts = 0;
const MAX_RECONNECT_ATTEMPTS = 10;
const MAX_PERSISTENT_ID_RETRIES = 5;
const PERSISTENT_ID_RETRY_DELAY = 5000;

// Incoming file transfer state (for P2P-to-P2P downloads)
interface IncomingFile {
    filename: string;
    size: number;
    hash: string;
    chunks: Map<number, Uint8Array>;
    receivedBytes: number;
    /** Transfer start timestamp (for real speed calculation) */
    startTime: number;
}
const incomingFiles = new Map<string, IncomingFile>();

// Incoming upload state (for Reverse Inbox) - incremental write to disk
interface IncomingUpload {
    filename: string;
    size: number;
    expectedHash: string;
    receivedBytes: number;
    /** Transfer start timestamp (for real speed calculation) */
    startTime: number;
    /** Flag to prevent double finalization (upload_end vs auto-finalize) */
    finalized: boolean;
    /** FIX: upload_complete idempotency. Prevents double send if both
     *  upload_end handler and auto-finalization try to confirm. */
    uploadCompleteSent: boolean;
    /** FIX: cancellation flag — chunks arriving after cancel are ignored */
    cancelled: boolean;
}
const incomingUploads = new Map<string, IncomingUpload>();

// FIX: idempotent helper to send upload_complete to the browser.
// Guarantees the message is sent exactly once, regardless of
// the code path (upload_end handler or auto-finalization).
function sendUploadComplete(conn: DataConnection, upload: IncomingUpload): void {
    if (upload.uploadCompleteSent) {
        log('ℹ️ upload_complete already sent, skipping');
        return;
    }
    upload.uploadCompleteSent = true;
    try {
        conn.send(JSON.stringify({ type: 'upload_complete' }));
        log('✅ upload_complete sent to browser');
    } catch (e) {
        log('❌ Could not send upload_complete: ' + getErrorMessage(e));
    }
}

// Per-peer message queue for serialization (avoids race condition: upload_end before last append)
const uploadMessageQueues = new Map<string, Promise<void>>();

// Track active WebRTC downloads to prevent duplicate progress bars
const activeWebRtcDownloads = new Set<string>();

// Track all active downloads for multi-download display
const activeDownloads = new Map<string, DownloadProgress>();

// Track all active uploads for multi-upload display
const activeUploads = new Map<string, UploadProgress>();

// Recent transfers log (resets on app restart - in-memory only)
interface TransferRecord {
    type: 'upload' | 'download';
    filename: string;
    size: number;
    timestamp: number;
}
const recentTransfers: TransferRecord[] = [];
const MAX_RECENT_TRANSFERS = 50;

// ---------- Connection path detection (LED green/yellow TURN) ----------
// Connection path per transfer key:
// 'direct' = P2P direct (host/srflx, no server bandwidth consumed)
// 'turn'   = traffic routed via TURN relay (~2x the file size)
const connectionPaths = new Map<string, 'direct' | 'turn'>();

const TOOLTIP_DIRECT = "Files don't consume server bandwidth. You can send files of any size.";
const TOOLTIP_TURN = "This file is consuming server bandwidth. Consumption is about twice the file size.";

/**
 * Detects the WebRTC connection path via getStats().
 * The correct chain is: selected candidate-pair -> local/remoteCandidateId
 * -> candidateType of each candidate (the field "candidateTypeLocal"
 * does NOT exist in standard stats).
 * - 'turn'   -> at least ONE of the two sides is a relay candidate
 * - 'direct' -> host/srflx/prflx (STUN only discovers the address, does not carry data)
 * - null     -> stats unavailable (badge remains hidden)
 */
async function detectConnectionPath(conn: DataConnection): Promise<'direct' | 'turn' | null> {
    try {
        // PeerJS exposes the RTCPeerConnection as a public property;
        // fallback to _pc for robustness across versions.
        const pc: RTCPeerConnection | null =
            (conn as any).peerConnection ?? (conn as any)._pc ?? null;
        if (!pc) return null;

        const stats = await pc.getStats();
        let pair: any = null;
        stats.forEach((report: any) => {
            if (report.type === 'candidate-pair' &&
                (report.selected === true || report.nominated === true || report.state === 'succeeded')) {
                pair = pair ?? report;
            }
        });
        if (!pair?.localCandidateId || !pair?.remoteCandidateId) return null;

        // RTCStatsReport implements Map but some lib.dom versions don't type it
        const statsMap = stats as unknown as Map<string, any>;
        const local = statsMap.get(pair.localCandidateId);
        const remote = statsMap.get(pair.remoteCandidateId);
        // Determina se è TURN in modo robusto:
        // 1. candidateType === 'relay' su uno dei due lati, OPPURE
        // 2. il campo url del candidato contiene 'turn:' (perché alcuni browser
        //    riportano candidateType come 'prflx' anche per connessioni TURN)
        const isRelay = (!!local && !!remote &&
            (local.candidateType === 'relay' || remote.candidateType === 'relay')) ||
            (local?.url && typeof local.url === 'string' && local.url.startsWith('turn:')) ||
            (remote?.url && typeof remote.url === 'string' && remote.url.startsWith('turn:'));
        return isRelay ? 'turn' : 'direct';
    } catch {
        return null;
    }
}

/** Updates the connection LED badge for a transfer row based on the known path.
 *  fileSize: optional. If passed and path === 'turn' and fileSize > TURN_MAX_FILE_SIZE,
 *  the badge turns red with an overlimit message.
 */
function applyConnBadge(badge: HTMLElement | null, key: string, fileSize?: number): void {
    if (!badge) return;
    const path = connectionPaths.get(key);
    if (path === 'turn') {
        // TURN: green if file is within limit, red if over limit.
        // Retrieve current limit from backend (cached in memory).
        const turnMax = (window as any).__turnMaxFileSize || 100 * 1024 * 1024;
        if (fileSize !== undefined && fileSize > turnMax) {
            badge.className = 'conn-badge turn-overlimit';
            badge.setAttribute('data-tooltip',
                'Your connection requires a relay server. To share this file, connect to WiFi.');
        } else {
            badge.className = 'conn-badge turn';
            badge.setAttribute('data-tooltip', TOOLTIP_TURN);
        }
    } else if (path === 'direct') {
        badge.className = 'conn-badge direct';
        badge.setAttribute('data-tooltip', TOOLTIP_DIRECT);
    }
    // path undefined: badge invisible (ICE negotiation in progress or stats absent)
}

// ---------- Global LED tooltip ----------
// A single position:fixed element on body: never clipped by overflow
// of scroll panels and always positioned inside the viewport.
const connTooltipEl = document.getElementById('conn-tooltip');

function hideConnTooltip(): void {
    connTooltipEl?.classList.remove('visible');
}

function showConnTooltip(badge: HTMLElement): void {
    if (!connTooltipEl) return;
    const text = badge.getAttribute('data-tooltip');
    if (!text) return;

    connTooltipEl.textContent = text;
    connTooltipEl.classList.add('visible');

    // Position relative to viewport (measure after text render)
    const rect = badge.getBoundingClientRect();
    const tw = connTooltipEl.offsetWidth;
    const th = connTooltipEl.offsetHeight;

    // Horizontal: aligned to the left edge of the LED, clamped to screen edges
    let left = Math.max(8, Math.min(rect.left, window.innerWidth - tw - 8));
    // Vertical: above the LED; if no space, below
    let top = rect.top - th - 10;
    if (top < 8) top = rect.bottom + 10;

    connTooltipEl.style.left = `${left}px`;
    connTooltipEl.style.top = `${top}px`;
}

// Delegated listeners: transfer rows are created/removed dynamically
document.addEventListener('mouseover', (event) => {
    const target = event.target as HTMLElement | null;
    const badge = target?.closest?.('.conn-badge');
    if (badge) showConnTooltip(badge as HTMLElement);
});
document.addEventListener('mouseout', (event) => {
    const target = event.target as HTMLElement | null;
    if (target?.closest?.('.conn-badge')) hideConnTooltip();
});
// Hide the tooltip if the window resizes (position no longer valid)
window.addEventListener('resize', hideConnTooltip);

// ---------- Utility ----------
function escapeHtml(text: string): string {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
}

function getErrorMessage(error: unknown): string {
    if (typeof error === 'string') return error;
    if (error instanceof Error) return error.message;
    if (error && typeof error === 'object' && 'message' in error) {
        return String((error as { message: unknown }).message);
    }
    return 'Unknown error';
}

type HeaderStatusType = 'success' | 'error' | 'info' | 'warning';

function showHeaderStatus(message: string, type: HeaderStatusType): void {
    const isLeft = (type === 'success' || type === 'info');
    const slotId = isLeft ? 'header-status-left' : 'header-status-right';
    const slot = document.getElementById(slotId);
    if (!slot) return;

    const timerKey = isLeft ? 'left' : 'right';
    if (headerStatusTimers[timerKey]) {
        clearTimeout(headerStatusTimers[timerKey]);
    }

    slot.textContent = message;
    slot.className = `header-status ${isLeft ? 'header-status-left' : 'header-status-right'}`;
    if (type === 'info') slot.classList.add('header-status-info');
    if (type === 'warning') slot.classList.add('header-status-warning');

    void slot.offsetWidth;
    slot.classList.add('visible');

    const duration = (type === 'success' || type === 'info') ? 2500 : 5000;
    headerStatusTimers[timerKey] = window.setTimeout(() => {
        slot.textContent = '';
        slot.className = `header-status ${isLeft ? 'header-status-left' : 'header-status-right'}`;
        headerStatusTimers[timerKey] = undefined;
    }, duration);
}

function formatSize(bytes: number): string {
    const units = ['B', 'KB', 'MB', 'GB', 'TB'];
    if (bytes === 0) return '0 B';
    const k = 1024;
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    const size = bytes / Math.pow(k, i);
    return `${size.toFixed(1)} ${units[i]}`;
}

function formatDate(dateString: string): string {
    try {
        const date = new Date(dateString);
        if (isNaN(date.getTime())) return 'Unknown date';
        return date.toLocaleDateString('en-US', {
            day: '2-digit', month: '2-digit', year: 'numeric',
            hour: '2-digit', minute: '2-digit',
        });
    } catch {
        return 'Unknown date';
    }
}

function showNotification(title: string, body: string): void {
    try {
        sendNotification({ title, body });
    } catch (error) {
        console.error('Error sending notification:', error);
    }
}

function updateServerLed(running: boolean): void {
    serverRunning = running;
    const cls = running ? 'led-on' : 'led-off';
    const txt = running ? 'Active' : 'Inactive';
    if (serverLed) {
        serverLed.className = `led-indicator ${cls}`;
        serverStatusText.textContent = txt;
    }
    if (serverLedSmall) {
        serverLedSmall.className = `led-indicator ${cls}`;
        serverStatusSmall.textContent = txt;
    }
}

// ---------- Upload Progress (hidden until there are uploads) ----------
// NOTE: each row is created ONCE per transfer key and then updated IN PLACE.
// Rebuilding the whole list with innerHTML on every progress event replaced
// the cancel button between mousedown and mouseup, so single clicks were
// silently dropped (the click event fired on the container, not the button).
function renderUploadProgressList(): void {
    if (!uploadProgressListRight) return;

    if (activeUploads.size === 0) {
        // Hide the container when there are no uploads
        if (uploadProgressContainerRight) {
            uploadProgressContainerRight.classList.add('hidden');
        }
        // Show empty state message
        uploadProgressListRight.innerHTML = '<div class="download-item no-downloads">No active uploads</div>';
        return;
    }

    // Show the container when there are uploads
    if (uploadProgressContainerRight) {
        uploadProgressContainerRight.classList.remove('hidden');
    }

    // Remove the empty-state message if present
    const emptyMsg = uploadProgressListRight.querySelector('.no-downloads');
    if (emptyMsg) emptyMsg.remove();

    // Index existing rows by key so they can be updated in place
    const staleRows = new Map<string, HTMLElement>();
    uploadProgressListRight.querySelectorAll<HTMLElement>('.upload-item').forEach(row => {
        const id = row.getAttribute('data-upload-id');
        if (id) staleRows.set(id, row);
    });

    for (const [key, u] of activeUploads) {
        const progress = u.total_bytes > 0 ? Math.min(u.progress, 100) : 0;
        // speed_mbps is provided by the backend (elapsed-time based); never fake it
        // with bytes_processed (that would show the file SIZE as the speed).
        const speed = u.speed_mbps ?? 0;
        const progressText = u.total_bytes === 0 ? '…' : `${Math.round(progress)}%`;

        let row = staleRows.get(key);
        if (!row) {
            // Create the row skeleton once; fields are updated below via textContent
            row = document.createElement('div');
            row.className = 'upload-item';
            row.setAttribute('data-upload-id', key);
            row.innerHTML = `
                <div class="download-header">
                    <span class="conn-badge" data-tooltip="${TOOLTIP_DIRECT}"></span>
                    <span class="download-filename"></span>
                    <span class="download-percentage"></span>
                    <button class="cancel-btn" title="Cancel upload">✕</button>
                </div>
                <div class="progress-bar">
                    <div class="progress-fill"></div>
                </div>
                <div class="download-details">
                    <span></span>
                    <span></span>
                </div>
            `;
            uploadProgressListRight.appendChild(row);
        }
        staleRows.delete(key);

        // In-place field updates (the cancel button node is NEVER replaced)
        const filenameEl = row.querySelector<HTMLElement>('.download-filename');
        if (filenameEl) filenameEl.textContent = u.filename;
        const percentageEl = row.querySelector<HTMLElement>('.download-percentage');
        if (percentageEl) percentageEl.textContent = progressText;
        const cancelBtn = row.querySelector<HTMLButtonElement>('.cancel-btn');
        if (cancelBtn) cancelBtn.setAttribute('data-upload-key', key);
        const fillEl = row.querySelector<HTMLElement>('.progress-fill');
        if (fillEl) fillEl.style.width = `${progress}%`;
        const detailEls = row.querySelectorAll<HTMLElement>('.download-details span');
        if (detailEls[0]) detailEls[0].textContent = `${formatSize(u.bytes_processed)} / ${formatSize(u.total_bytes)}`;
        if (detailEls[1]) detailEls[1].textContent = `${speed.toFixed(1)} MB/s`;

        // Connection LED (green = direct, yellow = TURN, red = over limit)
        // FIX: pass u.total_bytes so the SENDER's badge also turns red when the
        // file exceeds the TURN size limit. Previously only the browser receiver
        // (web-receiver.html) got the red error message; the sender's UI was silent.
        applyConnBadge(row.querySelector<HTMLElement>('.conn-badge'), key, u.total_bytes);
    }

    // FIX: SENDER feedback for TURN size limit. When a badge is red (overlimit),
    // show a status message in the footer so the sender knows the transfer was
    // blocked. The browser receiver already shows the error, but the sender's UI
    // had no indication.
    const turnOverlimitRows = uploadProgressListRight.querySelectorAll<HTMLElement>('.conn-badge.turn-overlimit');
    if (turnOverlimitRows.length > 0 && !turnLimitWarningShown) {
        turnLimitWarningShown = true;
        const firstRow = turnOverlimitRows[0].closest<HTMLElement>('.upload-item');
        if (firstRow && !firstRow.classList.contains('turn-limit-warned')) {
            firstRow.classList.add('turn-limit-warned');
            const filename = firstRow.querySelector<HTMLElement>('.download-filename')?.textContent || '';
            showHeaderStatus(`⚠️ File exceeds TURN size limit: ${filename}`, 'warning');
        }
    } else if (turnOverlimitRows.length === 0) {
        turnLimitWarningShown = false;
        // Clear any previous TURN limit warning if no overlimit rows remain
        const warned = uploadProgressListRight.querySelector<HTMLElement>('.upload-item.turn-limit-warned');
        if (warned) {
            warned.classList.remove('turn-limit-warned');
        }
    }

    // Remove rows whose transfer is no longer active
    staleRows.forEach(row => row.remove());
}

// ---------- Download Progress (left column - shows inbox downloads) ----------
function resetDownloadProgress(): void {
    if (downloadProgressListLeft) {
        downloadProgressListLeft.innerHTML = '<div class="download-item no-downloads"><span class="download-filename">No downloads in progress</span></div>';
    }
    if (downloadProgressContainerLeft) {
        downloadProgressContainerLeft.classList.add('hidden');
    }
}

function showDownloadActive(filename: string): void {
    // No longer needed - downloads are shown directly in the list
}

function updateDownloadProgress(downloads: DownloadProgress[]): void {
    if (!downloads || downloads.length === 0) {
        resetDownloadProgress();
        return;
    }

    if (!downloadProgressListLeft) return;

    // Show the download progress container when there are active downloads
    if (downloadProgressContainerLeft) {
        downloadProgressContainerLeft.classList.remove('hidden');
    }

    // Remove the empty-state message if present
    const emptyMsg = downloadProgressListLeft.querySelector('.no-downloads');
    if (emptyMsg) emptyMsg.remove();

    // Index existing rows by hash so they can be updated in place
    const staleRows = new Map<string, HTMLElement>();
    downloadProgressListLeft.querySelectorAll<HTMLElement>('.download-item').forEach(row => {
        const id = row.getAttribute('data-download-id');
        if (id) staleRows.set(id, row);
    });

    for (const d of downloads) {
        const eta = d.speed_mbps > 0
            ? Math.ceil((d.total_bytes - d.downloaded_bytes) / (1024 * 1024) / d.speed_mbps)
            : 0;

        let row = staleRows.get(d.hash);
        if (!row) {
            // Create the row skeleton once; fields are updated below via textContent
            row = document.createElement('div');
            row.className = 'download-item';
            row.setAttribute('data-download-id', d.hash);
            row.innerHTML = `
                <div class="download-header">
                    <span class="conn-badge" data-tooltip="${TOOLTIP_DIRECT}"></span>
                    <span class="download-filename"></span>
                    <span class="download-percentage"></span>
                    <button class="cancel-btn" title="Cancel download">✕</button>
                </div>
                <div class="progress-bar">
                    <div class="progress-fill"></div>
                </div>
                <div class="download-details">
                    <span></span>
                    <span></span>
                </div>
            `;
            downloadProgressListLeft.appendChild(row);
        }
        staleRows.delete(d.hash);

        // In-place field updates (the cancel button node is NEVER replaced)
        const filenameEl = row.querySelector<HTMLElement>('.download-filename');
        if (filenameEl) filenameEl.textContent = d.filename;
        const percentageEl = row.querySelector<HTMLElement>('.download-percentage');
        if (percentageEl) percentageEl.textContent = `${Math.round(d.progress)}%`;
        const cancelBtn = row.querySelector<HTMLButtonElement>('.cancel-btn');
        if (cancelBtn) cancelBtn.setAttribute('data-download-key', d.hash);
        const fillEl = row.querySelector<HTMLElement>('.progress-fill');
        if (fillEl) fillEl.style.width = `${Math.min(Math.round(d.progress), 100)}%`;
        const detailEls = row.querySelectorAll<HTMLElement>('.download-details span');
        if (detailEls[0]) detailEls[0].textContent = `${formatSize(d.downloaded_bytes)} / ${formatSize(d.total_bytes)}`;
        if (detailEls[1]) detailEls[1].textContent = `${d.speed_mbps.toFixed(1)} MB/s • ${eta}s`;

        // Connection LED (green = direct, yellow = TURN, red = over limit)
        // FIX: pass d.total_bytes so the red badge also appears on the RECEIVER's
        // download progress bar when the file exceeds the TURN size limit.
        applyConnBadge(row.querySelector<HTMLElement>('.conn-badge'), d.hash, d.total_bytes);
    }

    // Remove rows whose transfer is no longer active
    staleRows.forEach(row => row.remove());
}

// ---------- Recent Transfers (log, resets on restart) ----------
function addRecentTransfer(type: 'upload' | 'download', filename: string, size: number): void {
    recentTransfers.unshift({
        type,
        filename,
        size,
        timestamp: Date.now()
    });
    // Keep only the most recent MAX_RECENT_TRANSFERS entries
    if (recentTransfers.length > MAX_RECENT_TRANSFERS) {
        recentTransfers.length = MAX_RECENT_TRANSFERS;
    }
    renderRecentTransfers();
}

function renderRecentItem(t: TransferRecord, now: number, twoMin: number): string {
    const icon = t.type === 'upload' ? '📤' : '📥';
    const label = t.type === 'upload' ? 'Upload' : 'Download';
    const time = new Date(t.timestamp).toLocaleTimeString('it-IT', {
        hour: '2-digit', minute: '2-digit', second: '2-digit'
    });
    // Highlight transfers made in the last 2 minutes
    const freshClass = (now - t.timestamp) < twoMin ? ' recent-fresh' : '';
    return `
        <div class="recent-item recent-${t.type}${freshClass}">
            <span class="recent-icon">${icon}</span>
            <span class="recent-label">${label}</span>
            <span class="recent-filename" title="${escapeHtml(t.filename)}">${escapeHtml(t.filename)}</span>
            <span class="recent-size">${formatSize(t.size)}</span>
            <span class="recent-time">${time}</span>
        </div>
    `;
}

function renderRecentTransfers(): void {
    const downloadsEl = document.getElementById('recent-downloads-list');
    const uploadsEl = document.getElementById('recent-uploads-list');
    if (!downloadsEl || !uploadsEl) return;

    const now = Date.now();
    const TWO_MIN = 2 * 60 * 1000;

    const downloads = recentTransfers.filter(t => t.type === 'download');
    const uploads = recentTransfers.filter(t => t.type === 'upload');

    downloadsEl.innerHTML = downloads.length === 0
        ? '<div class="recent-empty">No downloads yet</div>'
        : downloads.map(t => renderRecentItem(t, now, TWO_MIN)).join('');

    uploadsEl.innerHTML = uploads.length === 0
        ? '<div class="recent-empty">No uploads yet</div>'
        : uploads.map(t => renderRecentItem(t, now, TWO_MIN)).join('');
}

// ---------- File list (columns + sorting) ----------
function getFilesFromMap(): FileInfo[] {
    return Array.from(window.fileMap.values());
}

function sortFiles(files: FileInfo[], sortBy: string): FileInfo[] {
    const sorted = [...files];
    if (sortBy === 'name') {
        sorted.sort((a, b) => a.filename.localeCompare(b.filename));
    } else if (sortBy === 'size') {
        sorted.sort((a, b) => b.size - a.size);
    } else if (sortBy === 'date') {
        sorted.sort((a, b) => b.uploaded_at.localeCompare(a.uploaded_at));
    }
    return sorted;
}

function applyColumnLayout(): void {
    const value = columnSelect.value;
    filesList.classList.remove('cols-1', 'cols-2', 'cols-3');
    if (value === '1' || value === '2' || value === '3') {
        filesList.classList.add(`cols-${value}`);
    }
}

async function loadFiles(): Promise<void> {
    filesList.innerHTML = '<div class="loading">⏳ Loading...</div>';
    try {
        const files = await invoke<FileInfo[]>('list_files');
        if (!files || files.length === 0) {
            filesList.innerHTML = '<div class="loading">📂 No shared files</div>';
            return;
        }
        window.fileMap = new Map<string, FileInfo>();
        files.forEach(f => window.fileMap.set(f.hash, f));
        renderFiles();
    } catch (error) {
        filesList.innerHTML = `<div class="loading">❌ Error: ${getErrorMessage(error)}</div>`;
    }
}

async function handleRefresh(): Promise<void> {
    filesList.innerHTML = '<div class="loading">⏳ Refreshing...</div>';
    try {
        const files = await invoke<FileInfo[]>('refresh_files');
        if (!files || files.length === 0) {
            filesList.innerHTML = '<div class="loading">📂 No shared files</div>';
            return;
        }
        window.fileMap = new Map<string, FileInfo>();
        files.forEach(f => window.fileMap.set(f.hash, f));
        renderFiles();
    } catch (error) {
        filesList.innerHTML = `<div class="loading">❌ Error: ${getErrorMessage(error)}</div>`;
    }
}

function renderFiles(): void {
    const sortBy = sortSelect.value || 'name';
    const sorted = sortFiles(getFilesFromMap(), sortBy);
    applyColumnLayout();

    if (sorted.length === 0) {
        filesList.innerHTML = '<div class="loading">📂 No shared files</div>';
        return;
    }

    filesList.innerHTML = sorted.map(file => {
        const now = Date.now();
        const copiedTime = copiedLinkFiles.get(file.hash);
        const isCopied = copiedTime && (now - copiedTime < 10000);
        const statusIcon = isCopied ? '✅' : '📄';
        return `
            <div class="file-item" data-hash="${escapeHtml(file.hash)}">
                <div class="file-info">
                    <div class="file-name">${escapeHtml(file.filename)}</div>
                    <div class="file-meta">${formatSize(file.size)} • ${formatDate(file.uploaded_at)}</div>
                </div>
                <span class="file-status">${statusIcon}</span>
            </div>
        `;
    }).join('');

    document.querySelectorAll('.file-item').forEach(item => {
        item.addEventListener('click', () => {
            const hash = (item as HTMLElement).dataset.hash;
            if (hash) {
                document.querySelectorAll('.file-item').forEach(i => i.classList.remove('selected'));
                item.classList.add('selected');
            }
        });
    });
}

// ---------- Upload ----------
async function handleUpload(filePath: string): Promise<void> {
    uploadBtn.disabled = true;
    uploadBtn.textContent = '⏳ Loading...';
    uploadStatus.className = 'status';
    const filename = filePath.split(/[\\/]/).pop() || 'file';

    try {
        uploadStatus.textContent = '📁 Processing file...';
        const hash = await invoke<string>('register_file', { filePath });

        uploadStatus.textContent = `✅ ${filename} uploaded successfully! (Hash: ${hash.substring(0, 16)}...)`;
        uploadStatus.className = 'status success';
        showNotification('Upload complete', `${filename} has been uploaded successfully`);
        addRecentTransfer('download', filename, 0);
        await loadFiles();
    } catch (error) {
        console.error('Error in handleUpload:', error);
        uploadStatus.textContent = `❌ Error: ${getErrorMessage(error)}`;
        uploadStatus.className = 'status error';
        showNotification('Upload error', getErrorMessage(error));
    } finally {
        uploadBtn.disabled = false;
        uploadBtn.textContent = '📤 Select File';
    }
}

async function handleSelectFile(): Promise<void> {
    try {
        const filePath = await open({
            multiple: false,
            filters: [{ name: 'All Files', extensions: ['*'] }],
        });
        if (filePath) {
            await handleUpload(filePath);
        }
    } catch (error) {
        uploadStatus.textContent = `❌ Error: ${getErrorMessage(error)}`;
        uploadStatus.className = 'status error';
    }
}

// ---------- Network / Link ----------
async function loadNetworkInfo(): Promise<void> {
    try {
        const info = await invoke<NetworkInfo>('get_network_info');
        currentNetworkInfo = info;
        localIpEl.textContent = info.ip;
    } catch (error) {
        localIpEl.textContent = 'Error';
        console.error('Network error:', error);
    }
}

async function handleStartServer(): Promise<void> {
    // Button removed from UI: this function is called by ensureServerRunning()
    if (!startServerBtn) {
        try {
            await invoke<string>('start_http_server');
            updateServerLed(true);
        } catch (error) {
            console.error('Error starting server:', error);
        }
        return;
    }
    startServerBtn.disabled = true;
    if (serverRunning) {
        startServerBtn.textContent = '⏳ Stopping...';
        try {
            await invoke<string>('stop_http_server');
            updateServerLed(false);
            startServerBtn.textContent = '▶️ Start Server';
        } catch (error) {
            startServerBtn.textContent = '🛑 Stop Server';
            console.error('Error stopping server:', error);
        }
    } else {
        startServerBtn.textContent = '⏳ Starting...';
        try {
            await invoke<string>('start_http_server');
            updateServerLed(true);
            startServerBtn.textContent = '🛑 Stop Server';
        } catch (error) {
            startServerBtn.textContent = '▶️ Start Server';
            console.error('Error starting server:', error);
        }
    }
    startServerBtn.disabled = false;
}

// Automatically starts the local HTTP server if not already running.
// Called by Generate Link / Create Inbox: the link embeds the LAN fallback
// (lan=http://IP:3000) only when this server is running.
async function ensureServerRunning(): Promise<void> {
    if (serverRunning) return;
    try {
        await invoke<string>('start_http_server');
        updateServerLed(true);
        log('🟢 Local server auto-started (LAN fallback enabled)');
    } catch (error) {
        log('⚠️ Local server auto-start failed: ' + getErrorMessage(error));
    }
}

// Generate local HTTP link for LAN network
async function handleGenerateLocalLink(): Promise<void> {
    const selectedHash = getSelectedHash();
    if (!selectedHash) {
        showHeaderStatus('❌ Select a file first', 'error');
        return;
    }
    generateLocalLinkBtn.disabled = true;
    generateLocalLinkBtn.textContent = '⏳ Generating...';
    try {
        const link = await invoke<string>('generate_local_link', { hash: selectedHash });
        if (localLinkDisplay) localLinkDisplay.textContent = link;
        if (localLinkContainer) localLinkContainer.classList.remove('hidden');
        showHeaderStatus('✅ Local link generated!', 'success');
        // Mark file as having its link generated (shows checkmark in file list)
        copiedLinkFiles.set(selectedHash, Date.now());
        renderFiles();
        // Reset checkmark after 10 seconds
        setTimeout(() => {
            copiedLinkFiles.delete(selectedHash);
            renderFiles();
        }, 10000);
    } catch (error) {
        showHeaderStatus(`❌ ${getErrorMessage(error)}`, 'error');
    }
    generateLocalLinkBtn.disabled = false;
    generateLocalLinkBtn.textContent = '🔗 Generate Local Link';
}

async function handleOpenFolder(): Promise<void> {
    try {
        await invoke('open_shared_folder');
        showHeaderStatus('✅ Shared folder opened', 'success');
    } catch (error) {
        showHeaderStatus(`❌ ${getErrorMessage(error)}`, 'error');
    }
}

// ---------- Drag & Drop ----------
function handleDragEnter(e: DragEvent): void {
    e.preventDefault(); e.stopPropagation();
    dropZone.classList.add('drag-over');
    dropOverlay.classList.remove('hidden');
}
function handleDragOver(e: DragEvent): void {
    e.preventDefault(); e.stopPropagation();
}
function handleDragLeave(e: DragEvent): void {
    e.preventDefault(); e.stopPropagation();
    const rect = dropZone.getBoundingClientRect();
    if (e.clientX <= rect.left || e.clientX >= rect.right || e.clientY <= rect.top || e.clientY >= rect.bottom) {
        dropZone.classList.remove('drag-over');
        dropOverlay.classList.add('hidden');
    }
}
async function handleDrop(e: DragEvent): Promise<void> {
    e.preventDefault(); e.stopPropagation();
    dropZone.classList.remove('drag-over');
    dropOverlay.classList.add('hidden');
    const files = e.dataTransfer?.files;
    if (!files || files.length === 0) {
        uploadStatus.textContent = '❌ No file dropped';
        uploadStatus.className = 'status error';
        return;
    }
    for (let i = 0; i < files.length; i++) {
        const path = (files[i] as any).path;
        if (path) await handleUpload(path);
    }
}

// ---------- Context (unified) ----------
// With "smart" links there is no longer a local/internet choice for the
// user: the path (LAN -> STUN -> TURN) is decided automatically.
// This function remains for compatibility but only ensures that all
// Sharing panel sections are visible and updates the relay status.
function updateContext(_context?: 'local' | 'internet'): void {
    ['local-context', 'web-link-section', 'internet-inbox-section',
     'row-peer-id', 'row-relay', 'connect-peer-container',
     'connection-status', 'peers-container'].forEach(id => {
        document.getElementById(id)?.classList.remove('hidden');
    });

    if (relayStatusEl) relayStatusEl.textContent = peer ? 'Active' : 'Waiting';
}

// ---------- P2P (Phase 4) ----------
// TURN credentials loaded from environment variables (Vite: VITE_TURN_USERNAME /
// VITE_TURN_PASSWORD) to avoid exposing secrets in source code.
// If not configured, only STUN is used (no TURN relay).
const TURN_USERNAME = import.meta.env.VITE_TURN_USERNAME || '';
const TURN_PASSWORD = import.meta.env.VITE_TURN_PASSWORD || '';

/**
 * Masks a Peer ID for display in user-visible logs.
 * Shows only the first 12 characters, the rest becomes "...".
 * E.g.: "peerino-74181e9e-1e4e-40e3-b7c0-134bced532ea" → "peerino-74181e9e-1e..."
 * The full Peer ID remains available in console.log for debugging.
 */
function maskPeerId(peerId: string | undefined | null): string {
    if (!peerId) return '(null)';
    if (peerId.length <= 12) return peerId;
    return peerId.slice(0, 12) + '...';
}

const iceServers: RTCIceServer[] = [
    { urls: 'stun:stun.l.google.com:19302' },
    { urls: 'stun:stun1.l.google.com:19302' },
];

// Add TURN server only if credentials are available.
if (TURN_USERNAME && TURN_PASSWORD) {
    iceServers.push({
        urls: [
            'stun:stun.relay.metered.ca:80',
            'turn:global.relay.metered.ca:80',
            'turn:global.relay.metered.ca:80?transport=tcp',
            'turn:global.relay.metered.ca:443',
            'turns:global.relay.metered.ca:443?transport=tcp',
        ],
        username: TURN_USERNAME,
        credential: TURN_PASSWORD,
    });
}

async function initPeer(forceRandom: boolean = false): Promise<void> {
    try {
        // Prevent leak: destroy any existing peer
        if (peer) {
            try { peer.destroy(); } catch (e) {}
            peer = null;
        }

        const isSelfHosted = p2pConfig.signalingUrl !== undefined && !p2pConfig.signalingUrl.includes('peerjs.com');

        // FIX: use a stable PeerID persisted on disk so generated links survive
        // app restarts. Without this, PeerJS assigns a new random ID every
        // launch, orphaning old links (receiver hangs on "Connecting to sender...").
        let peerIdArg: string | undefined = undefined;
        if (!forceRandom) {
            try {
                const persistentId = await invoke<string>('get_persistent_peer_id');
                if (persistentId) peerIdArg = persistentId;
            } catch (e) {
                log('⚠️ Could not read persistent PeerID, using random: ' + getErrorMessage(e));
            }
        }

        const peerOptions = {
            host: p2pConfig.signalingUrl || '0.peerjs.com',
            port: 443,
            path: '/',
            config: { iceServers },
            secure: isSelfHosted,
            debug: 2,
        };
        peer = peerIdArg ? new Peer(peerIdArg, peerOptions) : new Peer(peerOptions);

        peer.on('open', (id) => {
            currentPeerId = id;
            reconnectAttempts = 0; // Reset reconnect counter
            persistentIdRetryAttempts = 0; // Reset persistent ID retry counter
            if (myPeerIdEl) myPeerIdEl.textContent = id;
            if (relayStatusEl) relayStatusEl.textContent = 'Active';
            // Update the PeerID in backend for web link generation
            invoke('set_peer_id', { peerId: id }).catch(console.error);
            log('✅ PeerJS connected with ID: ' + maskPeerId(id) + (isSelfHosted ? ' (self-hosted)' : ' (cloud)') + (peerIdArg ? ' (persistent)' : ' (random)'));
            console.log('Full PeerJS ID (debug only):', id);
        });
        peer.on('connection', (conn: DataConnection) => {
            log('📥 Connection from: ' + conn.peer);
            handleIncomingConnection(conn);
        });
        peer.on('error', (err) => {
            log('❌ PeerJS error: ' + err);
            const msg = (err && typeof err.message === 'string') ? err.message : '';
            if (msg.includes('Lost connection to server')) {
                if (reconnectAttempts < MAX_RECONNECT_ATTEMPTS) {
                    reconnectAttempts++;
                    const delay = Math.min(2000 * Math.pow(1.5, reconnectAttempts - 1), 30000);
                    log(`⏳ PeerJS reconnecting in ${delay}ms (attempt ${reconnectAttempts}/${MAX_RECONNECT_ATTEMPTS})`);
                    setTimeout(() => {
                        if (peer) {
                            try { peer.destroy(); } catch (e) {}
                            peer = null;
                        }
                        initPeer();
                    }, delay);
                    updateConnectionStatus('pending', `Reconnecting... (${reconnectAttempts}/${MAX_RECONNECT_ATTEMPTS})`);
                } else {
                    log('❌ Could not reconnect to PeerJS.');
                    updateConnectionStatus('error', 'Could not connect to signaling server.');
                }
            } else if ((msg.includes('taken') || msg.toLowerCase().includes('id is taken')) && !forceRandom) {
                // Persistent PeerID is taken on the signaling server (old session not released yet).
                // Instead of immediately falling back to random ID, retry with the persistent ID
                // a few times so the server has time to release the old session.
                if (persistentIdRetryAttempts < MAX_PERSISTENT_ID_RETRIES) {
                    persistentIdRetryAttempts++;
                    log(`⚠️ Persistent ID taken, retrying in ${PERSISTENT_ID_RETRY_DELAY}ms (attempt ${persistentIdRetryAttempts}/${MAX_PERSISTENT_ID_RETRIES})`);
                    setTimeout(() => {
                        if (peer) { try { peer.destroy(); } catch (e) {} peer = null; }
                        initPeer();
                    }, PERSISTENT_ID_RETRY_DELAY);
                    updateConnectionStatus('pending', `Retrying ID... (${persistentIdRetryAttempts}/${MAX_PERSISTENT_ID_RETRIES})`);
                } else {
                    log('⚠️ Persistent ID still in use after retries, falling back to random ID for this session');
                    if (peer) { try { peer.destroy(); } catch (e) {} peer = null; }
                    initPeer(true);
                }
            } else {
                updateConnectionStatus('error', 'P2P Error');
            }
        });
    } catch (error) {
        log('❌ Failed to initialize PeerJS: ' + getErrorMessage(error));
    }
}

// Restore connections after hibernation/suspension (system-resumed event from backend)
async function handleSystemResume(): Promise<void> {
    // 1. Restart HTTP server (if it was running)
    if (serverRunning) {
        try {
            await invoke('stop_http_server').catch(() => {});
        } catch (e) {}
        try {
            await invoke('start_http_server');
            console.log('✅ HTTP server restarted after hibernation');
            updateServerLed(true); // existing UI function (also updates serverRunning)
        } catch (e) {
            console.error('❌ Error restarting server:', e);
            serverRunning = false;
            updateServerLed(false);
        }
    }

    // 2. Reconnect PeerJS – destroy and reinitialize
    if (peer) {
        try { peer.destroy(); } catch (e) {}
        peer = null;
    }
    initPeer(); // Reuse existing function (don't rewrite it)

    // 3. Update UI
    await loadFiles();
    await loadNetworkInfo();
    // Reset upload progress on system resume
    activeUploads.clear();
    renderUploadProgressList();
    showNotification('🔄 System restored', 'Connections re-established.');
}

// Stream file to a specific connection (used for P2P-to-Web)
// Protocol: sends 'file_meta' (JSON string) then raw binary chunks
// Uses 256KB chunks and backpressure via bufferedAmount
async function streamFileToConnection(conn: DataConnection, hash: string): Promise<void> {
    // FIX #1: Validate against backend (get_file_info) instead of window.fileMap
    // This ensures the file actually exists on disk, not just in the frontend index
    let fileInfo: { filename: string; size: number; hash: string } | null = null;
    try {
        const info = await invoke<{ filename: string; size: number; hash: string } | null>('get_file_info', { hash });
        if (info) {
            fileInfo = info;
        }
    } catch (e) {
        // get_file_info throws if file not found
    }

    if (!fileInfo) {
        const errorMsg = `File not found for hash: ${hash}`;
        log('❌ ' + errorMsg);
        // FIX #2: Notify receiver of the error instead of silent return
        try {
            conn.send(JSON.stringify({ type: 'error', message: errorMsg }));
        } catch (e) {
            log('❌ Could not send error to receiver: ' + getErrorMessage(e));
        }
        return;
    }

    // Create unique key for this upload (peer + hash) to support multiple simultaneous uploads
    const uploadKey = conn.peer + '-' + fileInfo.hash;

    // TURN size limit (Phase 2): blocks files exceeding TURN_MAX_FILE_SIZE on TURN.
    // FIX CRITICAL #1: ICE path detection is retried up to
    // 10 times (1s interval) instead of a single call. A single
    // getStats() right after connection open often returns
    // null (candidate-pair not yet selected), and in that case the limit
    // is not applied, allowing files >100MB to transfer over TURN.
    // FIX LOW #11: use the __turnMaxFileSize cache (loaded at startup)
    // instead of calling the get_turn_limits IPC every time. Avoids 2 redundant IPCs.
    const turnMaxSize = (window as any).__turnMaxFileSize || 100 * 1024 * 1024;
    // Fallback: if the cache is not ready yet, try loading it once
    if (!(window as any).__turnMaxFileSize) {
        try {
            const limits = await invoke<{ max_file_size: number }>('get_turn_limits');
            (window as any).__turnMaxFileSize = limits.max_file_size;
        } catch { /* best-effort */ }
    }

    // Poll ICE stats with retries (up to 10 attempts, 1s apart).
    // FIX: if the loop exhausts without a result, assume 'turn'
    // (maximum caution) to prevent files >100MB from passing over TURN
    // when detection fails. An undetected path is suspicious and may
    // hide a relay.
    // Path detection con gate condizionale:
    // - File > turnMaxSize: serve il gate (potenziale rifiuto TURN)
    // - File <= turnMaxSize: salta il gate, invia metadata subito.
    //   Il badge LED verrà aggiornato dal pollTurnPath esistente
    //   (righe ~1319-1335), che gira in background dopo l'invio del metadata.
    if (fileInfo.size > turnMaxSize) {
        let detectedPath: 'direct' | 'turn' | null = null;
        // Check sincrono: se la connessione è già stabile, il primo tentativo
        // dovrebbe riuscire immediatamente. Riduciamo il loop a 5s totali
        // con polling ogni 250ms (20 tentativi) invece di 10s (10 tentativi).
        const deadline = Date.now() + 5000;
        while (Date.now() < deadline) {
            detectedPath = await detectConnectionPath(conn).catch(() => null);
            if (detectedPath) break;
            await new Promise<void>(r => setTimeout(r, 250));
        }
        if (detectedPath === null) {
            log('⚠️ detectConnectionPath: timeout (5s). Assuming TURN for safety.');
            detectedPath = 'turn';
        }

        // Gate TURN (invariato): rifiuta solo se il file supera il limite.
        if (detectedPath === 'turn') {
            const errMsg = 'Your connection requires a relay server. To share this file, connect to WiFi.';
            log('TURN size limit exceeded: ' + errMsg);
            // Telemetry: record the rejection on the backend side.
            invoke('record_turn_rejection_cmd').catch(() => { /* best-effort */ });
            // FIX: also set the path to 'turn' for the LED badge
            connectionPaths.set(uploadKey, 'turn');
            activeUploads.set(uploadKey, {
                hash: fileInfo.hash,
                filename: fileInfo.filename,
                bytes_processed: 0,
                total_bytes: fileInfo.size,
                progress: 0,
                speed_mbps: 0,
                peer_id: conn.peer,
                cancelled: false
            });
            renderUploadProgressList();
            // Update the badge to red (overlimit)
            const badge = uploadProgressListRight?.querySelector<HTMLElement>('[data-upload-id="' + CSS.escape(uploadKey) + '"] .conn-badge');
            if (badge) {
                badge.className = 'conn-badge turn-overlimit';
                badge.setAttribute('data-tooltip',
                    'Your connection requires a relay server. To share this file, connect to WiFi.');
            }
            try {
                conn.send(JSON.stringify({
                    type: 'error',
                    reason: 'turn_size_limit',
                    message: errMsg,
                    max_size: turnMaxSize,
                    file_size: fileInfo.size,
                }));
            } catch (e) {
                log('❌ Could not send turn_size_limit error: ' + getErrorMessage(e));
            }
            return;
        }
        // Path detected as 'direct': set the green badge
        if (detectedPath === 'direct') {
            connectionPaths.set(uploadKey, 'direct');
            renderUploadProgressList();
        }
        // detectedPath === 'turn' but file <= limit: set yellow badge
        if (detectedPath === 'turn' && fileInfo.size <= turnMaxSize) {
            connectionPaths.set(uploadKey, 'turn');
            renderUploadProgressList();
        }
    } else {
        log(`ℹ️ File ${fileInfo.size} <= TURN limit ${turnMaxSize}: skipping path-detection gate, sending metadata immediately`);
    }

    try {
        // 1. Send file metadata as JSON string
        const meta = {
            type: 'file_meta',
            filename: fileInfo.filename,
            size: fileInfo.size,
            hash: fileInfo.hash
        };
        conn.send(JSON.stringify(meta));
        log(`📤 Metadata sent: ${fileInfo.filename} (${fileInfo.size} bytes) to ${conn.peer}`);

        // 2. Stream file data in 256KB chunks (raw binary) with backpressure
        let offset = 0;
        const totalSize = fileInfo.size;
        const startTime = Date.now();

        // Track this as an active WebRTC upload (app is SENDING the file)
        activeWebRtcDownloads.add(fileInfo.hash);

        // Add to active uploads map for unified progress display
        // FIX: this is an UPLOAD (app sends file), not a download
        activeUploads.set(uploadKey, {
            hash: fileInfo.hash,
            filename: fileInfo.filename,
            bytes_processed: 0,
            total_bytes: totalSize,
            progress: 0,
            speed_mbps: 0,
            peer_id: conn.peer,
            cancelled: false
        });
        renderUploadProgressList();

        // Detect whether this connection goes through the TURN relay (updates the LED when ready).
        // FIX: poll ICE stats with retries instead of a single call.
        // A single getStats() right after the connection opens often
        // returns no selected candidate pair yet, so the badge stays
        // hidden for the whole transfer. Poll up to 10 times (1s apart).
        let pollAttempts = 0;
        const pollTurnPath = () => {
            if (pollAttempts >= 10) return;
            pollAttempts++;
            detectConnectionPath(conn).then((path) => {
                if (!path) {
                    setTimeout(pollTurnPath, 1000);
                    return;
                }
                connectionPaths.set(uploadKey, path);
                renderUploadProgressList();
            }).catch(() => {
                if (pollAttempts < 10) setTimeout(pollTurnPath, 1000);
            });
        };
        // Start polling after a short delay to let ICE settle
        setTimeout(pollTurnPath, 500);
        // renderUploadProgressList() is called on every chunk anyway, which
        // re-applies the badge via applyConnBadge.

        while (offset < totalSize) {
            // Check if upload was cancelled
            const uploadEntry = activeUploads.get(uploadKey);
            if (uploadEntry && uploadEntry.cancelled) {
                log(`❌ Upload cancelled: ${fileInfo.filename}`);
                activeUploads.delete(uploadKey);
                renderUploadProgressList();
                activeWebRtcDownloads.delete(fileInfo.hash);
                return;
            }

            const chunk = await invoke<string | null>('read_file_chunk', {
                hash: hash,
                offset: offset
            });

            if (!chunk) break; // EOF

            // Decode base64 to binary and send raw bytes
            const binaryChunk = atob(chunk);
            const bytes = new Uint8Array(binaryChunk.length);
            for (let i = 0; i < binaryChunk.length; i++) {
                bytes[i] = binaryChunk.charCodeAt(i);
            }

            // Backpressure: wait if buffer is too full
            if (conn.dataChannel && conn.dataChannel.bufferedAmount > 1024 * 1024) { // 1MB threshold
                await new Promise<void>((resolve) => {
                    const onLow = () => {
                        conn.dataChannel.removeEventListener('bufferedamountlow', onLow);
                        resolve();
                    };
                    conn.dataChannel.addEventListener('bufferedamountlow', onLow);
                });
            }

            conn.send(bytes);

            // Increment by actual bytes read (not chunkSize)
            offset += bytes.length;

            // Update upload progress bar in real-time
            const elapsedMs = Date.now() - startTime;
            const speedMbps = elapsedMs > 0 ? (offset / (1024 * 1024)) / (elapsedMs / 1000) : 0;
            const progress = Math.min(100, (offset / totalSize) * 100);
            // Update the active uploads map (this is an upload operation)
            // Use the same unique key for this upload
            // FIX: preserve the existing `cancelled` flag instead of hardcoding
            // `false`. Without this, every loop iteration overwrites the flag
            // set by the user's cancel click, so the upload never stops.
            const existingEntry = activeUploads.get(uploadKey);
            activeUploads.set(uploadKey, {
                hash: fileInfo.hash,
                filename: fileInfo.filename,
                bytes_processed: offset,
                total_bytes: totalSize,
                progress: progress,
                speed_mbps: speedMbps,
                peer_id: conn.peer,
                cancelled: existingEntry ? existingEntry.cancelled : false
            });
            renderUploadProgressList();
        }

        // Mark upload as complete - remove from active uploads
        activeUploads.delete(uploadKey);
        renderUploadProgressList();
        
        // Remove from active WebRTC uploads
        activeWebRtcDownloads.delete(fileInfo.hash);
        
        log(`✅ File sent to ${conn.peer}`);
        
        // Record as upload (file leaving this computer)
        addRecentTransfer('upload', fileInfo.filename, fileInfo.size);
        
        // Clear the pending file hash
        invoke('clear_pending_file_hash').catch(console.error);
    } catch (error) {
        const errorMsg = getErrorMessage(error);
        log('❌ Error streaming file: ' + errorMsg);
        // Remove from active uploads on error
        activeUploads.delete(uploadKey);
        renderUploadProgressList();
        // Remove from active WebRTC uploads on error
        activeWebRtcDownloads.delete(fileInfo.hash);
        // FIX #2: Notify receiver of the error
        try {
            conn.send(JSON.stringify({ type: 'error', message: errorMsg }));
        } catch (e) {
            log('❌ Could not send error to receiver: ' + getErrorMessage(e));
        }
    }
}

function handleIncomingConnection(conn: DataConnection): void {
    connections.set(conn.peer, conn);
    updateConnectionStatus('connected', 'Connected to ' + conn.peer);
    
    // Initialize message queue for this peer (serialization)
    let messageQueue = Promise.resolve();
    uploadMessageQueues.set(conn.peer, messageQueue);
    
    // Register data listener FIRST to avoid race condition
        conn.on('data', (data: any) => {
            log('📨 Received data from ' + conn.peer + ': ' + (typeof data === 'string' ? data : '[binary]'));
            
            // Serialize all messages for this peer to avoid race condition
            // Update the queue after each message to ensure proper ordering
            const currentQueue = uploadMessageQueues.get(conn.peer) || Promise.resolve();
            const newQueue = currentQueue.then(() => processIncomingMessage(conn, data))
                .catch(console.error);
            uploadMessageQueues.set(conn.peer, newQueue);
        });
    
    // When a connection is established, notify ready but DON'T send file yet
    conn.on('open', () => {
        log('🔗 Connection open with: ' + conn.peer);
        
        // Check for pending file hash (P2P-to-Web flow)
        // Send a 'ready' signal to let receiver know we're listening
        invoke<string | null>('get_pending_file_hash')
            .then((hash) => {
                if (hash) {
                    log('📤 Pending file available, waiting for request from ' + conn.peer);
                    // Send ready signal - receiver will request the file
                    conn.send(JSON.stringify({ type: 'ready', hash: hash }));
                }
            })
            .catch(console.error);
    });
    
    conn.on('close', () => {
            connections.delete(conn.peer);
            incomingUploads.delete(conn.peer);
            uploadMessageQueues.delete(conn.peer);
            // Remove all uploads from this peer from the map
            for (const key of activeUploads.keys()) {
                if (key.startsWith(conn.peer + '-')) {
                    activeUploads.delete(key);
                }
            }
            // Remove the connection path information for this peer
            for (const key of connectionPaths.keys()) {
                if (key.startsWith(conn.peer + '-')) {
                    connectionPaths.delete(key);
                }
            }
            renderUploadProgressList();
            updateConnectionStatus('disconnected', 'Disconnected');
        });
}

// Auto-finalize: called when receivedBytes >= size (all bytes written to disk).
// This is the ULTRA-ROBUST path — it does NOT depend on the out-of-order
// `upload_end` text message from PeerJS, which can arrive before the last
// binary chunks are processed. Prevents hash mismatch from premature finalize.
async function finalizeIncomingUpload(conn: DataConnection, upload: IncomingUpload): Promise<void> {
    // Remove from active downloads
    // FIX P0: use the SAME logic as line 1376 (msg.hash has priority
    // over peer-filename) to ensure set/delete operate on the same
    // entry. Without this, the entry created at line 1376 with key msg.hash
    // remains orphaned and appears as a download bar stuck at 0%.
    const downloadId = upload.expectedHash || `${conn.peer}-${upload.filename}`;
    activeDownloads.delete(downloadId);
    updateDownloadProgress(Array.from(activeDownloads.values()));

    // Call finalize_incoming_file with try/catch
    try {
        log('📌 finalize_incoming_file with peerId: ' + maskPeerId(conn.peer));
        console.log('Full peerId (debug):', conn.peer);
        const result = await invoke<string>('finalize_incoming_file', {
            peerId: conn.peer
        });
        log('✅ File saved: ' + upload.filename + ' (hash: ' + result + ')');
        sendUploadComplete(conn, upload);
        // Refresh file list
        loadFiles();
        // Record as a download (app received the file via inbox)
        addRecentTransfer('download', upload.filename, upload.size);
    } catch (err) {
        // FIX P0: distinguishes hash mismatch from generic errors.
        // If the backend rejects the upload for integrity, the
        // file was NOT saved to shared-folder/. The user
        // must know exactly what happened.
        const errMsg = getErrorMessage(err);
        const isHashMismatch = errMsg.toLowerCase().includes('hash mismatch');
        if (isHashMismatch) {
            log('❌❌ HASH MISMATCH: file rejected by backend for integrity: ' + errMsg);
            // System notification with explicit title
            showNotification(
                '❌ Upload REJECTED: hash mismatch',
                `${upload.filename}: the received file does not match the declared hash. It was NOT saved to ensure integrity.`
            );
            // Messaggio al browser (se P2P-to-Web)
            try {
                conn.send(JSON.stringify({
                    type: 'upload_error',
                    reason: 'hash_mismatch',
                    message: errMsg
                }));
            } catch (e) { /* best-effort */ }
        } else {
            log('❌ Error finalizing file: ' + errMsg);
            try {
                conn.send(JSON.stringify({
                    type: 'upload_error',
                    reason: 'finalize_failed',
                    message: errMsg
                }));
            } catch (e) { /* best-effort */ }
        }
    }
}

// Process incoming message (serialized per-peer)
async function processIncomingMessage(conn: DataConnection, data: any): Promise<void> {
    // Handle P2P-to-Web file request (reverse handshake)
    if (typeof data === 'string') {
        try {
            const msg = JSON.parse(data);
            if (msg.type === 'request_file') {
                // Receiver is requesting the file - start streaming
                log('📤 Receiver requested file: ' + msg.hash);
                await streamFileToConnection(conn, msg.hash);
                return;
            }
            // The receiver cancelled a download in progress. We mark the
            // matching upload as cancelled so the sender loop (which polls
            // uploadEntry.cancelled) stops sending more chunks.
            if (msg.type === 'cancel_upload') {
                log('⛔ Receiver cancelled upload: ' + msg.hash);
                for (const [key, value] of activeUploads.entries()) {
                    if (value.hash === msg.hash) {
                        value.cancelled = true;
                        activeUploads.set(key, value);
                        break;
                    }
                }
                return;
            }
            if (msg.type === 'upload_file') {
                // Browser is sending a file (Reverse Inbox)
                log('📥 Upload request from browser: ' + msg.filename + ' (peerId: ' + maskPeerId(conn.peer) + ')');
                console.log('Full peerId (debug):', conn.peer);

                // TURN size limit (Phase 2): blocks browser>app uploads exceeding TURN_MAX_FILE_SIZE.
                // FIX: show the download bar IMMEDIATELY (before detectConnectionPath)
                // so the user gets instant feedback instead of waiting 1-2s for ICE stats.
                // The badge is updated asynchronously when the path is detected.
                // FIX LOW #11: use the __turnMaxFileSize cache instead of calling IPC.
                const turnMaxSize2 = (window as any).__turnMaxFileSize || 100 * 1024 * 1024;
                // Fallback: if the cache is not ready yet, load it once
                if (!(window as any).__turnMaxFileSize) {
                    try {
                        const limits = await invoke<{ max_file_size: number }>('get_turn_limits');
                        (window as any).__turnMaxFileSize = limits.max_file_size;
                    } catch { /* best-effort */ }
                }
                // FIX: declare downloadId BEFORE the TURN limit check so the
                // red badge + download bar also appear on the RECEIVER's side
                // when the file exceeds the TURN size limit (same fix as the
                // upload path in streamFileToConnection).
                const downloadId = msg.hash || `${conn.peer}-${msg.filename}`;

                // Show the download bar IMMEDIATELY so the user sees feedback
                // while ICE stats are being fetched (1-2s).
                activeDownloads.set(downloadId, {
                    hash: downloadId,
                    filename: msg.filename,
                    total_bytes: msg.size,
                    downloaded_bytes: 0,
                    progress: 0,
                    speed_mbps: 0,
                    peer_ip: conn.peer,
                    cancelled: false
                });
                updateDownloadProgress(Array.from(activeDownloads.values()));

                // FIX HIGH: unify the two polling mechanisms into one that
                // both updates the badge and determines the path for the limit
                // check. Two parallel polls on the same connection could
                // generate a race condition on `connectionPaths`.
                // Poll ICE stats with retries — a single getStats() right after
                // the connection opens often returns no selected candidate pair.
                const downloadKey2 = downloadId;
                let detectedPath2: 'direct' | 'turn' | null = null;
                let pollAttempts2 = 0;
                const pollTurnPath = () => {
                    if (pollAttempts2 >= 10) return;
                    pollAttempts2++;
                    detectConnectionPath(conn).then((path) => {
                        if (!path) {
                            setTimeout(pollTurnPath, 1000);
                            return;
                        }
                        detectedPath2 = path;
                        connectionPaths.set(downloadKey2, path);
                        updateDownloadProgress(Array.from(activeDownloads.values()));
                    }).catch(() => {
                        if (pollAttempts2 < 10) setTimeout(pollTurnPath, 1000);
                    });
                };
                setTimeout(pollTurnPath, 500);

                // Wait for the first path detection result before deciding
                // whether to reject (TURN + >100MB) or accept.
                // Use the pollTurnPath callback above (already running) to
                // set detectedPath2. This replaces the previous duplicate
                // for-loop that called detectConnectionPath a second time
                // on the same connection (race condition).
                // Attesa attiva: max 5s, polling ogni 250ms invece di 500ms.
                // Il polling async (pollTurnPath) continua a girare in background
                // per aggiornare il badge LED; questo loop serve solo a determinare
                // il path per il controllo del limite TURN.
                const deadline2 = Date.now() + 5000;
                while (Date.now() < deadline2 && !detectedPath2) {
                    detectedPath2 = await detectConnectionPath(conn).catch(() => null);
                    if (detectedPath2) break;
                    await new Promise<void>(r => setTimeout(r, 250));
                }
                // Fallback: se ancora null, assumi TURN per sicurezza
                if (!detectedPath2) {
                    log('⚠️ detectConnectionPath (upload): timeout (5s). Assuming TURN for safety.');
                    detectedPath2 = 'turn';
                }
                if (detectedPath2 === 'turn' && msg.size > turnMaxSize2) {
                    const errMsg = 'Your connection requires a relay server. To share this file, connect to WiFi.';
                    log('TURN size limit exceeded (inbox): ' + errMsg);
                    invoke('record_turn_rejection_cmd').catch(() => { /* best-effort */ });

                    // The download bar is already shown above; just set the red badge.
                    connectionPaths.set(downloadId, 'turn');
                    updateDownloadProgress(Array.from(activeDownloads.values()));

                    try {
                        conn.send(JSON.stringify({
                            type: 'upload_error',
                            reason: 'turn_size_limit',
                            message: errMsg,
                            max_size: turnMaxSize2,
                            file_size: msg.size,
                        }));
                    } catch (e) {
                        log('Could not send turn_size_limit error: ' + getErrorMessage(e));
                    }
                    return;
                }

                // Initialize incoming upload state (incremental write to disk)
                const upload: IncomingUpload = {
                    filename: msg.filename,
                    size: msg.size,
                    expectedHash: msg.hash || '',
                    receivedBytes: 0,
                    startTime: Date.now(),
                    finalized: false,
                    uploadCompleteSent: false,
                    cancelled: false
                };
                incomingUploads.set(conn.peer, upload);

                // Call init_incoming_upload to create temp file.
                // FIX CRITICAL #3: pass the detected ICE path (detectedPath2)
                // to the backend, which uses it to apply the TURN limit server-side
                // (defense in depth). Without this, the path is always None and the
                // check in init_incoming_upload.rs:29 never activates.
                try {
                    log('📌 init_incoming_upload with peerId: ' + maskPeerId(conn.peer));
                    console.log('Full peerId (debug):', conn.peer);
                    await invoke('init_incoming_upload', {
                        peerId: conn.peer,
                        filename: msg.filename,
                        size: msg.size,
                        expectedHash: msg.hash || '',
                        path: detectedPath2 === 'turn' ? 'turn' : 'direct'
                    });
                    // Acknowledge and start receiving
                    conn.send(JSON.stringify({ type: 'upload_accepted' }));
                } catch (err) {
                    log('❌ Error initializing upload: ' + getErrorMessage(err));
                    conn.send(JSON.stringify({ type: 'upload_error', message: getErrorMessage(err) }));
                    // Remove from active downloads on error
                    activeDownloads.delete(downloadId);
                    updateDownloadProgress(Array.from(activeDownloads.values()));
                }
                return;
            }
            if (msg.type === 'upload_end') {
                            // Browser finished sending file.
                            // NOTE: This message can arrive OUT OF ORDER (before the last
                            // binary chunks are processed) due to how PeerJS delivers
                            // text vs binary data. We do NOT finalize here — the file is
                            // auto-finalized in the binary chunk handler when
                            // receivedBytes >= size (see finalizeIncomingUpload).
                            // This prevents the classic "hash mismatch" caused by
                            // finalizing an incomplete file.
                            // FIX HIGH: we simplified the logic. Auto-finalization
                            // (line 1653) is the primary path and already sends
                            // `upload_complete`. This handler is a fallback: if
                            // the upload is already finalized, confirm; otherwise
                            // do nothing (do NOT send `upload_complete`) to avoid
                            // a race condition with the arrival of the last chunks.
                            log('📥 upload_end received (auto-finalize handles completion)');
                            const upload = incomingUploads.get(conn.peer);
                            if (upload && upload.finalized) {
                                log('✅ File already finalized by auto-finalize, confirming to browser');
                                sendUploadComplete(conn, upload);
                            } else {
                                log('⏳ upload_end received but auto-finalize not yet done; waiting for last chunk');
                            }
                            return;
                        }
        } catch (e) {
            // Not JSON, ignore
        }
        return;
    }
    
    // Handle binary data (file chunks)
    if (data instanceof ArrayBuffer || data instanceof Uint8Array || ArrayBuffer.isView(data)) {
        let chunk: Uint8Array;
        if (data instanceof ArrayBuffer) {
            chunk = new Uint8Array(data);
        } else if (data instanceof Uint8Array) {
            chunk = data;
        } else {
            chunk = new Uint8Array(data.buffer, data.byteOffset, data.byteLength);
        }

        const upload = incomingUploads.get(conn.peer);
        if (!upload) return;
        if (upload.cancelled) {
            // FIX: chunk residuo arrivato dopo il cancel - ignora
            log('Upload cancelled, ignoring residual chunk');
            return;
        }
        // Call append_incoming_chunk to write incrementally to disk
        try {
            log('append_incoming_chunk with peerId: ' + maskPeerId(conn.peer) + ', chunk size: ' + chunk.length);
            console.log('Full peerId (debug):', conn.peer);
            await invoke('append_incoming_chunk', {
                peerId: conn.peer,
                chunk: chunk
            });
            upload.receivedBytes += chunk.length;
            const progress = Math.round((upload.receivedBytes / upload.size) * 100);
            log('Received upload chunk: ' + progress + '%');

            // ULTRA-ROBUST auto-finalize: when ALL bytes declared in
            // upload_file have been received and written to disk, finalize
            // IMMEDIATELY. This does NOT depend on the out-of-order
            // `upload_end` text message from PeerJS, which can arrive
            // before the last binary chunks are processed.
            // This eliminates the root cause of hash mismatch.
            if (!upload.finalized && upload.receivedBytes >= upload.size) {
                upload.finalized = true;
                log('All bytes received (' + upload.receivedBytes + '/' + upload.size + '), auto-finalizing...');
                // FIX HIGH: do NOT delete incomingUploads BEFORE finalizeIncomingUpload.
                // If finalize fails (e.g. hash mismatch), the entry must remain
                // to allow a retry from the browser. finalizeIncomingUpload
                // already sends `upload_complete` to the browser on success.
                await finalizeIncomingUpload(conn, upload);
                incomingUploads.delete(conn.peer);
                return;
            }

            // Real transfer speed: bytes received / elapsed time since start
            const elapsedMs = Date.now() - upload.startTime;
            const speedMbps = elapsedMs > 0 ? (upload.receivedBytes / (1024 * 1024)) / (elapsedMs / 1000) : 0;
            // Update download progress bar (Reverse Inbox: app is receiving from browser)
            // FIX P0: consistency with line 1376 (msg.hash prioritized) to avoid orphaned entries.
            const downloadId = upload.expectedHash || (conn.peer + '-' + upload.filename);
            activeDownloads.set(downloadId, {
                hash: upload.expectedHash || downloadId,
                filename: upload.filename,
                total_bytes: upload.size,
                downloaded_bytes: upload.receivedBytes,
                progress: progress,
                speed_mbps: speedMbps,
                peer_ip: conn.peer,
                cancelled: false
            });
            updateDownloadProgress(Array.from(activeDownloads.values()));
        } catch (err) {
            const errorMsg = getErrorMessage(err);
            log('Error appending chunk: ' + errorMsg);
            // Notify browser so it doesn't hang sending chunks forever
            try {
                conn.send(JSON.stringify({ type: 'upload_error', message: errorMsg }));
            } catch (e) {
                log('Could not send upload_error to browser: ' + getErrorMessage(e));
            }
            // Remove from active downloads on error
            // FIX P0: consistency with line 1376 (msg.hash prioritized) to avoid orphaned entries.
            const downloadId = upload.expectedHash || (conn.peer + '-' + upload.filename);
            activeDownloads.delete(downloadId);
            updateDownloadProgress(Array.from(activeDownloads.values()));
        }
    }

    // Handle incoming file offers (P2P-to-P2P)
    if (data?.type === 'file-offer') {
        // Initialize incoming file state
        incomingFiles.set(data.hash, {
            filename: data.filename,
            size: data.size,
            hash: data.hash,
            chunks: new Map(),
            receivedBytes: 0,
            startTime: Date.now()
        });
        // Add to active downloads for progress display
        // FIX #1: always use data.hash as the key (it is the msg.expectedHash
        // from the sender, which matches the backend cancellation key)
        activeDownloads.set(data.hash, {
            hash: data.hash,
            filename: data.filename,
            total_bytes: data.size,
            downloaded_bytes: 0,
            progress: 0,
            speed_mbps: 0,
            peer_ip: conn.peer,
            cancelled: false
        });
        updateDownloadProgress(Array.from(activeDownloads.values()));
        // Detect whether this connection goes through the TURN relay (LED badge).
        // FIX: poll ICE stats with retries instead of a single call.
        // A single getStats() right after the connection opens often
        // returns no selected candidate pair yet, so the badge stays
        // hidden for the whole transfer. Poll up to 10 times (1s apart).
        let p2pPollAttempts = 0;
        const p2pPollTurnPath = () => {
            if (p2pPollAttempts >= 10) return;
            p2pPollAttempts++;
            detectConnectionPath(conn).then((path) => {
                if (!path) {
                    setTimeout(p2pPollTurnPath, 1000);
                    return;
                }
                connectionPaths.set(data.hash, path);
                updateDownloadProgress(Array.from(activeDownloads.values()));
            }).catch(() => {
                if (p2pPollAttempts < 10) setTimeout(p2pPollTurnPath, 1000);
            });
        };
        setTimeout(p2pPollTurnPath, 500);
        log('File offer received: ' + data.filename + ' (' + data.size + ' bytes)');
        return;
    }
    if (data?.type === 'file-chunk') {
        const incoming = incomingFiles.get(data.hash);
        if (incoming) {
            incoming.chunks.set(data.offset, data.chunk);
            incoming.receivedBytes += data.chunk.length;
            const progress = Math.round((incoming.receivedBytes / incoming.size) * 100);
            log('Received chunk: ' + progress + '%');

            // Update download progress bar
            const elapsedMs = Date.now() - incoming.startTime;
            const speedMbps = elapsedMs > 0 ? (incoming.receivedBytes / (1024 * 1024)) / (elapsedMs / 1000) : 0;
            activeDownloads.set(data.hash, {
                hash: data.hash,
                filename: incoming.filename,
                total_bytes: incoming.size,
                downloaded_bytes: incoming.receivedBytes,
                progress: progress,
                speed_mbps: speedMbps,
                peer_ip: conn.peer,
                cancelled: false
            });
            updateDownloadProgress(Array.from(activeDownloads.values()));

            // Check if all chunks received
            if (incoming.chunks.size > 0 && incoming.receivedBytes >= incoming.size) {
                log('All chunks received, assembling file...');
                // Assemble file from chunks
                const assembled = new Uint8Array(incoming.size);
                let pos = 0;
                for (const [offset, chunk] of incoming.chunks) {
                    assembled.set(chunk, offset);
                    pos += chunk.length;
                }
                // Create blob and download
                const blob = new Blob([assembled]);
                const url = URL.createObjectURL(blob);
                const a = document.createElement('a');
                a.href = url;
                a.download = incoming.filename;
                document.body.appendChild(a);
                a.click();
                document.body.removeChild(a);
                setTimeout(() => URL.revokeObjectURL(url), 5000);
                log('File downloaded via P2P');
                incomingFiles.delete(data.hash);
                activeDownloads.delete(data.hash);
                updateDownloadProgress(Array.from(activeDownloads.values()));
            }
        }
        return;
    }
}

function downloadReceivedFile(filename: string, data: Uint8Array): void {
    const blob = new Blob([data.buffer as ArrayBuffer]);
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = filename;
    a.click();
    URL.revokeObjectURL(url);
    showNotification('File received', `${filename} has been received successfully`);
    addRecentTransfer('download', filename, data.byteLength);
}

async function connectToPeer(): Promise<void> {
    // Manual connection UI removed: this function remains only for compatibility
    if (!remotePeerIdInput || !connectPeerBtn) return;
    const remotePeerId = remotePeerIdInput.value.trim();
    if (!remotePeerId || !peer) return;
    connectPeerBtn.disabled = true;
    connectPeerBtn.textContent = '⏳ Connecting...';
    updateConnectionStatus('pending', 'Waiting...');
    try {
        const conn = peer.connect(remotePeerId, { reliable: true });
        conn.on('open', () => {
            connections.set(remotePeerId, conn);
            updateConnectionStatus('connected', 'Connected to ' + remotePeerId);
            connectPeerBtn.textContent = '✅ Connected';
        });
        conn.on('error', (err) => {
            log('❌ Connection error: ' + err);
            updateConnectionStatus('error', 'Connection error');
            connectPeerBtn.textContent = 'Connect';
        });
        conn.on('close', () => {
            connections.delete(remotePeerId);
            updateConnectionStatus('disconnected', 'Disconnected');
            connectPeerBtn.textContent = 'Connect';
        });
    } catch (error) {
        log('❌ Failed to connect: ' + getErrorMessage(error));
        updateConnectionStatus('error', 'Error');
        connectPeerBtn.textContent = 'Connect';
    }
    connectPeerBtn.disabled = false;
}

async function disconnectFromPeer(): Promise<void> {
    // Manual connection UI removed: this function remains only for compatibility
    if (!remotePeerIdInput || !connectPeerBtn) return;
    // The Rust command requires a peer_id; we use the remote ID entered
    // or the first connected peer on the frontend side (PeerJS).
    const targetPeerId = remotePeerIdInput.value.trim() ||
        (connections.size > 0 ? Array.from(connections.keys())[0] : '');
    try {
        await invoke('disconnect_from_peer', { peerId: targetPeerId });
    } catch (error) {
        log('Disconnect error: ' + getErrorMessage(error));
    }
    connections.forEach(conn => conn.close());
    connections.clear();
    updateConnectionStatus('disconnected', 'Disconnected');
    connectPeerBtn.textContent = 'Connect';
}

function updateConnectionStatus(status: 'connected' | 'disconnected' | 'error' | 'pending', text: string): void {
    if (connectionStatusEl && connectionStatusText) {
        connectionStatusEl.className = `connection-status status-${status}`;
        connectionStatusText.textContent = text;
    }
}

async function loadPeers(): Promise<void> {
    // Active Peers UI removed: the function remains for compatibility but exits immediately
    if (!peersListEl) return;
    try {
        const peers = await invoke<PeerInfo[]>('list_peers');
        if (!peers || peers.length === 0) {
            peersListEl.innerHTML = '<li class="loading">No peers</li>';
            return;
        }
        peersListEl.innerHTML = peers.map(p => `<li>${escapeHtml(p.peer_id)}</li>`).join('');
    } catch {
        peersListEl.innerHTML = '<li class="loading">No peers</li>';
    }
}

// Generate a P2P-to-Web link that includes the sender's PeerID
async function generateWebLink(): Promise<void> {
    const selectedHash = getSelectedHash();
    if (!selectedHash) {
        showHeaderStatus('❌ Select a file before generating web link', 'error');
        return;
    }
    generateWebLinkBtn.disabled = true;
    generateWebLinkBtn.textContent = '⏳ Generating...';
    try {
        // Local server active => the link includes the LAN fallback (lan=...)
        await ensureServerRunning();
        // ICE/signaling configuration is entirely env-driven in the backend:
        // no static credentials passed from the frontend.
        const link = await invoke<string>('generate_web_link', {
            hash: selectedHash,
        });
        if (webLinkDisplay) webLinkDisplay.textContent = link;
        if (webLinkContainer) webLinkContainer.classList.remove('hidden');
        showHeaderStatus('✅ Web link generated!', 'success');
        // Mark file as having its link generated (shows checkmark in file list)
        copiedLinkFiles.set(selectedHash, Date.now());
        renderFiles();
        // Reset checkmark after 10 seconds
        setTimeout(() => {
            copiedLinkFiles.delete(selectedHash);
            renderFiles();
        }, 10000);
    } catch (error) {
        showHeaderStatus(`❌ ${getErrorMessage(error)}`, 'error');
    }
    generateWebLinkBtn.disabled = false;
    generateWebLinkBtn.textContent = '🔗 Generate Link';
}

function getSelectedHash(): string | null {
    const selected = document.querySelector('.file-item.selected');
    if (selected) return (selected as HTMLElement).dataset.hash || null;
    return null;
}

function showFileReceiveNotification(filename: string, peerId: string): void {
    if (confirm(`📥 Receive file from ${peerId}: ${filename}?`)) {
        log('✅ File accepted: ' + filename);
    } else {
        log('❌ File rejected: ' + filename);
    }
}

function log(message: string): void {
    console.log('[P2P] ' + message);
}

async function copyToClipboard(text: string, successMsg: string): Promise<void> {
    try {
        await writeText(text);
        showHeaderStatus(`✅ ${successMsg}`, 'success');
    } catch (error) {
        showHeaderStatus(`❌ ${getErrorMessage(error)}`, 'error');
    }
}

export function configureP2p(config: P2pConfig): void {
    p2pConfig = { ...p2pConfig, ...config };
    if (peer) peer.destroy();
    initPeer();
}

// ---------- Reverse Inbox ----------
async function handleCreateInboxLocal(): Promise<void> {
    createInboxLocalBtn.disabled = true;
    createInboxLocalBtn.textContent = '⏳ Generating...';
    try {
        // Local inbox = pure HTTP (/inbox/{id}): no signaling/TURN involved
        const link = await invoke<string>('create_inbox_local');
        if (inboxLocalLinkEl) inboxLocalLinkEl.textContent = link;
        if (inboxLocalLinkContainer) inboxLocalLinkContainer.classList.remove('hidden');
        showHeaderStatus('✅ Local inbox link generated!', 'success');
    } catch (error) {
        showHeaderStatus(`❌ ${getErrorMessage(error)}`, 'error');
    }
    createInboxLocalBtn.disabled = false;
    createInboxLocalBtn.textContent = '📥 Create Local Inbox';
}

async function handleCreateInboxInternet(): Promise<void> {
    createInboxInternetBtn.disabled = true;
    createInboxInternetBtn.textContent = '⏳ Generating...';
    try {
        // Local server active => the link includes the LAN fallback (lan=...)
        await ensureServerRunning();
        const link = await invoke<string>('create_inbox');
        if (inboxInternetLinkEl) inboxInternetLinkEl.textContent = link;
        if (inboxInternetLinkContainer) inboxInternetLinkContainer.classList.remove('hidden');
        showHeaderStatus('✅ Internet inbox link generated!', 'success');
    } catch (error) {
        showHeaderStatus(`❌ ${getErrorMessage(error)}`, 'error');
    }
    createInboxInternetBtn.disabled = false;
    createInboxInternetBtn.textContent = '📥 Create Inbox';
}

// ---------- Event listeners ----------
uploadBtn.addEventListener('click', handleSelectFile);
refreshBtn.addEventListener('click', handleRefresh);
// Start Server button removed from UI: auto-start via ensureServerRunning()
startServerBtn?.addEventListener('click', handleStartServer);
openFolderBtn.addEventListener('click', handleOpenFolder);

dropZone.addEventListener('dragenter', handleDragEnter);
dropZone.addEventListener('dragover', handleDragOver);
dropZone.addEventListener('dragleave', handleDragLeave);
dropZone.addEventListener('drop', handleDrop);

// Toggle Local / Internet
document.querySelectorAll('.toggle-btn').forEach(btn => {
    btn.addEventListener('click', () => {
        document.querySelectorAll('.toggle-btn').forEach(b => b.classList.remove('active'));
        btn.classList.add('active');
        const ctx = (btn as HTMLElement).dataset.context as 'local' | 'internet';
        updateContext(ctx);
    });
});

// Columns / Sorting
columnSelect.addEventListener('change', renderFiles);
sortSelect.addEventListener('change', renderFiles);

// Generate Local Link
generateLocalLinkBtn?.addEventListener('click', handleGenerateLocalLink);
copyLocalLinkBtn?.addEventListener('click', () => {
    if (localLinkDisplay) copyToClipboard(localLinkDisplay.textContent || '', 'Local link copied');
});

// Local Inbox
createInboxLocalBtn?.addEventListener('click', handleCreateInboxLocal);
copyInboxLocalLinkBtn?.addEventListener('click', () => {
    if (inboxLocalLinkEl) copyToClipboard(inboxLocalLinkEl.textContent || '', 'Local inbox link copied');
});

// Internet Inbox
createInboxInternetBtn?.addEventListener('click', handleCreateInboxInternet);
copyInboxInternetLinkBtn?.addEventListener('click', () => {
    if (inboxInternetLinkEl) copyToClipboard(inboxInternetLinkEl.textContent || '', 'Internet inbox link copied');
});

copyPeerIdBtn?.addEventListener('click', () => {
    if (currentPeerId) copyToClipboard(currentPeerId, 'ID copied');
});

// P2P (manual connection UI removed: PeerJS engine runs headless)
connectPeerBtn?.addEventListener('click', connectToPeer);
disconnectPeerBtn?.addEventListener('click', disconnectFromPeer);
generateWebLinkBtn.addEventListener('click', generateWebLink);
copyWebLinkBtn.addEventListener('click', () => {
    if (webLinkDisplay) copyToClipboard(webLinkDisplay.textContent || '', 'Web link copied');
});

// ---------- Init ----------
// Listen for download-progress events from HTTP server (for "Generate Local Link" and "Inbox" downloads)
listen('download-progress', (event) => {
    const progress: DownloadProgress = event.payload as DownloadProgress;
    // Only update if this is not an active WebRTC download (prevents duplicate bars)
    // WebRTC downloads use activeDownloads directly in processIncomingMessage
    // But inbox downloads (peer_ip === 'inbox') should be shown
    if (!activeWebRtcDownloads.has(progress.hash) || progress.peer_ip === 'inbox') {
        // Update the active downloads map
        if (progress.progress >= 100) {
            // Download completed, remove it
            activeDownloads.delete(progress.hash);
            // Record as a download ONCE (guard against repeated 100% events
            // from the backend, which would otherwise create duplicate rows)
            if (!recordedDownloadHashes.has(progress.hash)) {
                recordedDownloadHashes.add(progress.hash);
                addRecentTransfer('download', progress.filename, progress.total_bytes);
            }
            // File received via HTTP inbox: backend saved it to
            // shared-folder -> refresh the list to show it immediately.
            if (progress.peer_ip === 'inbox') loadFiles();
        } else {
            // Add or update the download in the map
            activeDownloads.set(progress.hash, progress);
            // Local server HTTP transfer: never TURN -> LED always green
            connectionPaths.set(progress.hash, 'direct');
        }
        // Update UI with ALL active downloads
        updateDownloadProgress(Array.from(activeDownloads.values()));
    }
}).catch(console.error);

// Listen for upload-progress events from HTTP server (for "Generate Local Link" uploads)
listen('upload-progress', (event) => {
    const progress: UploadProgress = event.payload as UploadProgress;
    // Use unique key: peer_id + '-' + hash (for HTTP, peer_id is like "http-192.168.1.100")
    const key = (progress.peer_id || 'http') + '-' + progress.hash;
    // Update the active uploads map
    if (progress.hash) {
        if (progress.progress >= 100) {
            // Upload completed, remove it
            activeUploads.delete(key);
            // Record as an upload ONCE (guard against repeated 100% events)
            if (!recordedUploadHashes.has(key)) {
                recordedUploadHashes.add(key);
                addRecentTransfer('upload', progress.filename, progress.total_bytes);
            }
        } else {
            // Add or update the upload in the map
            activeUploads.set(key, progress);
            // Local server HTTP transfer: never TURN -> LED always green
            connectionPaths.set(key, 'direct');
        }
    }
    // Update UI with ALL active uploads
    renderUploadProgressList();
}).catch(console.error);

    // Polling progress (500ms) - only for HTTP uploads, P2P uploads use activeUploads directly
    const recordedUploadHashes = new Set<string>();
    const recordedDownloadHashes = new Set<string>();
    setInterval(async () => {
        // FIX: fast-path — skip everything when the local server is not running.
        if (!serverRunning) return;
        let httpUploads: UploadProgress[] = [];
        try {
            httpUploads = await invoke<UploadProgress[]>('get_upload_progress');
        } catch (error) {
            console.error('Error polling upload progress:', error);
            return;
        }
        // FIX: fast-path — no active uploads (neither in backend nor in the local
        // activeUploads map driven by P2P events). Avoids log spam and DOM churn
        // when the app is idle.
        if (httpUploads.length === 0 && activeUploads.size === 0) return;
        // Update only HTTP uploads in the map (they have hash, no peer prefix)
        httpUploads.forEach(u => {
            if (u.hash) {
                // Use unique key: peer_id + '-' + hash
                const key = (u.peer_id || 'http') + '-' + u.hash;
                activeUploads.set(key, u);
                // Local server HTTP transfer: never TURN -> LED always green
                connectionPaths.set(key, 'direct');
                // Record completed uploads ONCE (avoid duplicates from polling)
                if (u.progress === 100 && !recordedUploadHashes.has(key)) {
                    recordedUploadHashes.add(key);
                    addRecentTransfer('upload', u.filename, u.total_bytes);
                }
            }
        });
        // Remove completed uploads from the active display.
        // NOTE: we intentionally KEEP the hash in recordedUploadHashes so the
        // same completed upload is never recorded twice (the backend may keep
        // reporting it at 100% on subsequent polls).
        for (const [key, u] of activeUploads) {
            if (u.progress === 100) {
                activeUploads.delete(key);
            }
        }
        // Update UI
        renderUploadProgressList();
    }, 500);

document.addEventListener('DOMContentLoaded', async () => {
    // Initialize empty upload list (container hidden)
    renderUploadProgressList();
    resetDownloadProgress();
    renderRecentTransfers();
    loadFiles();
    await loadNetworkInfo();
    // Auto-start HTTP server only if explicitly enabled by user
    // (avoids opening a port at startup that wasn't requested).
    if (localStorage.getItem('autoStartServer') === 'true') {
        await handleStartServer();
    }
    await initPeer();
    // Cache TURN limit (Phase 2): blocks files > 100MB on TURN.
    invoke<{ max_file_size: number; max_file_size_buffer: number; rejections_total: number }>('get_turn_limits').then((limits) => { (window as any).__turnMaxFileSize = limits.max_file_size; }).catch(() => { (window as any).__turnMaxFileSize = 100 * 1024 * 1024; });
    // Listen for system resume from hibernation/suspension (emitted by Rust backend)
    listen('system-resumed', () => {
        console.log('🔄 System resumed. Restoring connections...');
        handleSystemResume();
    }).catch(console.error);
    await loadPeers();
    updateContext(currentContext);

    // ===== Help button (flashing with tooltip) =====
    const helpBtn = document.querySelector('.help-btn') as HTMLElement | null;
    if (helpBtn) {
        // Stop the pulse animation on first interaction (hover or click)
        helpBtn.addEventListener('mouseenter', () => {
            helpBtn.classList.add('interacted');
        });

        // Click: toggle tooltip (critical for mobile, where there is no hover)
        helpBtn.addEventListener('click', (e: MouseEvent) => {
            e.stopPropagation();
            helpBtn.classList.toggle('show-tooltip');
        });

        // Close the tooltip if clicked outside the button
        document.addEventListener('click', () => {
            helpBtn.classList.remove('show-tooltip');
        });

        // Automatic tooltip on first start: shows instructions for 5 seconds
        if (!localStorage.getItem('helpSeen')) {
            setTimeout(() => {
                helpBtn.classList.add('show-tooltip');
                setTimeout(() => {
                    helpBtn.classList.remove('show-tooltip');
                    localStorage.setItem('helpSeen', 'true');
                }, 5000);
            }, 2000);
        }
    }

    // Event delegation for cancel buttons (download and upload)
    document.addEventListener('click', async (event) => {
        const target = event.target as HTMLElement;
        const cancelBtn = target.closest('.cancel-btn');
        if (!cancelBtn) return;
        
        const downloadKey = cancelBtn.getAttribute('data-download-key');
        const uploadKey = cancelBtn.getAttribute('data-upload-key');
        const key = downloadKey || uploadKey;
        
        if (!key) return;
        
        // Determine if this is a download or upload
        const isDownload = !!downloadKey;
        
        try {
            if (isDownload) {
                // FIX #4: call ONLY cancel_download (no more cancel_upload).
                // cancel_download is sufficient for:
                // 1) local download_file (download_tracker)
                // 2) P2P stream_file (download_tracker)
                // 3) inbox_upload_handler (download_tracker)
                // 4) ProgressTrackingStream (download_tracker)
                // The double call caused a race condition on the active mutex.
                await invoke('cancel_download', { hash: key });
                console.log(`❌ Download cancelled: ${key}`);
                // Mark as cancelled in frontend map (for P2P download loops)
                const existing = activeDownloads.get(key);
                if (existing) {
                    existing.cancelled = true;
                    activeDownloads.set(key, existing);
                    // FIX #2: notify the sender (via WebRTC) to stop sending chunks.
                    // Without this, the sender keeps reading the file and sending
                    // chunks on the data channel until the loop ends — wasting
                    // bandwidth and CPU.
                    // FIX: use the already-tracked `connections` map (Map<peerId, DataConnection>)
                    // instead of accessing `peer.connections[senderPeer][0]` which is an
                    // undocumented internal PeerJS structure and may return a
                    // closed connection even when other active ones exist.
                    const senderPeer = existing.peer_ip;
                    if (senderPeer && senderPeer !== 'inbox') {
                        // Try the typed `connections` map first
                        let conn = connections.get(senderPeer);
                        // Fallback: search in peer.connections (internal PeerJS structure)
                        if (!conn && peer && peer.open) {
                            const conns: any[] = (peer as any).connections?.[senderPeer] || [];
                            conn = conns.find((c: any) => c && c.open) || conns[0];
                        }
                        if (conn && (conn as any).open) {
                            try {
                                (conn as DataConnection).send(JSON.stringify({ type: 'cancel_upload', hash: key }));
                            } catch (e) { /* best-effort */ }
                        }
                    }
                }
            } else {
                // Look up the upload to get the actual hash for the backend
                const existing = activeUploads.get(key);
                const uploadHash = existing ? existing.hash : key;
                await invoke('cancel_upload', { hash: uploadHash });
                console.log(`❌ Upload cancelled: ${key}`);
                if (existing) {
                    existing.cancelled = true;
                    activeUploads.set(key, existing);

                    // FIX: notify the browser (sender) to stop sending chunks.
                    // The browser's sendFileChunks loop has no cancellation
                    // mechanism, so we must send a WebRTC message to tell it
                    // to abort. Without this, the browser keeps sending until
                    // the connection closes or the file is fully uploaded.
                    const browserPeer = existing.peer_id;
                    if (browserPeer && browserPeer !== 'inbox') {
                        let conn = connections.get(browserPeer);
                        if (!conn && peer && peer.open) {
                            const conns: any[] = (peer as any).connections?.[browserPeer] || [];
                            conn = conns.find((c: any) => c && c.open) || conns[0];
                        }
                        if (conn && (conn as any).open) {
                            try {
                                (conn as DataConnection).send(JSON.stringify({
                                    type: 'transfer_cancelled',
                                    hash: key
                                }));
                                log('Sent transfer_cancelled to browser: ' + browserPeer);
                            } catch (e) { /* best-effort */ }
                        }
                    }

                    // FIX: mark incomingUploads as cancelled so residual chunks
                    // in flight are ignored by the binary chunk handler.
                    const peerKey2a: string = browserPeer || '';
                    const incUpload = incomingUploads.get(peerKey2a);
                    if (incUpload) {
                        incUpload.cancelled = true;
                    }

                    // FIX: schedule cleanup of incomingUploads entry after a delay,
                    // giving time for residual chunks in flight to arrive and be
                    // ignored by the cancelled flag check in the chunk handler.
                    const peerKey2c: string = browserPeer || '';
                    if (peerKey2c) {
                        setTimeout(() => {
                            incomingUploads.delete(peerKey2c);
                        }, 1000);
                    }
                }
            }
            
            // Remove from active maps
            // FIX #6: "cancelling in progress" animation before removal,
            // to give visual feedback to the user and prevent multiple clicks.
            if (isDownload) {
                const row = downloadProgressListLeft?.querySelector(
                    `[data-download-id="${CSS.escape(key)}"]`
                );
                const cancelBtn = row?.querySelector<HTMLButtonElement>('.cancel-btn');
                // Disable the button immediately to prevent double clicks
                if (cancelBtn) {
                    cancelBtn.disabled = true;
                    cancelBtn.textContent = '⏳';
                }
                if (row) {
                    row.classList.add('cancelling');
                    setTimeout(() => {
                        activeDownloads.delete(key);
                        updateDownloadProgress(Array.from(activeDownloads.values()));
                    }, 250);
                } else {
                    activeDownloads.delete(key);
                    updateDownloadProgress(Array.from(activeDownloads.values()));
                }
                // Visual feedback in the header
                showHeaderStatus('Download cancelled', 'info');
            } else {
                const row = uploadProgressListRight?.querySelector(
                    `[data-upload-id="${CSS.escape(key)}"]`
                );
                const cancelBtn = row?.querySelector<HTMLButtonElement>('.cancel-btn');
                if (cancelBtn) {
                    cancelBtn.disabled = true;
                    cancelBtn.textContent = '⏳';
                }
                if (row) {
                    row.classList.add('cancelling');
                    setTimeout(() => {
                        activeUploads.delete(key);
                        renderUploadProgressList();
                    }, 250);
                } else {
                    activeUploads.delete(key);
                    renderUploadProgressList();
                }
                showHeaderStatus('Upload cancelled', 'info');
            }
        } catch (error) {
            console.error('Cancel error:', error);
        }
    });
});

 
