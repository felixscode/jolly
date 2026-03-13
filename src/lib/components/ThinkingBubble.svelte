<script lang="ts">
	let {
		active,
		scale = 1.0,
		scaleSpeed = 2,
		offsetX = 0,
		offsetY = 0,
		boxTop = 5,
		boxLeft = 42,
		boxW = 72,
		boxH = 38,
		debugBubbles = false,
		isDark = false
	}: {
		active: boolean;
		scale?: number;
		scaleSpeed?: number;
		offsetX?: number;
		offsetY?: number;
		boxTop?: number;
		boxLeft?: number;
		boxW?: number;
		boxH?: number;
		debugBubbles?: boolean;
		isDark?: boolean;
	} = $props();

	const minScale = $derived(scale * 0.875);
	const maxScale = $derived(scale * 1.125);

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
	let phase = $state<'hidden' | 'wrong' | 'right'>('hidden');
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
		phase = 'wrong';

		wordTimer = setTimeout(() => {
			phase = 'hidden';
			wordTimer = setTimeout(() => {
				phase = 'right';
				wordTimer = setTimeout(() => {
					phase = 'hidden';
					wordTimer = setTimeout(() => {
						if (active) cycleWords();
					}, 200);
				}, 900);
			}, 150);
		}, 700);
	}

	$effect(() => {
		if (active) {
			phase = 'hidden';
			const startTimer = setTimeout(() => cycleWords(), 300);
			return () => {
				clearTimeout(startTimer);
				clearTimers();
				phase = 'hidden';
			};
		} else {
			clearTimers();
			phase = 'hidden';
		}
	});
</script>

{#if active}
	<div
		class="thinking-bubble pointer-events-none absolute bottom-full"
		style="
			left: 50%;
			animation: bubble-pulse {scaleSpeed}s linear infinite;
			--min-scale: {minScale};
			--max-scale: {maxScale};
			--offset-x: {offsetX}px;
			--offset-y: {offsetY}px;
		"
	>
		<img
			src={isDark ? '/jolly_thinking_bubble_dark.svg' : '/jolly_thinking_bubble.svg'}
			alt=""
			style="display: block; width: 150px; max-width: none;"
		/>
		<div
			class="pointer-events-none absolute flex items-center justify-center"
			class:outline={debugBubbles}
			class:outline-red-500={debugBubbles}
			style="top:{boxTop}px; left:{boxLeft}px; width:{boxW}px; height:{boxH}px;"
		>
			{#if phase === 'wrong'}
				<span class="word-fade text-jolly-accent text-xs font-semibold line-through"
					>{current.wrong}</span
				>
			{:else if phase === 'right'}
				<span class="word-fade text-accent text-xs font-semibold">{current.right}</span>
			{/if}
		</div>
	</div>
{/if}

<style>
	@keyframes bubble-pulse {
		0%,
		100% {
			transform: translate(calc(-50% + var(--offset-x)), var(--offset-y)) scale(var(--min-scale));
		}
		50% {
			transform: translate(calc(-50% + var(--offset-x)), var(--offset-y)) scale(var(--max-scale));
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
