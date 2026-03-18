import { env } from '$env/dynamic/private';
import { OpenRouterProvider } from './openrouter';
import type { LLMProvider } from './types';

export function createLLMProvider(): LLMProvider {
	return new OpenRouterProvider(env.OPENROUTER_API_KEY ?? '');
}
