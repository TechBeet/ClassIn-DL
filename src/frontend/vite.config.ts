import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

export default defineConfig({
  plugins: [svelte()],
  server: {
    port: 8899,
    strictPort: true,
    host: '127.0.0.1',
  },
  build: {
    target: 'esnext',
    minify: 'esbuild',
    outDir: './dist',
  },
});