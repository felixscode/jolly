import { json, error } from '@sveltejs/kit';
import { createLLMProvider } from '$lib/server/llm';
import type { RequestHandler } from './$types';

export const POST: RequestHandler = async ({ request }) => {
	const body = await request.json().catch(() => null);

	if (!body || typeof body.text !== 'string' || body.text.length === 0) {
		throw error(400, 'Missing or empty "text" field');
	}

	if (body.text.length > 10_000) {
		throw error(400, 'Text exceeds 10,000 character limit');
	}

	const provider = createLLMProvider();
	const result = await provider.correctText(body.text);

	return json({ correctedText: result.correctedText });
};
