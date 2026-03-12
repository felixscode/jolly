<script lang="ts">
	import { onMount } from 'svelte';
	import ThinkingBubble from '$lib/components/ThinkingBubble.svelte';

	let {
		onCorrect,
		compact = false
	}: {
		onCorrect: (text: string) => Promise<string>;
		compact?: boolean;
	} = $props();

	// ── Scene configuration ────────────────────────────────────────────────────
	const BRANCH_SCALE = 1.4;
	const BRANCH_OFFSET_X = -10;
	const BRANCH_OFFSET_Y = -67;
	const BIRD_LAND_X = -150;
	const FLY_DEPART_X = 260;

	// ── Bubble tuning ──────────────────────────────────────────────────────────
	const DEBUG_BUBBLES = false;

	const TALK_BUBBLE_OFFSET_X = 140;
	const TALK_BUBBLE_OFFSET_Y = -10;
	const TALK_BOX_TOP = 13;
	const TALK_BOX_LEFT = 48;
	const TALK_BOX_W = 128;
	const TALK_BOX_H = 58;

	const THINK_SCALE = 2.0;
	const THINK_SCALE_SPEED = 2;
	const THINK_BUBBLE_OFFSET_X = 140;
	const THINK_BUBBLE_OFFSET_Y = -10;
	const THINK_BOX_TOP = 5;
	const THINK_BOX_LEFT = 42;
	const THINK_BOX_W = 72;
	const THINK_BOX_H = 38;
	// ───────────────────────────────────────────────────────────────────────────

	type SceneState =
		| 'flying'
		| 'landed'
		| 'greeting'
		| 'idle'
		| 'hovering'
		| 'correcting'
		| 'quoting';
	type Pose = 'normal' | 'blink' | 'fly1' | 'fly2' | 'thinking';

	let scene = $state<SceneState>('flying');
	let flyFrame = $state<1 | 2>(1);
	let blinking = $state(false);
	let flyingIn = $state(true);
	let startY = $state(-400);

	const GREETING_TEXT = "Hi, I'm Jolly!\npress Enter";

	const quotes = [
		"You're welcome, Shakespeare.",
		'Another one bites the dust.',
		'I do this for free, you know.',
		'English is hard. I get it.',
		"Fixed. Don't let it happen again.",
		'Your keyboard owes me one.'
	];
	let quoteText = $state(quotes[0]);
	let lastQuoteIdx = -1;

	let containerEl: HTMLDivElement | null = null;
	let posLayerEl: HTMLDivElement | null = null;

	function offsetTop(el: HTMLElement): number {
		let top = 0;
		let cur: HTMLElement | null = el;
		while (cur) {
			top += cur.offsetTop;
			cur = cur.offsetParent as HTMLElement | null;
		}
		return top;
	}

	const pose = $derived<Pose>(
		scene === 'flying'
			? flyFrame === 1
				? 'fly1'
				: 'fly2'
			: scene === 'correcting'
				? 'thinking'
				: blinking
					? 'blink'
					: 'normal'
	);

	const bubbleVisible = $derived(
		scene === 'greeting' || scene === 'hovering' || scene === 'quoting'
	);

	const bubbleText = $derived(scene === 'quoting' ? quoteText : GREETING_TEXT);

	let onEnterRef: (() => void) | undefined;
	let onLeaveRef: (() => void) | undefined;

	function pickQuote(): string {
		let idx: number;
		do {
			idx = Math.floor(Math.random() * quotes.length);
		} while (idx === lastQuoteIdx && quotes.length > 1);
		lastQuoteIdx = idx;
		return quotes[idx];
	}

	async function readClipboard(): Promise<string> {
		try {
			return await navigator.clipboard.readText();
		} catch {
			// Fallback to Tauri clipboard plugin if available
			try {
				const { readText } = await import('@tauri-apps/plugin-clipboard-manager');
				return await readText();
			} catch {
				throw new Error('Clipboard access denied');
			}
		}
	}

	async function writeClipboard(text: string): Promise<void> {
		try {
			await navigator.clipboard.writeText(text);
		} catch {
			// Fallback to Tauri clipboard plugin if available
			try {
				const { writeText } = await import('@tauri-apps/plugin-clipboard-manager');
				await writeText(text);
			} catch {
				throw new Error('Clipboard write denied');
			}
		}
	}

	onMount(() => {
		let mounted = true;
		let greetingTimer: ReturnType<typeof setTimeout>;
		let quotingTimer: ReturnType<typeof setTimeout>;
		let blinkTimer: ReturnType<typeof setTimeout>;

		// Measure fly-in start position
		if (containerEl && posLayerEl) {
			startY = -(offsetTop(posLayerEl) - offsetTop(containerEl));
		}

		// Alternate fly frames while flying
		const flyInterval = setInterval(() => {
			flyFrame = flyFrame === 1 ? 2 : 1;
		}, 150);

		// Kick off fly-in CSS transition
		const t1 = setTimeout(() => {
			flyingIn = false;
		}, 50);

		// Land after flight
		const t2 = setTimeout(() => {
			if (!mounted) return;
			clearInterval(flyInterval);
			scene = 'landed';

			// Brief pause, then greeting
			setTimeout(() => {
				if (!mounted) return;
				scene = 'greeting';
				greetingTimer = setTimeout(() => {
					if (!mounted) return;
					if (scene === 'greeting') scene = 'idle';
				}, 30000);
			}, 500);
		}, 2300);

		// Blink loop — only blinks during greeting, idle, or quoting
		function scheduleBlink() {
			const delay = 2500 + Math.random() * 4000;
			blinkTimer = setTimeout(() => {
				if (!mounted) return;
				const canBlink = scene === 'idle' || scene === 'greeting' || scene === 'quoting';
				if (canBlink && !blinking) {
					blinking = true;
					blinkTimer = setTimeout(() => {
						if (!mounted) return;
						blinking = false;
						scheduleBlink();
					}, 160);
				} else {
					scheduleBlink();
				}
			}, delay);
		}

		const t3 = setTimeout(scheduleBlink, 3300);

		// Mouse handlers stored so they can reference timers
		function handleEnter() {
			if (scene === 'idle' || scene === 'greeting') {
				clearTimeout(greetingTimer);
				scene = 'hovering';
			}
		}

		function handleLeave() {
			if (scene === 'hovering') {
				scene = 'greeting';
				greetingTimer = setTimeout(() => {
					if (!mounted) return;
					if (scene === 'greeting') scene = 'idle';
				}, 30000);
			}
		}

		async function handleCorrection() {
			if (scene !== 'idle' && scene !== 'hovering' && scene !== 'greeting') return;
			clearTimeout(greetingTimer);
			scene = 'correcting';
			const minDelay = new Promise((r) => setTimeout(r, 5000));

			try {
				const text = await readClipboard();
				if (!text) {
					quoteText = 'Nothing to fix here!';
				} else {
					try {
						const correctedText = await onCorrect(text);
						await writeClipboard(correctedText);
						quoteText = pickQuote();
					} catch (err) {
						console.error('Correction failed:', err);
						quoteText = "Oops, couldn't fix that!";
					}
				}
			} catch (err) {
				console.error('Clipboard read failed:', err);
				quoteText = "Oops, couldn't fix that!";
			}

			await minDelay;
			if (!mounted) return;
			scene = 'quoting';
			quotingTimer = setTimeout(() => {
				if (!mounted) return;
				if (scene === 'quoting') {
					scene = 'greeting';
					greetingTimer = setTimeout(() => {
						if (!mounted) return;
						if (scene === 'greeting') scene = 'idle';
					}, 30000);
				}
			}, 5000);
		}

		function handleKeydown(e: KeyboardEvent) {
			if (e.key !== 'Enter') return;
			const tag = (e.target as Element)?.tagName;
			if (tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT') return;
			handleCorrection();
		}

		// Attach handlers to window and expose to template
		window.addEventListener('keydown', handleKeydown);
		onEnterRef = handleEnter;
		onLeaveRef = handleLeave;

		return () => {
			mounted = false;
			clearInterval(flyInterval);
			clearTimeout(t1);
			clearTimeout(t2);
			clearTimeout(t3);
			clearTimeout(greetingTimer);
			clearTimeout(quotingTimer);
			clearTimeout(blinkTimer);
			window.removeEventListener('keydown', handleKeydown);
		};
	});
</script>

<div bind:this={containerEl} class="flex flex-col items-center select-none">
	<!-- Position layer: X + Y with cubic-bezier easing, opacity fade-in -->
	<div
		bind:this={posLayerEl}
		style="
		transform: {flyingIn
			? `translate(${BIRD_LAND_X + FLY_DEPART_X}px, ${startY}px)`
			: `translate(${BIRD_LAND_X}px, 0)`};
		opacity: {flyingIn ? 0 : 1};
		transition: transform 1800ms cubic-bezier(0.3, 0.3, 0.8, 0.9), opacity 500ms ease-out;
	"
	>
		<!-- Scale layer: 0.75 → 1 linearly over the same duration -->
		<div
			style="
			transform: {flyingIn ? 'scale(0.75)' : 'scale(1)'};
			transition: transform 1800ms linear;
		"
		>
			<!-- Bird + bubbles -->
			<div
				class="relative cursor-pointer"
				onmouseenter={() => onEnterRef?.()}
				onmouseleave={() => onLeaveRef?.()}
				role="img"
				aria-label="Jolly"
			>
				<!-- Talk bubble (SVG oval) -->
				<div
					class="pointer-events-none absolute bottom-full"
					style="left: 50%; opacity: {bubbleVisible
						? 1
						: 0}; transition: opacity 300ms ease; transform: translate(calc(-50% + {TALK_BUBBLE_OFFSET_X}px), {TALK_BUBBLE_OFFSET_Y}px);"
				>
					<div class="relative">
						<img
							src="/jolly_talk.svg"
							alt=""
							aria-hidden="true"
							style="display: block; width: 180px; max-width: none;"
						/>
						<div
							class="pointer-events-none absolute flex items-center justify-center text-center"
							class:outline={DEBUG_BUBBLES}
							class:outline-red-500={DEBUG_BUBBLES}
							style="top:{TALK_BOX_TOP}px; left:{TALK_BOX_LEFT}px; width:{TALK_BOX_W}px; height:{TALK_BOX_H}px;"
						>
							<span class="text-sm font-bold whitespace-pre-line text-[#241e4e]"
								>{bubbleText}</span
							>
						</div>
					</div>
				</div>

				<!-- Thinking bubble (clipboard correction) -->
				<ThinkingBubble
					active={scene === 'correcting'}
					scale={THINK_SCALE}
					scaleSpeed={THINK_SCALE_SPEED}
					offsetX={THINK_BUBBLE_OFFSET_X}
					offsetY={THINK_BUBBLE_OFFSET_Y}
					boxTop={THINK_BOX_TOP}
					boxLeft={THINK_BOX_LEFT}
					boxW={THINK_BOX_W}
					boxH={THINK_BOX_H}
					debugBubbles={DEBUG_BUBBLES}
				/>

				<!-- Layered bird poses -->
				<div class="relative {compact ? 'h-32 w-28' : 'h-44 w-36'}">
					<img
						src="/jolly_normal.svg"
						alt="Jolly"
						class="absolute inset-0 h-full w-full object-contain object-bottom"
						style="opacity: {pose === 'normal' ? 1 : 0}"
					/>
					<img
						src="/jolly_blink.svg"
						alt=""
						aria-hidden="true"
						class="absolute inset-0 h-full w-full object-contain object-bottom"
						style="opacity: {pose === 'blink' ? 1 : 0}"
					/>
					<img
						src="/jolly_fly1.svg"
						alt=""
						aria-hidden="true"
						class="absolute inset-0 h-full w-full object-contain object-bottom"
						style="opacity: {pose === 'fly1' ? 1 : 0}"
					/>
					<img
						src="/jolly_fly2.svg"
						alt=""
						aria-hidden="true"
						class="absolute inset-0 h-full w-full object-contain object-bottom"
						style="opacity: {pose === 'fly2' ? 1 : 0}"
					/>
					<img
						src="/jolly_thinking.svg"
						alt=""
						aria-hidden="true"
						class="absolute inset-0 h-full w-full object-contain object-bottom"
						style="opacity: {pose === 'thinking' ? 1 : 0}"
					/>
				</div>
			</div>
		</div>
	</div>

	<!-- Branch -->
	<img
		src="/jolly_branch.svg"
		alt=""
		aria-hidden="true"
		style="width: {BRANCH_SCALE *
			100}%; max-width: none; transform: translateX({BRANCH_OFFSET_X}px); margin-top: {BRANCH_OFFSET_Y}px;"
	/>
</div>
