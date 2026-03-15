import tailwindcss from '@tailwindcss/vite';
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';
import svgr from 'vite-plugin-svgr';
import mkcert from 'vite-plugin-mkcert';

export default defineConfig({
	plugins: [tailwindcss(), sveltekit(), svgr(), mkcert()],
	server: {
		watch: {
			ignored: ['**/src-tauri/target/**']
		}
	}
});
