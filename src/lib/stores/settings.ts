import type { Model } from '$lib/types/models';

// Lazy-load Tauri plugins to avoid errors when running outside Tauri.
// Cache the store instance so repeated calls don't create duplicates.
let _store: any = null;

async function getStore() {
	if (!_store) {
		const { LazyStore } = await import('@tauri-apps/plugin-store');
		_store = new LazyStore('settings.json');
	}
	return _store;
}

// Keyring access via Tauri IPC commands (no JS bindings package exists)
async function getInvoke() {
	const { invoke } = await import('@tauri-apps/api/core');
	return invoke;
}

function createSettingsStore() {
	let apiKey = $state('');
	let darkModeOverride = $state(false);
	let downloadedModels = $state<Model[]>([]);
	let activeModelId = $state<string | null>(null);
	let downloadProgress = $state<{ modelId: string; percent: number } | null>(null);

	async function loadAll() {
		try {
			const store = await getStore();
			darkModeOverride = (((await store.get('darkModeOverride')) as boolean | null) ?? false);
			downloadedModels = (((await store.get('downloadedModels')) as Model[] | null) ?? []);
			activeModelId = (((await store.get('activeModelId')) as string | null) ?? null);
		} catch (e) {
			console.warn('Failed to load settings from store:', e);
		}

		try {
			const invoke = await getInvoke();
			const key = await invoke<string | null>('plugin:keyring|get_password', {
				service: 'com.jolly.desktop',
				user: 'openrouter_api_key'
			});
			apiKey = key ?? '';
		} catch (e) {
			// No key stored yet, or keyring unavailable
			console.warn('Failed to load API key from keyring:', e);
		}
	}

	async function saveApiKey(key: string) {
		apiKey = key;
		try {
			const invoke = await getInvoke();
			if (key) {
				await invoke('plugin:keyring|set_password', {
					service: 'com.jolly.desktop',
					user: 'openrouter_api_key',
					password: key
				});
			} else {
				await invoke('plugin:keyring|delete_password', {
					service: 'com.jolly.desktop',
					user: 'openrouter_api_key'
				});
			}
		} catch (e) {
			console.error('Failed to save API key to keyring:', e);
		}
	}

	async function setDarkModeOverride(value: boolean) {
		darkModeOverride = value;
		try {
			const store = await getStore();
			await store.set('darkModeOverride', value);
			await store.save();
		} catch (e) {
			console.error('Failed to save dark mode preference:', e);
		}
	}

	async function addDownloadedModel(model: Model) {
		downloadedModels = [...downloadedModels, model];
		try {
			const store = await getStore();
			await store.set('downloadedModels', downloadedModels);
			await store.save();
		} catch (e) {
			console.error('Failed to save downloaded models:', e);
		}
	}

	async function setActiveModel(modelId: string | null) {
		activeModelId = modelId;
		try {
			const store = await getStore();
			await store.set('activeModelId', modelId);
			await store.save();
		} catch (e) {
			console.error('Failed to save active model:', e);
		}
	}

	function setDownloadProgress(progress: { modelId: string; percent: number } | null) {
		downloadProgress = progress;
	}

	// OS color scheme tracking (centralized here so layout + Settings don't duplicate)
	let systemDark = $state(false);

	function initSystemDarkListener() {
		const mq = window.matchMedia('(prefers-color-scheme: dark)');
		systemDark = mq.matches;
		const handler = (e: MediaQueryListEvent) => { systemDark = e.matches; };
		mq.addEventListener('change', handler);
		return () => mq.removeEventListener('change', handler);
	}

	return {
		get apiKey() { return apiKey; },
		get darkModeOverride() { return darkModeOverride; },
		get downloadedModels() { return downloadedModels; },
		get activeModelId() { return activeModelId; },
		get downloadProgress() { return downloadProgress; },
		get systemDark() { return systemDark; },
		get isDark() { return darkModeOverride || systemDark; },
		loadAll,
		saveApiKey,
		setDarkModeOverride,
		addDownloadedModel,
		setActiveModel,
		setDownloadProgress,
		initSystemDarkListener
	};
}

export const settings = createSettingsStore();
