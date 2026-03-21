import type { Linter } from 'harper.js';

let linter: Linter | null = null;

async function getLinter(): Promise<Linter> {
	if (linter) return linter;

	const harper = await import('harper.js');
	linter = new harper.WorkerLinter({
		binary: harper.binary,
		dialect: harper.Dialect.American
	});
	return linter;
}

// Errors propagate to BirdScene's catch handler ("Oops, couldn't fix that!")
export async function harperCorrect(text: string): Promise<string> {
	if (!text.trim()) return text;

	const l = await getLinter();
	const lints = await l.lint(text);

	if (lints.length === 0) return text;

	// Sort lints by span start descending (reverse order)
	// so applying fixes from end-to-start preserves earlier spans
	const sorted = [...lints].sort((a, b) => b.span().start - a.span().start);

	let result = text;
	let lastStart = text.length;
	for (const lint of sorted) {
		if (lint.suggestion_count() === 0) continue;
		const span = lint.span();
		// Skip overlapping lints (same guard as Rust-side HarperProvider)
		if (span.end > lastStart) continue;
		const suggestion = lint.suggestions()[0];
		result = await l.applySuggestion(result, lint, suggestion);
		lastStart = span.start;
	}

	return result;
}
