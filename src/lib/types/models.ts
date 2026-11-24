export interface ModelEntry {
	id: string;
	name: string;
	fileName: string;
	url: string;
	sizeBytes: number;
	sha256: string;
}

export type DownloadState =
	| { state: 'available' }
	| { state: 'partial'; bytesReceived: number; totalBytes: number }
	| { state: 'downloaded' };

export type ModelWithState = ModelEntry & DownloadState;

export interface CustomModel {
	id: string;
	name: string;
	path: string;
}

export interface RemoteProviderPreset {
	id: string;
	name: string;
	kind: 'openAiCompatible' | 'anthropic';
	baseUrl: string;
	defaultModel: string;
	keyEnv: string;
	keyPlaceholder: string;
	signupUrl: string;
	needsKey: boolean;
}

/** User overrides for a remote provider; empty fields fall back to the preset. */
export interface RemoteConfig {
	model?: string;
	baseUrl?: string;
}

export interface HistoryEntry {
	/** Text before correction (absent for entries from older versions). */
	original?: string;
	corrected: string;
	at: number;
}

export interface ShortcutStatus {
	accelerator: string | null;
	active: boolean;
	backend: 'native' | 'portal' | 'none';
	triggerDescription: string | null;
	error: string | null;
}
