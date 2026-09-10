// Vitest setup file for mocking Tauri and plugins
// This file is used to mock window.__TAURI__ and plugins for testing

import { vi } from 'vitest';

// Mock window.__TAURI__ for testing
(global as any).window = {
    __TAURI__: {
        invoke: vi.fn(),
        event: vi.fn(),
    },
};

// Mock the Tauri API
vi.mock('@tauri-apps/api/core', () => ({
    invoke: vi.fn(),
}));

// Mock the dialog plugin
vi.mock('@tauri-apps/plugin-dialog', () => ({
    open: vi.fn().mockResolvedValue('/fake/path/file.txt'),
    save: vi.fn().mockResolvedValue('/fake/path/output.txt'),
}));

// Mock the clipboard manager plugin
vi.mock('@tauri-apps/plugin-clipboard-manager', () => ({
    writeText: vi.fn().mockResolvedValue(undefined),
    readText: vi.fn().mockResolvedValue(''),
}));

// Mock the notification plugin
vi.mock('@tauri-apps/plugin-notification', () => ({
    sendNotification: vi.fn(),
}));

// Mock the autostart plugin
vi.mock('@tauri-apps/plugin-autostart', () => ({
    isEnabled: vi.fn().mockResolvedValue(false),
    enable: vi.fn().mockResolvedValue(undefined),
    disable: vi.fn().mockResolvedValue(undefined),
}));

// Mock the event listener
vi.mock('@tauri-apps/api/event', () => ({
    listen: vi.fn().mockImplementation(() => Promise.resolve(() => {})),
}));

// Mock PeerJS
vi.mock('peerjs', () => ({
    Peer: vi.fn().mockImplementation(() => ({
        on: vi.fn(),
        connect: vi.fn().mockImplementation(() => ({
            on: vi.fn(),
        })),
    })),
    DataConnection: vi.fn(),
}));