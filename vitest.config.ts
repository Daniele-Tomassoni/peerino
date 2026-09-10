import { defineConfig } from 'vitest/config';

export default defineConfig({
    test: {
        setupFiles: ['./tests/setup.ts'],
        environment: 'happy-dom',
        globals: true,
    },
});