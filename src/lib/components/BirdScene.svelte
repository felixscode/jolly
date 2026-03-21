<script lang="ts">
	import { onMount } from 'svelte';
	import ThinkingBubble from '$lib/components/ThinkingBubble.svelte';

	let {
		onCorrect
	}: {
		onCorrect: (text: string) => Promise<string>;
	} = $props();

	// ── Scene configuration ────────────────────────────────────────────────────
	// Bird visual center offset (eye position as % of image, relative to geometric center)
	const BIRD_EYE_OFFSET_X = 10; // eye is 10% right of geometric center
	const BIRD_EYE_OFFSET_Y = 25; // eye is 25% above geometric center

	// Branch positioning relative to the bird
	const BRANCH_ANCHOR_X = -90; // px from bird left edge to branch image left edge
	const BRANCH_ANCHOR_Y = -75; // px offset from bird bottom to branch trunk
	const BRANCH_WIDTH = 135; // rem
	const BRANCH_FADE_START = 75; // % from left where fade begins
	const BRANCH_FADE_END = 75; // % from left where fully transparent

	// Fly-in animation
	const FLY_DEPART_X = 260; // bird flies in from this many px to the right

	// ── Bubble tuning ──────────────────────────────────────────────────────────
	const DEBUG_BUBBLES = false;

	const TALK_BUBBLE_OFFSET_X = 140;
	const TALK_BUBBLE_OFFSET_Y = -10;
	const TALK_BOX_TOP = 16;
	const TALK_BOX_LEFT = 40;
	const TALK_BOX_W = 128;
	const TALK_BOX_H = 58;

	const THINK_SCALE = 1.0;
	const THINK_PULSE = 0.03;
	const THINK_SCALE_SPEED = 3;
	const THINK_BUBBLE_OFFSET_X = 140;
	const THINK_BUBBLE_OFFSET_Y = -10;
	const THINK_BOX_TOP = 16;
	const THINK_BOX_LEFT = 40;
	const THINK_BOX_W = 85;
	const THINK_BOX_H = 50;
	// ───────────────────────────────────────────────────────────────────────────

	type SceneState =
		| 'flying'
		| 'landed'
		| 'greeting'
		| 'idle'
		| 'hovering'
		| 'correcting'
		| 'quoting';
	type Pose = 'normal' | 'blink' | 'fly1' | 'fly2' | 'thinking' | 'dead';

	let scene = $state<SceneState>('flying');
	let flyFrame = $state<1 | 2>(1);
	let blinking = $state(false);
	let flyingIn = $state(true);
	let dead = $state(false);

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

	const pose = $derived<Pose>(
		dead
			? 'dead'
			: scene === 'flying'
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
	let onClickRef: (() => void) | undefined;

	function pickQuote(): string {
		let idx: number;
		do {
			idx = Math.floor(Math.random() * quotes.length);
		} while (idx === lastQuoteIdx && quotes.length > 1);
		lastQuoteIdx = idx;
		return quotes[idx];
	}

	async function writeClipboard(text: string): Promise<void> {
		try {
			await navigator.clipboard.writeText(text);
		} catch {
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

			setTimeout(() => {
				if (!mounted) return;
				scene = 'greeting';
				greetingTimer = setTimeout(() => {
					if (!mounted) return;
					if (scene === 'greeting') scene = 'idle';
				}, 30000);
			}, 500);
		}, 2300);

		// Blink loop
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

		async function handleCorrection(text: string) {
			if (scene !== 'idle' && scene !== 'hovering' && scene !== 'greeting') return;
			clearTimeout(greetingTimer);
			scene = 'correcting';
			const minDelay = new Promise((r) => setTimeout(r, 5000));

			if (!text) {
				quoteText = 'Nothing to fix here!';
			} else {
				try {
					const correctedText = await onCorrect(text);
					await writeClipboard(correctedText);
					quoteText = pickQuote();
				} catch (err) {
					console.error('Correction failed:', err);
					const code = String(err);
					if (code.includes('no_api_key')) {
						quoteText = 'No API key set — add one in Settings!';
					} else if (code.includes('bad_api_key')) {
						quoteText = 'API key seems wrong — check Settings!';
					} else if (code.includes('model_not_loaded')) {
						quoteText = 'Model not loaded — grab one in Settings!';
					} else if (code.includes('local_inference_failed')) {
						quoteText = "Hmm, the model couldn't handle that — try again!";
					} else if (code.includes('api_failed')) {
						quoteText = "Can't reach the API — check your connection!";
					} else {
						quoteText = "Oops, couldn't fix that!";
					}
				}
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

		async function readClipboard(): Promise<string> {
			try {
				const { readText } = await import('@tauri-apps/plugin-clipboard-manager');
				return await readText();
			} catch {
				return await navigator.clipboard.readText();
			}
		}

		async function handleKeydown(e: KeyboardEvent) {
			if (e.key !== 'Enter') return;
			const tag = (e.target as Element)?.tagName;
			if (tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT') return;
			let text: string;
			try {
				text = await readClipboard();
			} catch (err) {
				console.error('Clipboard read failed:', err);
				return;
			}
			handleCorrection(text);
		}

		let deadTimer: ReturnType<typeof setTimeout> | undefined;
		function handleClick() {
			if (scene === 'flying' || scene === 'correcting') return;
			dead = true;
			clearTimeout(deadTimer);
			deadTimer = setTimeout(() => {
				dead = false;
			}, 800);
		}

		window.addEventListener('keydown', handleKeydown);
		onEnterRef = handleEnter;
		onLeaveRef = handleLeave;
		onClickRef = handleClick;

		return () => {
			mounted = false;
			clearInterval(flyInterval);
			clearTimeout(t1);
			clearTimeout(t2);
			clearTimeout(t3);
			clearTimeout(greetingTimer);
			clearTimeout(quotingTimer);
			clearTimeout(blinkTimer);
			clearTimeout(deadTimer);
			window.removeEventListener('keydown', handleKeydown);
		};
	});
</script>

<!-- Container: relative anchor for bird + branch -->
<div
	class="relative select-none"
	style="transform: translate({-BIRD_EYE_OFFSET_X}%, {BIRD_EYE_OFFSET_Y}%);"
>
	<!-- Position layer: fly-in animation (lands at 0,0 = centered) -->
	<div
		style="
		transform: {flyingIn ? `translate(${FLY_DEPART_X}px, -300px)` : 'translate(0, 0)'};
		opacity: {flyingIn ? 0 : 1};
		transition: transform 1800ms cubic-bezier(0.3, 0.3, 0.8, 0.9), opacity 500ms ease-out;
	"
	>
		<!-- Scale layer: 0.75 → 1 during fly-in -->
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
				onclick={() => onClickRef?.()}
				role="img"
				aria-label="Jolly"
			>
				<!-- Talk bubble -->
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
							<span class="text-sm font-bold whitespace-pre-line text-[#423f37]">{bubbleText}</span>
						</div>
					</div>
				</div>

				<!-- Thinking bubble -->
				<ThinkingBubble
					active={scene === 'correcting'}
					scale={THINK_SCALE}
					pulseAmount={THINK_PULSE}
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
				<div class="relative h-32 w-28 md:h-44 md:w-36">
					<img
						src="/jolly_normal.svg"
						alt="Jolly"
						class="absolute inset-0 h-full w-full object-contain object-bottom"
						style="opacity: {pose === 'normal' ? 1 : 0}; transition: opacity 0ms;"
					/>
					<img
						src="/jolly_bilnzel.svg"
						alt=""
						aria-hidden="true"
						class="absolute inset-0 h-full w-full object-contain object-bottom"
						style="opacity: {pose === 'blink' ? 1 : 0}; transition: opacity 0ms;"
					/>
					<img
						src="/jolly_fly1.svg"
						alt=""
						aria-hidden="true"
						class="absolute inset-0 h-full w-full object-contain object-bottom"
						style="opacity: {pose === 'fly1' ? 1 : 0}; transition: opacity 0ms;"
					/>
					<img
						src="/jolly_fly2.svg"
						alt=""
						aria-hidden="true"
						class="absolute inset-0 h-full w-full object-contain object-bottom"
						style="opacity: {pose === 'fly2' ? 1 : 0}; transition: opacity 0ms;"
					/>
					<img
						src="/jolly_thinking.svg"
						alt=""
						aria-hidden="true"
						class="absolute inset-0 h-full w-full object-contain object-bottom"
						style="opacity: {pose === 'thinking' ? 1 : 0}; transition: opacity 0ms;"
					/>
					<img
						src="/jolly_dead.svg"
						alt=""
						aria-hidden="true"
						class="absolute inset-0 h-full w-full object-contain object-bottom"
						style="opacity: {pose === 'dead' ? 1 : 0}; transition: opacity 0ms;"
					/>
				</div>
			</div>
		</div>
	</div>

	<!-- Branch: positioned relative to bird -->
	<img
		src="/jolly_tree.svg"
		alt=""
		aria-hidden="true"
		class="pointer-events-none absolute"
		style="
			width: {BRANCH_WIDTH}rem;
			max-width: none;
			top: calc(100% + {BRANCH_ANCHOR_Y}px);
			left: {BRANCH_ANCHOR_X}px;
			-webkit-mask-image: linear-gradient(to right, black {BRANCH_FADE_START}%, transparent {BRANCH_FADE_END}%);
			mask-image: linear-gradient(to right, black {BRANCH_FADE_START}%, transparent {BRANCH_FADE_END}%);
		"
	/>
</div>
