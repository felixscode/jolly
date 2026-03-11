import { OPENROUTER_API_KEY } from '$env/static/private';
import { OpenRouterProvider } from './openrouter';
import type { LLMProvider } from './types';

export function createLLMProvider(): LLMProvider {
	return new OpenRouterProvider(OPENROUTER_API_KEY);
}
