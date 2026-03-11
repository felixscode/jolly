<script lang="ts">
	let {
		active,
		minScale = 0.9,
		maxScale = 1.15,
		scaleSpeed = 2
	}: {
		active: boolean;
		minScale?: number;
		maxScale?: number;
		scaleSpeed?: number;
	} = $props();

	const corrections = [
		{ wrong: 'recieve', right: 'receive' },
		{ wrong: 'definately', right: 'definitely' },
		{ wrong: 'wierd', right: 'weird' },
		{ wrong: 'occured', right: 'occurred' },
		{ wrong: 'seperate', right: 'separate' },
		{ wrong: 'teh', right: 'the' },
		{ wrong: 'accomodate', right: 'accommodate' },
		{ wrong: 'neccessary', right: 'necessary' },
		{ wrong: 'embarass', right: 'embarrass' },
		{ wrong: 'goverment', right: 'government' }
	];

	let current = $state(corrections[0]);
	let showWord = $state(false);
	let lastIdx = -1;

	function pickRandom() {
		let idx: number;
		do {
			idx = Math.floor(Math.random() * corrections.length);
		} while (idx === lastIdx);
		lastIdx = idx;
		return corrections[idx];
	}

	let wordTimer: ReturnType<typeof setTimeout> | undefined;

	function clearTimers() {
		clearTimeout(wordTimer);
		wordTimer = undefined;
	}

	function cycleWords() {
		current = pickRandom();
		showWord = true;

		wordTimer = setTimeout(() => {
			showWord = false;
			wordTimer = setTimeout(() => {
				if (active) cycleWords();
			}, 200);
		}, 1300);
	}

	$effect(() => {
		if (active) {
			showWord = false;
			const startTimer = setTimeout(() => cycleWords(), 300);
			return () => {
				clearTimeout(startTimer);
				clearTimers();
				showWord = false;
			};
		} else {
			clearTimers();
			showWord = false;
		}
	});
</script>

{#if active}
	<div
		class="thinking-bubble absolute bottom-full left-1/2 mb-2 pointer-events-none"
		style="
			animation: bubble-pulse {scaleSpeed}s linear infinite;
			--min-scale: {minScale};
			--max-scale: {maxScale};
		"
	>
		<img src="/jolly_thinking_bubble.svg" alt="" class="w-32" />
		{#if showWord}
			<span
				class="word-fade absolute inset-0 flex items-center justify-center text-xs font-medium whitespace-nowrap"
			>
				<span class="text-jolly-accent line-through">{current.wrong}</span>
				<span class="text-accent ml-1">&rarr; {current.right}</span>
			</span>
		{/if}
	</div>
{/if}

<style>
	@keyframes bubble-pulse {
		0%,
		100% {
			transform: translate(-50%, 0) scale(var(--min-scale));
		}
		50% {
			transform: translate(-50%, 0) scale(var(--max-scale));
		}
	}

	@keyframes word-fade {
		0% {
			opacity: 0;
		}
		15% {
			opacity: 1;
		}
		85% {
			opacity: 1;
		}
		100% {
			opacity: 0;
		}
	}

	.thinking-bubble {
		transform-origin: bottom center;
	}

	.word-fade {
		animation: word-fade 1.3s ease-in-out forwards;
	}
</style>
