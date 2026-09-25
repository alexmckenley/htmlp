import vinext from 'vinext';
import { defineConfig } from 'vite';
export default defineConfig({ plugins: [vinext()], base: process.env.HTMLP_BASE_PATH || '/' });
