<script lang="ts">
	import { onMount } from 'svelte';
	import ThinkingBubble from '$lib/components/ThinkingBubble.svelte';

	// ── Scene configuration ────────────────────────────────────────────────────
	const BRANCH_SCALE = 1.4;
	const BRANCH_OFFSET_X = -10;
	const BRANCH_OFFSET_Y = -67;
	const BIRD_LAND_X = -150;
	const FLY_DEPART_X = 260;
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
			: scene === 'hovering' || scene === 'correcting'
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
				}, 3000);
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
				scene = 'idle';
			}
		}

		async function handleCorrection() {
			if (scene !== 'idle' && scene !== 'hovering' && scene !== 'greeting') return;
			clearTimeout(greetingTimer);
			scene = 'correcting';

			try {
				const text = await navigator.clipboard.readText();
				if (!text) {
					quoteText = 'Nothing to fix here!';
				} else {
					const res = await fetch('/api/correct', {
						method: 'POST',
						headers: { 'Content-Type': 'application/json' },
						body: JSON.stringify({ text })
					});

					if (res.ok) {
						const { correctedText } = await res.json();
						await navigator.clipboard.writeText(correctedText);
					}
					quoteText = pickQuote();
				}
			} catch (err) {
				console.error('Clipboard correction failed:', err);
				quoteText = pickQuote();
			}

			if (!mounted) return;
			scene = 'quoting';
			quotingTimer = setTimeout(() => {
				if (!mounted) return;
				if (scene === 'quoting') scene = 'idle';
			}, 3000);
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

<div
	bind:this={containerEl}
	class="mx-auto flex max-w-4xl flex-col px-6"
	style="min-height: calc(100svh - 130px);"
>
	<!-- Hero: fills available height, branch scene centered vertically -->
	<div class="grid flex-1 grid-cols-2 items-center gap-16">
		<!-- Left: copy -->
		<div>
			<p class="mb-3 text-xs font-semibold tracking-widest text-[#960200] uppercase">
				Your writing companion
			</p>
			<h1 class="mb-6 text-4xl leading-tight font-extrabold text-[#241e4e]">
				Catch typos.<br />Keep your voice.
			</h1>
			<p class="mb-10 leading-relaxed text-gray-500">
				Jolly watches what you type, catches your typos, and suggests fixes — with a bit of wit. No
				red squiggles. No drama. No judgement.
			</p>
			<a
				href="/download"
				class="inline-block rounded-lg border-4 border-[#241e4e] bg-[#960200] px-6 py-3 text-sm font-bold text-white transition-opacity hover:opacity-90"
			>
				Download Jolly — it's free
			</a>
		</div>

		<!-- Right: character scene -->
		<div class="flex flex-col items-center select-none">
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
						<!-- Talk bubble -->
						<div
							class="pointer-events-none absolute bottom-full left-1/2 mb-2 -translate-x-1/2 rounded-xl border-2 border-[#241e4e] bg-white px-4 py-3 text-center text-sm font-medium whitespace-pre-line text-[#241e4e] shadow-sm"
							style="opacity: {bubbleVisible ? 1 : 0}; transition: opacity 300ms ease;"
						>
							{bubbleText}
							<div
								class="absolute -bottom-[9px] left-1/2 flex -translate-x-1/2 flex-col items-center gap-[3px]"
							>
								<div class="h-2 w-2 rounded-full border-2 border-[#241e4e] bg-white"></div>
								<div class="h-1.5 w-1.5 rounded-full border-2 border-[#241e4e] bg-white"></div>
							</div>
						</div>

						<!-- Thinking bubble (clipboard correction) -->
						<ThinkingBubble
							active={scene === 'correcting'}
							minScale={0.9}
							maxScale={1.15}
							scaleSpeed={2}
						/>

						<!-- Layered bird poses -->
						<div class="relative h-44">
							<img
								src="/jolly_normal.svg"
								alt="Jolly"
								class="absolute bottom-0 left-1/2 h-full -translate-x-1/2 transition-opacity duration-150"
								style="opacity: {pose === 'normal' ? 1 : 0}"
							/>
							<img
								src="/jolly_blink.svg"
								alt=""
								aria-hidden="true"
								class="absolute bottom-0 left-1/2 h-full -translate-x-1/2 transition-opacity duration-150"
								style="opacity: {pose === 'blink' ? 1 : 0}"
							/>
							<img
								src="/jolly_fly1.svg"
								alt=""
								aria-hidden="true"
								class="absolute bottom-0 left-1/2 h-full -translate-x-1/2 transition-opacity duration-150"
								style="opacity: {pose === 'fly1' ? 1 : 0}"
							/>
							<img
								src="/jolly_fly2.svg"
								alt=""
								aria-hidden="true"
								class="absolute bottom-0 left-1/2 h-full -translate-x-1/2 transition-opacity duration-150"
								style="opacity: {pose === 'fly2' ? 1 : 0}"
							/>
							<img
								src="/jolly_thinking.svg"
								alt=""
								aria-hidden="true"
								class="absolute bottom-0 left-1/2 h-full -translate-x-1/2 transition-opacity duration-150"
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
	</div>
	<!-- /grid -->

	<hr class="border-gray-200" />

	<!-- Feature strip -->
	<div class="my-16 grid grid-cols-3 gap-16">
		<div>
			<h2 class="mb-3 text-xl font-bold text-[#241e4e]">Local</h2>
			<p class="text-sm leading-relaxed text-gray-500">
				Runs entirely on your machine. Nothing leaves your computer — no cloud, no account required.
			</p>
		</div>
		<div>
			<h2 class="mb-3 text-xl font-bold text-[#241e4e]">Quiet</h2>
			<p class="text-sm leading-relaxed text-gray-500">
				No notifications, no interruptions. Jolly stays in the background until a fix is worth
				mentioning.
			</p>
		</div>
		<div>
			<h2 class="mb-3 text-xl font-bold text-[#241e4e]">Free</h2>
			<p class="text-sm leading-relaxed text-gray-500">
				Open source and free forever. Download it, use it, read the source code if you're curious.
			</p>
		</div>
	</div>
</div>
