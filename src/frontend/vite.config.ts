import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

export default defineConfig({
  plugins: [svelte()],
  server: {
    port: 1420,
    strictPort: true,
    host: 'localhost',
  },
  build: {
    target: 'esnext',
    minify: 'esbuild',
    outDir: '../src-tauri/src/frontend/dist',
  },
});