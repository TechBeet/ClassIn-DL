import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

const config = {
  extensions: ['.svelte'],
  preprocess: vitePreprocess(),
  compilerOptions: {
    dev: false,
  },
};

export default config;