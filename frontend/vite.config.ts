import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
	plugins: [sveltekit()],
	server: {
		allowedHosts: ['ly-perrier'],
		proxy: {
			'/api': {
				target: 'http://127.0.0.1:3001',
				changeOrigin: true,
				ws: true
			}
		}
	}
});
