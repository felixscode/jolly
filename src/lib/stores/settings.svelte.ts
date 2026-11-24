import type {
	ModelWithState,
	CustomModel,
	RemoteProviderPreset,
	RemoteConfig,
	HistoryEntry,
	ShortcutStatus
} from '$lib/types/models';

const KEYRING_SERVICE = 'com.jolly.desktop';
export const DEFAULT_SHORTCUT = 'CommandOrControl+Shift+J';

/** Keyring account per provider; OpenRouter keeps its pre-0.2 name. */
function keyringAccount(providerId: string) {
	return providerId === 'openrouter' ? 'openrouter_api_key' : `api_key_${providerId}`;
}

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
	let themeMode = $state<'system' | 'light' | 'dark'>('system');
	let activeModelId = $state<string | null>(null);
	let useHarper = $state(false);
	let correctionHistory = $state<HistoryEntry[]>([]);

	// Remote (cloud API) providers
	let useRemote = $state(false);
	let remoteProvider = $state('openrouter');
	let remoteConfigs = $state<Record<string, RemoteConfig>>({});
	let remoteProviders = $state<RemoteProviderPreset[]>([]);
	let apiKeys = $state<Record<string, string>>({});

	// Global hotkey
	let globalShortcut = $state<string | null>(DEFAULT_SHORTCUT);
	let shortcutStatus = $state<ShortcutStatus | null>(null);
	let inlineReplace = $state(true);

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
			// `useOpenRouter` is the pre-0.2 name of `useRemote`.
			useRemote =
				((await store.get('useRemote')) as boolean | null) ??
				((await store.get('useOpenRouter')) as boolean | null) ??
				false;
			remoteProvider = ((await store.get('remoteProvider')) as string | null) ?? 'openrouter';
			remoteConfigs =
				((await store.get('remoteConfigs')) as Record<string, RemoteConfig> | null) ?? {};
			useHarper = ((await store.get('useHarper')) as boolean | null) ?? false;
			const rawHistory = ((await store.get('correctionHistory')) as unknown[] | null) ?? [];
			// Older versions stored plain corrected strings.
			correctionHistory = rawHistory.map((h) =>
				typeof h === 'string' ? { corrected: h, at: 0 } : (h as HistoryEntry)
			);
			const savedShortcut = await store.get('globalShortcut');
			globalShortcut =
				savedShortcut === undefined ? DEFAULT_SHORTCUT : (savedShortcut as string | null);
			inlineReplace = ((await store.get('inlineReplace')) as boolean | null) ?? true;
			customModels = ((await store.get('customModels')) as CustomModel[] | null) ?? [];
		} catch (e) {
			console.warn('Failed to load settings from store:', e);
		}

		try {
			const invoke = await getInvoke();
			remoteProviders = await invoke<RemoteProviderPreset[]>('list_remote_providers');
		} catch (e) {
			console.warn('Failed to load remote providers:', e);
		}
		await loadApiKey(remoteProvider);
		await refreshShortcutStatus();

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

		const u5 = await listen<ShortcutStatus>('shortcut-status', (event) => {
			shortcutStatus = event.payload;
		});

		unlisteners = [u1, u2, u3, u4, u5];
	}

	/** Load a provider's API key from the keyring (cached in `apiKeys`). */
	async function loadApiKey(providerId: string) {
		if (providerId in apiKeys) return;
		try {
			const invoke = await getInvoke();
			const key = await invoke<string | null>('plugin:keyring|get_password', {
				service: KEYRING_SERVICE,
				user: keyringAccount(providerId)
			});
			apiKeys = { ...apiKeys, [providerId]: key ?? '' };
		} catch (e) {
			console.warn('Failed to load API key from keyring:', e);
		}
	}

	async function saveApiKey(providerId: string, key: string) {
		key = key.trim();
		apiKeys = { ...apiKeys, [providerId]: key };
		const invoke = await getInvoke();
		const args = { service: KEYRING_SERVICE, user: keyringAccount(providerId) };
		if (key) {
			await invoke('plugin:keyring|set_password', { ...args, password: key });
		} else {
			try {
				await invoke('plugin:keyring|delete_password', args);
			} catch {
				// Nothing stored — fine.
			}
		}
	}

	async function setRemoteProvider(providerId: string) {
		remoteProvider = providerId;
		await loadApiKey(providerId);
		await persist({ remoteProvider: providerId });
	}

	async function setRemoteConfig(providerId: string, config: RemoteConfig) {
		const cleaned: RemoteConfig = {};
		if (config.model?.trim()) cleaned.model = config.model.trim();
		if (config.baseUrl?.trim()) cleaned.baseUrl = config.baseUrl.trim();
		remoteConfigs = { ...remoteConfigs, [providerId]: cleaned };
		await persist({ remoteConfigs: $state.snapshot(remoteConfigs) });
	}

	async function persist(values: Record<string, unknown>) {
		try {
			const store = await getStore();
			for (const [k, v] of Object.entries(values)) await store.set(k, v);
			await store.save();
		} catch (e) {
			console.error('Failed to save settings:', e);
		}
	}

	async function refreshShortcutStatus() {
		try {
			const invoke = await getInvoke();
			shortcutStatus = await invoke<ShortcutStatus>('get_shortcut_status');
		} catch {
			shortcutStatus = null;
		}
	}

	/** Register a new global hotkey (null disables it). Throws if registration fails. */
	async function setGlobalShortcut(accelerator: string | null) {
		const invoke = await getInvoke();
		shortcutStatus = await invoke<ShortcutStatus>('set_global_shortcut', { accelerator });
		globalShortcut = accelerator;
	}

	/** Open the desktop's own shortcut dialog (Wayland portal backend). */
	async function configureSystemShortcut() {
		const invoke = await getInvoke();
		await invoke('configure_system_shortcut');
	}

	async function setInlineReplace(value: boolean) {
		inlineReplace = value;
		await persist({ inlineReplace: value });
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

	/** Load the model in the backend, then persist the choice. Throws if loading fails. */
	async function setActiveModel(modelId: string | null) {
		if (modelId) {
			const invoke = await getInvoke();
			await invoke('activate_model', { modelId });
		}
		activeModelId = modelId;
		try {
			const store = await getStore();
			await store.set('activeModelId', modelId);
			await store.save();
		} catch (e) {
			console.error('Failed to save active model:', e);
		}
	}

	async function setUseRemote(value: boolean) {
		useRemote = value;
		// Mutual exclusion: remote ON → Harper OFF
		if (value && useHarper) useHarper = false;
		await persist({ useRemote: value, useHarper });
	}

	async function setUseHarper(value: boolean) {
		useHarper = value;
		// Mutual exclusion: Harper ON → remote OFF
		if (value && useRemote) useRemote = false;
		await persist({ useHarper: value, useRemote });
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

	async function addToHistory(corrected: string, original?: string) {
		if (!corrected.trim()) return;
		const entry: HistoryEntry = { corrected, at: Date.now() };
		if (original && original !== corrected) entry.original = original;
		correctionHistory = [
			entry,
			...correctionHistory.filter((h) => h.corrected !== corrected)
		].slice(0, MAX_HISTORY);
		try {
			const store = await getStore();
			await store.set('correctionHistory', $state.snapshot(correctionHistory));
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
		get apiKeys() {
			return apiKeys;
		},
		get useRemote() {
			return useRemote;
		},
		get remoteProvider() {
			return remoteProvider;
		},
		get remoteConfigs() {
			return remoteConfigs;
		},
		get remoteProviders() {
			return remoteProviders;
		},
		get activeRemotePreset() {
			return remoteProviders.find((p) => p.id === remoteProvider) ?? null;
		},
		get globalShortcut() {
			return globalShortcut;
		},
		get shortcutStatus() {
			return shortcutStatus;
		},
		get inlineReplace() {
			return inlineReplace;
		},
		get themeMode() {
			return themeMode;
		},
		get activeModelId() {
			return activeModelId;
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
		loadApiKey,
		setRemoteProvider,
		setRemoteConfig,
		setGlobalShortcut,
		configureSystemShortcut,
		refreshShortcutStatus,
		setInlineReplace,
		setThemeMode,
		setActiveModel,
		setUseRemote,
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
