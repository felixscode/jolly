import adapterAuto from '@sveltejs/adapter-auto';
import adapterStatic from '@sveltejs/adapter-static';

const isTauri = !!process.env.TAURI_ENV_PLATFORM;

/** @type {import('@sveltejs/kit').Config} */
const config = {
	kit: {
		adapter: isTauri
			? adapterStatic({ fallback: 'index.html' })
			: adapterAuto()
	}
};

export default config;
