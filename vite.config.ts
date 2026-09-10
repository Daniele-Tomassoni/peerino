import { defineConfig } from 'vite';

export default defineConfig({
    root: 'src/ui',
    envDir: '.',
    server: {
        port: 5173,
        strictPort: true,
    },
    build: {
        outDir: '../../dist',
        emptyOutDir: true,
    },
});