<script lang="ts">
	import { onMount } from 'svelte';
	import { fade } from 'svelte/transition';
	import ThinkingBubble from '$lib/components/ThinkingBubble.svelte';

	// ── Scene configuration ────────────────────────────────────────────────────
	const BRANCH_SCALE = 1.4; // width multiplier relative to column width
	const BRANCH_OFFSET_X = -10; // px: shift branch left(–) or right(+)
	const BRANCH_OFFSET_Y = -67; // px: bird–branch overlap (negative = overlap)
	const BIRD_LAND_X = -150; // px: Jolly's landing offset from column center
	const FLY_DEPART_X = 260; // px: additional horizontal offset at flight start
	const TALK_SCALE = 0.75; // scale of the talk bubble relative to its natural size
	const TALK_OFFSET_X = 150; // px: horizontal offset from bird center (positive = right)
	const TALK_OFFSET_Y = 0; // px: vertical offset above bird (negative = higher)
	const TALK_TEXT = "Hi, I'm Jolly! \n press Enter"; // text inside the bubble
	const TALK_TEXT_SIZE = 30; // px: font size
	const TALK_TEXT_OFFSET_X = 40; // px: nudge text horizontally inside bubble
	const TALK_TEXT_OFFSET_Y = -12; // px: nudge text vertically inside bubble
	// ───────────────────────────────────────────────────────────────────────────

	type JollyState = 'flying' | 'normal' | 'blink' | 'thinking' | 'talk';

	let jollyState = $state<JollyState>('flying');
	let flyFrame = $state(1);
	let landed = $state(false);
	let hovering = $state(false);
	let showBubble = $state(false);
	let flyingIn = $state(true);
	let startY = $state(-400); // computed from DOM at mount
	let correcting = $state(false);

	const thoughts = [
		'"recieve" → receive. Classic.',
		'"definately" → definitely. You\'re forgiven.',
		'"wierd" → weird. I before E... except here.',
		'"occured" → occurred. Double letters are hard.',
		'"seperate" → separate. A rat lives in separate.',
		'"teh" → the. Happens to the best of us.'
	];
	let thoughtIdx = $state(0);

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

	const jollyImg = $derived(
		jollyState === 'flying'
			? `/jolly_fly${flyFrame}.svg`
			: jollyState === 'talk'
				? '/jolly_talk.svg'
				: jollyState === 'blink'
					? '/jolly_blink.svg'
					: jollyState === 'thinking'
						? '/jolly_thinking.svg'
						: '/jolly_normal.svg'
	);

	onMount(() => {
		let mounted = true;

		// Measure: distance from container top to bird resting position
		if (containerEl && posLayerEl) {
			startY = -(offsetTop(posLayerEl) - offsetTop(containerEl));
		}

		// Alternate fly frames while landing
		const flyInterval = setInterval(() => {
			flyFrame = flyFrame === 1 ? 2 : 1;
		}, 150);

		// Move toward branch
		const t1 = setTimeout(() => {
			flyingIn = false;
		}, 50);

		// Land
		const t2 = setTimeout(() => {
			if (!mounted) return;
			clearInterval(flyInterval);
			jollyState = 'normal';
			landed = true;
		}, 2300);

		// Blink loop
		let blinkTimer: ReturnType<typeof setTimeout>;
		function scheduleBlink() {
			const delay = 2500 + Math.random() * 4000;
			blinkTimer = setTimeout(() => {
				if (!mounted) return;
				if (jollyState === 'normal' && !hovering && !correcting) {
					jollyState = 'blink';
					blinkTimer = setTimeout(() => {
						if (!mounted) return;
						if (!hovering) jollyState = 'normal';
						scheduleBlink();
					}, 160);
				} else {
					scheduleBlink();
				}
			}, delay);
		}

		const t3 = setTimeout(scheduleBlink, 3300);

		return () => {
			mounted = false;
			clearInterval(flyInterval);
			clearTimeout(t1);
			clearTimeout(t2);
			clearTimeout(t3);
			clearTimeout(blinkTimer);
		};
	});

	function onEnter() {
		if (!landed || correcting) return;
		hovering = true;
		jollyState = 'thinking';
		thoughtIdx = (thoughtIdx + 1) % thoughts.length;
		showBubble = true;
	}

	function onLeave() {
		if (correcting) return;
		hovering = false;
		jollyState = 'normal';
		showBubble = false;
	}

	function onBubbleTalk() {
		jollyState = 'talk';
	}

	function onBubbleThink() {
		jollyState = 'thinking';
	}

	async function correctClipboard() {
		correcting = true;
		jollyState = 'thinking';
		showBubble = false;

		try {
			const text = await navigator.clipboard.readText();
			if (!text) {
				correcting = false;
				jollyState = landed ? 'normal' : 'flying';
				return;
			}

			const res = await fetch('/api/correct', {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({ text })
			});

			if (res.ok) {
				const { correctedText } = await res.json();
				await navigator.clipboard.writeText(correctedText);
			}
		} catch (err) {
			console.error('Clipboard correction failed:', err);
		} finally {
			correcting = false;
			if (landed && !hovering) {
				jollyState = 'normal';
			}
		}
	}

	function onKeydown(e: KeyboardEvent) {
		if (e.key !== 'Enter' || correcting) return;
		const tag = (e.target as Element)?.tagName;
		if (tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT') return;
		correctClipboard();
	}
</script>

<svelte:window onkeydown={onKeydown} />

<div
	bind:this={containerEl}
	class="mx-auto flex max-w-4xl flex-col px-6"
	style="min-height: calc(100svh - 130px);"
>
	<!-- Hero: fills available height, branch scene centered vertically -->
	<div class="grid flex-1 grid-cols-2 items-center gap-16">
		<!-- Left: copy -->
		<div>
			<p class="mb-3 text-xs font-semibold tracking-widest uppercase text-[#960200]">
				Your writing companion
			</p>
			<h1 class="mb-6 text-4xl font-extrabold leading-tight text-[#241e4e]">
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
					<!-- Bird + thought bubble -->
					<div
						class="relative cursor-pointer"
						onmouseenter={onEnter}
						onmouseleave={onLeave}
						role="img"
						aria-label="Jolly"
					>
						<!-- Talk bubble (always visible above bird) -->
						<div
							style="
							position: absolute;
							bottom: 100%;
							left: 50%;
							transform: translate(calc(-50% + {TALK_OFFSET_X}px), {TALK_OFFSET_Y}px) scale({TALK_SCALE});
							transform-origin: bottom center;
							pointer-events: none;
						"
						>
							<img
								src="/jolly_talk.svg"
								alt=""
								aria-hidden="true"
								style="display: block; max-width: none;"
							/>
							<span
								style="
								position: absolute;
								top: 50%;
								left: 50%;
								transform: translate(calc(-50% + {TALK_TEXT_OFFSET_X}px), calc(-50% + {TALK_TEXT_OFFSET_Y}px));
								font-size: {TALK_TEXT_SIZE}px;
								font-weight: bold;
								color: #241e4e;
								white-space: pre-line;
								text-align: center;
							">{TALK_TEXT}</span
							>
						</div>

						<!-- Thinking bubble (clipboard correction) -->
						<ThinkingBubble
							active={correcting}
							minScale={0.9}
							maxScale={1.15}
							scaleSpeed={2}
							onTalk={onBubbleTalk}
							onThink={onBubbleThink}
						/>

						<!-- Thought bubble (on hover) -->
						{#if showBubble && !correcting}
							<div
								class="bubble absolute bottom-full left-1/2 mb-2 -translate-x-1/2 rounded-xl border-2 border-[#241e4e] bg-white px-4 py-3 text-center text-sm font-medium whitespace-nowrap text-[#241e4e] shadow-sm"
							>
								{thoughts[thoughtIdx]}
								<div
									class="absolute -bottom-[9px] left-1/2 flex -translate-x-1/2 flex-col items-center gap-[3px]"
								>
									<div class="h-2 w-2 rounded-full border-2 border-[#241e4e] bg-white"></div>
									<div class="h-1.5 w-1.5 rounded-full border-2 border-[#241e4e] bg-white"></div>
								</div>
							</div>
						{/if}

						{#key jollyImg}
							<img
								src={jollyImg}
								alt="Jolly"
								class="h-44"
								in:fade={{ duration: 200 }}
								out:fade={{ duration: 200 }}
							/>
						{/key}
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

<style>
	.bubble {
		animation: pop 0.15s ease-out;
	}

	@keyframes pop {
		from {
			transform: translateX(-50%) scale(0.85);
			opacity: 0;
		}
		to {
			transform: translateX(-50%) scale(1);
			opacity: 1;
		}
	}
</style>
