import { defineConfig } from 'vitest/config';

export default defineConfig({
    test: {
        root: '.',
        setupFiles: ['./tests/setup.ts'],
        environment: 'happy-dom',
        globals: true,
        include: ['tests/**/*.test.ts', 'src/tests/**/*.test.ts'],
    },
});