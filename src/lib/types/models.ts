export interface Model {
	id: string;
	name: string;
	sizeGb: number;
	inferenceMs: number; // ms per token
}

export const AVAILABLE_MODELS: Model[] = [
	{ id: 'llama-3.2-1b', name: 'Llama 3.2 1B', sizeGb: 1.2, inferenceMs: 45 },
	{ id: 'llama-3.2-3b', name: 'Llama 3.2 3B', sizeGb: 3.4, inferenceMs: 95 },
	{ id: 'phi-3-mini', name: 'Phi-3 Mini', sizeGb: 2.3, inferenceMs: 70 },
	{ id: 'mistral-7b', name: 'Mistral 7B', sizeGb: 4.1, inferenceMs: 130 },
	{ id: 'gemma-2b', name: 'Gemma 2B', sizeGb: 1.5, inferenceMs: 55 }
];
