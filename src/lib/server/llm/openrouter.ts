import type { CorrectionResult, LLMProvider } from './types';

export class OpenRouterProvider implements LLMProvider {
	private apiKey: string;
	private model: string;

	constructor(apiKey: string, model = 'openai/gpt-4o-mini') {
		this.apiKey = apiKey;
		this.model = model;
	}

	async correctText(text: string): Promise<CorrectionResult> {
		const controller = new AbortController();
		const timeout = setTimeout(() => controller.abort(), 30_000);

		try {
			const res = await fetch('https://openrouter.ai/api/v1/chat/completions', {
				method: 'POST',
				headers: {
					Authorization: `Bearer ${this.apiKey}`,
					'Content-Type': 'application/json'
				},
				body: JSON.stringify({
					model: this.model,
					messages: [
						{
							role: 'system',
							content: 'Return ONLY the corrected text, no commentary.'
						},
						{ role: 'user', content: text }
					]
				}),
				signal: controller.signal
			});

			if (!res.ok) {
				const body = await res.text();
				throw new Error(`OpenRouter API error ${res.status}: ${body}`);
			}

			const data = await res.json();
			const correctedText = data.choices?.[0]?.message?.content?.trim() ?? text;
			return { correctedText };
		} finally {
			clearTimeout(timeout);
		}
	}
}
