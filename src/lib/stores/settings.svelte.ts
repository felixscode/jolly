import type { ModelWithState, CustomModel } from '$lib/types/models';

// Lazy-load Tauri plugins to avoid errors when running outside Tauri.
let _store: any = null;

async function getStore() {
	if (!_store) {
		const { LazyStore } = await import('@tauri-apps/plugin-store');
		_store = new LazyStore('settings.json');
	}
	return _store;
}

async function getInvoke() {
	const { invoke } = await import('@tauri-apps/api/core');
	return invoke;
}

function createSettingsStore() {
	let apiKey = $state('');
	let themeMode = $state<'system' | 'light' | 'dark'>('system');
	let activeModelId = $state<string | null>(null);
	let useOpenRouter = $state(false);
	let useHarper = $state(false);
	let correctionHistory = $state<string[]>([]);

	// Models — populated from backend
	let availableModels = $state<ModelWithState[]>([]);
	let downloadedModelIds = $state<string[]>([]);

	// Download progress — driven by Tauri events
	let downloadProgress = $state<{
		modelId: string;
		bytesReceived: number;
		totalBytes: number;
	} | null>(null);
	let downloadError = $state<string | null>(null);

	// Custom imported models — persisted in store
	let customModels = $state<CustomModel[]>([]);

	// Event listener cleanup functions
	let unlisteners: (() => void)[] = [];

	async function loadAll() {
		// Load preferences from store
		try {
			const store = await getStore();
			themeMode =
				((await store.get('themeMode')) as 'system' | 'light' | 'dark' | null) ?? 'system';
			activeModelId = ((await store.get('activeModelId')) as string | null) ?? null;
			useOpenRouter = ((await store.get('useOpenRouter')) as boolean | null) ?? false;
			useHarper = ((await store.get('useHarper')) as boolean | null) ?? false;
			correctionHistory = ((await store.get('correctionHistory')) as string[] | null) ?? [];
			customModels = ((await store.get('customModels')) as CustomModel[] | null) ?? [];
		} catch (e) {
			console.warn('Failed to load settings from store:', e);
		}

		// Load API key from keyring
		try {
			const invoke = await getInvoke();
			const key = await invoke<string | null>('plugin:keyring|get_password', {
				service: 'com.jolly.desktop',
				user: 'openrouter_api_key'
			});
			apiKey = key ?? '';
		} catch (e) {
			console.warn('Failed to load API key from keyring:', e);
		}

		// Load models from backend
		await refreshModels();

		// Validate custom model paths (prune stale references)
		await validateCustomModels();

		// Subscribe to download events (once)
		if (unlisteners.length === 0) {
			await subscribeToEvents();
		}
	}

	async function refreshModels() {
		try {
			const invoke = await getInvoke();
			availableModels = await invoke<ModelWithState[]>('list_available_models');
			downloadedModelIds = await invoke<string[]>('list_downloaded_models');
		} catch (e) {
			console.warn('Failed to load models from backend:', e);
		}
	}

	async function validateCustomModels() {
		if (customModels.length === 0) return;
		try {
			const invoke = await getInvoke();
			const validPaths = await invoke<string[]>('validate_custom_models', {
				paths: customModels.map((m) => m.path)
			});
			const validSet = new Set(validPaths);
			const staleModels = customModels.filter((m) => !validSet.has(m.path));
			if (staleModels.length > 0) {
				customModels = customModels.filter((m) => validSet.has(m.path));
				// If active model was stale, clear selection
				if (activeModelId && staleModels.some((m) => m.id === activeModelId)) {
					activeModelId = null;
					const store = await getStore();
					await store.set('activeModelId', null);
				}
				const store = await getStore();
				await store.set('customModels', customModels);
				await store.save();
			}
		} catch (e) {
			console.warn('Failed to validate custom models:', e);
		}
	}

	async function importCustomModel() {
		try {
			const invoke = await getInvoke();
			const entry = await invoke<CustomModel | null>('import_custom_model');
			if (!entry) return; // User cancelled

			// Deduplicate by path
			if (customModels.some((m) => m.path === entry.path)) {
				return; // Already imported
			}

			customModels = [...customModels, entry];
			const store = await getStore();
			await store.set('customModels', customModels);
			await store.save();
		} catch (e) {
			console.error('Failed to import custom model:', e);
		}
	}

	async function removeCustomModel(id: string) {
		customModels = customModels.filter((m) => m.id !== id);
		try {
			const store = await getStore();
			await store.set('customModels', customModels);
			// If the removed model was active, clear selection
			if (activeModelId === id) {
				activeModelId = null;
				await store.set('activeModelId', null);
			}
			await store.save();
		} catch (e) {
			console.error('Failed to remove custom model:', e);
		}
	}

	async function subscribeToEvents() {
		const { listen } = await import('@tauri-apps/api/event');

		const u1 = await listen<{ modelId: string; bytesReceived: number; totalBytes: number }>(
			'download-progress',
			(event) => {
				downloadProgress = event.payload;
				downloadError = null;
			}
		);

		const u2 = await listen<{ modelId: string }>('download-complete', async () => {
			downloadProgress = null;
			downloadError = null;
			await refreshModels();
		});

		const u3 = await listen<{ modelId: string }>('download-cancelled', async () => {
			downloadProgress = null;
			await refreshModels();
		});

		const u4 = await listen<{ modelId: string; error: string }>('download-error', (event) => {
			downloadProgress = null;
			downloadError = event.payload.error;
		});

		unlisteners = [u1, u2, u3, u4];
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

	async function setThemeMode(value: 'system' | 'light' | 'dark') {
		themeMode = value;
		try {
			const store = await getStore();
			await store.set('themeMode', value);
			await store.save();
		} catch (e) {
			console.error('Failed to save theme preference:', e);
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

	async function setUseOpenRouter(value: boolean) {
		useOpenRouter = value;
		try {
			const store = await getStore();
			await store.set('useOpenRouter', value);
			// Mutual exclusion: OpenRouter ON → Harper OFF
			if (value && useHarper) {
				useHarper = false;
				await store.set('useHarper', false);
			}
			await store.save();
		} catch (e) {
			console.error('Failed to save OpenRouter preference:', e);
		}
	}

	async function setUseHarper(value: boolean) {
		useHarper = value;
		try {
			const store = await getStore();
			await store.set('useHarper', value);
			// Mutual exclusion: Harper ON → OpenRouter OFF
			if (value && useOpenRouter) {
				useOpenRouter = false;
				await store.set('useOpenRouter', false);
			}
			await store.save();
		} catch (e) {
			console.error('Failed to save Harper preference:', e);
		}
	}

	async function startDownload(modelId: string) {
		downloadError = null;
		try {
			const invoke = await getInvoke();
			await invoke('start_download', { modelId });
		} catch (e) {
			downloadError = String(e);
		}
	}

	async function cancelDownload() {
		try {
			const invoke = await getInvoke();
			await invoke('cancel_download');
		} catch (e) {
			console.error('Failed to cancel download:', e);
		}
	}

	async function deleteModel(modelId: string) {
		try {
			const invoke = await getInvoke();
			await invoke('delete_model', { modelId });
			await refreshModels();
			// If the deleted model was active, clear selection
			if (activeModelId === modelId) {
				await setActiveModel(null);
			}
		} catch (e) {
			console.error('Failed to delete model:', e);
		}
	}

	const MAX_HISTORY = 20;

	async function addToHistory(text: string) {
		if (!text.trim()) return;
		correctionHistory = [text, ...correctionHistory.filter((t) => t !== text)].slice(
			0,
			MAX_HISTORY
		);
		try {
			const store = await getStore();
			await store.set('correctionHistory', correctionHistory);
			await store.save();
		} catch (e) {
			console.error('Failed to save correction history:', e);
		}
	}

	async function clearHistory() {
		correctionHistory = [];
		try {
			const store = await getStore();
			await store.set('correctionHistory', []);
			await store.save();
		} catch (e) {
			console.error('Failed to clear correction history:', e);
		}
	}

	// OS color scheme tracking
	let systemDark = $state(false);

	function initSystemDarkListener() {
		const mq = window.matchMedia('(prefers-color-scheme: dark)');
		systemDark = mq.matches;
		const handler = (e: MediaQueryListEvent) => {
			systemDark = e.matches;
		};
		mq.addEventListener('change', handler);
		return () => mq.removeEventListener('change', handler);
	}

	function cleanup() {
		for (const u of unlisteners) u();
		unlisteners = [];
	}

	return {
		get apiKey() {
			return apiKey;
		},
		get themeMode() {
			return themeMode;
		},
		get activeModelId() {
			return activeModelId;
		},
		get useOpenRouter() {
			return useOpenRouter;
		},
		get useHarper() {
			return useHarper;
		},
		get availableModels() {
			return availableModels;
		},
		get downloadedModelIds() {
			return downloadedModelIds;
		},
		get downloadProgress() {
			return downloadProgress;
		},
		get downloadError() {
			return downloadError;
		},
		get systemDark() {
			return systemDark;
		},
		get isDark() {
			return themeMode === 'dark' || (themeMode === 'system' && systemDark);
		},
		get correctionHistory() {
			return correctionHistory;
		},
		get customModels() {
			return customModels;
		},
		loadAll,
		addToHistory,
		clearHistory,
		saveApiKey,
		setThemeMode,
		setActiveModel,
		setUseOpenRouter,
		setUseHarper,
		startDownload,
		cancelDownload,
		deleteModel,
		refreshModels,
		importCustomModel,
		removeCustomModel,
		initSystemDarkListener,
		cleanup
	};
}

export const settings = createSettingsStore();
