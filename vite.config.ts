import { defineConfig } from 'vite';
import { resolve } from 'node:path';

const host = process.env.TAURI_DEV_HOST;

export default defineConfig({
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host ? { protocol: 'ws', host, port: 1421 } : undefined,
    watch: { ignored: ['**/src-tauri/**'] },
  },
  envPrefix: ['VITE_', 'TAURI_ENV_*'],
  build: {
    target: 'es2022',
    sourcemap: false,
    rollupOptions: {
      input: {
        dock: resolve(import.meta.dirname, 'index.html'),
        settings: resolve(import.meta.dirname, 'settings.html'),
      },
    },
  },
});
