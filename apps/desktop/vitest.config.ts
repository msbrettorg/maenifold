import { defineConfig } from 'vitest/config';
import vue from '@vitejs/plugin-vue';
import { fileURLToPath } from 'node:url';

const cdpExcludes = process.env.CDP_ELECTRON_URL ? [] : ['**/agents-perf-cdp.test.ts'];

export default defineConfig({
  plugins: [vue()],
  resolve: {
    alias: {
      src: fileURLToPath(new URL('./src', import.meta.url)),
      '@': fileURLToPath(new URL('./src', import.meta.url)),
    },
  },
  test: {
    environment: 'jsdom',
    setupFiles: ['src/test/setup.ts'],
    root: '.',
    exclude: [
      '**/node_modules/**',
      '**/dist/**',
      'evidence/**',
      'tests/e2e/**',
      'tests/visual/**',
      ...cdpExcludes,
    ],
  },
});
