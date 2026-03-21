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
