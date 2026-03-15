<script lang="ts">
	import { onMount } from 'svelte';

	let linuxOpen = $state(false);

	let flyFrame = $state<1 | 2>(1);
	let flying = $state(true);
	let flyingIn = $state(true);
	let blinking = $state(false);
	let showBubble = $state(false);
	let bubbleTimer: ReturnType<typeof setTimeout> | undefined;

	function triggerYeahh() {
		showBubble = true;
		clearTimeout(bubbleTimer);
		bubbleTimer = setTimeout(() => { showBubble = false; }, 3000);
	}

	onMount(() => {
		let mounted = true;

		const flyInterval = setInterval(() => {
			flyFrame = flyFrame === 1 ? 2 : 1;
		}, 150);

		setTimeout(() => { flyingIn = false; }, 50);

		setTimeout(() => {
			if (!mounted) return;
			clearInterval(flyInterval);
			flying = false;
		}, 2300);

		function scheduleBlink() {
			const delay = 2500 + Math.random() * 4000;
			setTimeout(() => {
				if (!mounted) return;
				if (!flying) {
					blinking = true;
					setTimeout(() => {
						if (!mounted) return;
						blinking = false;
						scheduleBlink();
					}, 160);
				} else {
					scheduleBlink();
				}
			}, delay);
		}
		setTimeout(scheduleBlink, 3300);

		return () => { mounted = false; clearInterval(flyInterval); };
	});

	const pose = $derived(
		flying
			? flyFrame === 1 ? 'fly1' : 'fly2'
			: blinking ? 'blink' : 'normal'
	);
</script>

<svelte:window on:click={() => (linuxOpen = false)} />

<div class="mx-auto flex max-w-4xl flex-col justify-center px-6" style="min-height: calc(100svh - 116px - 117px);">
	<!-- Header: Jolly drops in from top with speech bubble -->
	<div class="-mt-8 mb-12 flex items-center justify-center">
		<div
			class="relative h-28 w-24"
			style="
				transform: {flyingIn ? 'translateY(-500px) scale(0.75)' : 'translateY(0) scale(1)'};
				opacity: {flyingIn ? 0 : 1};
				transition: transform 1800ms cubic-bezier(0.3, 0.3, 0.8, 0.9), opacity 500ms ease-out;
			"
		>
			<!-- Speech bubble (offset right like on home) -->
			<div
				class="pointer-events-none absolute bottom-full"
				style="left: 50%; opacity: {showBubble ? 1 : 0}; transition: opacity 300ms ease; transform: translate(calc(-50% + 80px), -5px);"
			>
				<img
					src="/jolly_talk.svg"
					alt=""
					aria-hidden="true"
					style="display: block; width: 120px; max-width: none;"
				/>
				<div
					class="pointer-events-none absolute flex items-center justify-center text-center"
					style="top: 10px; left: 26px; width: 86px; height: 40px;"
				>
					<span class="text-xs font-bold text-[#423f37]">Thx!</span>
				</div>
			</div>

			<img
				src="/jolly_normal.svg"
				alt="Jolly"
				class="absolute inset-0 h-full w-full object-contain"
				style="opacity: {pose === 'normal' ? 1 : 0};"
			/>
			<img
				src="/jolly_bilnzel.svg"
				alt=""
				aria-hidden="true"
				class="absolute inset-0 h-full w-full object-contain"
				style="opacity: {pose === 'blink' ? 1 : 0};"
			/>
			<img
				src="/jolly_fly1.svg"
				alt=""
				aria-hidden="true"
				class="absolute inset-0 h-full w-full object-contain"
				style="opacity: {pose === 'fly1' ? 1 : 0};"
			/>
			<img
				src="/jolly_fly2.svg"
				alt=""
				aria-hidden="true"
				class="absolute inset-0 h-full w-full object-contain"
				style="opacity: {pose === 'fly2' ? 1 : 0};"
			/>
		</div>
	</div>

	<hr class="mb-12 border-gray-200 dark:border-gray-700" />

	<!-- Platform grid -->
	<div class="grid grid-cols-3 gap-12">
		<div class="flex flex-col items-center text-center">
			<h2 class="mb-3 text-2xl font-bold text-[#423f37] dark:text-[#e8e8e3]">Windows</h2>
			<p class="mb-8 text-sm leading-relaxed text-gray-500 dark:text-gray-400">
				Windows 10 or later. 64-bit.
			</p>
			<a
				href="#"
				onclick={triggerYeahh}
				class="mt-auto inline-block rounded-lg border-4 border-[#960200] bg-transparent px-5 py-2.5 text-sm font-bold text-[#423f37] transition-colors hover:bg-[#ffd046] hover:text-[#960200] dark:border-[#ffd046] dark:text-[#e8e8e3] dark:hover:bg-[#960200] dark:hover:text-[#ffd046]"
			>
				Download .exe
			</a>
		</div>

		<div class="flex flex-col items-center text-center">
			<h2 class="mb-3 text-2xl font-bold text-[#423f37] dark:text-[#e8e8e3]">macOS</h2>
			<p class="mb-8 text-sm leading-relaxed text-gray-500 dark:text-gray-400">
				macOS 12 or later. Apple Silicon &amp; Intel.
			</p>
			<a
				href="#"
				onclick={triggerYeahh}
				class="mt-auto inline-block rounded-lg border-4 border-[#960200] bg-transparent px-5 py-2.5 text-sm font-bold text-[#423f37] transition-colors hover:bg-[#ffd046] hover:text-[#960200] dark:border-[#ffd046] dark:text-[#e8e8e3] dark:hover:bg-[#960200] dark:hover:text-[#ffd046]"
			>
				Download .dmg
			</a>
		</div>

		<div class="flex flex-col items-center text-center">
			<h2 class="mb-3 text-2xl font-bold text-[#423f37] dark:text-[#e8e8e3]">Linux</h2>
			<p class="mb-8 text-sm leading-relaxed text-gray-500 dark:text-gray-400">
				AppImage, .deb, and .rpm available.
			</p>
			<div class="relative mt-auto">
				<button
					class="inline-flex items-center gap-2 rounded-lg border-4 border-[#960200] bg-transparent px-5 py-2.5 text-sm font-bold text-[#423f37] transition-colors hover:bg-[#ffd046] hover:text-[#960200] dark:border-[#ffd046] dark:text-[#e8e8e3] dark:hover:bg-[#960200] dark:hover:text-[#ffd046]"
					onclick={(e) => {
						e.stopPropagation();
						linuxOpen = !linuxOpen;
					}}
				>
					Download
					<svg
						class="h-3 w-3 transition-transform {linuxOpen ? 'rotate-180' : ''}"
						viewBox="0 0 10 6"
						fill="none"
						xmlns="http://www.w3.org/2000/svg"
					>
						<path
							d="M1 1l4 4 4-4"
							stroke="currentColor"
							stroke-width="1.5"
							stroke-linecap="round"
							stroke-linejoin="round"
						/>
					</svg>
				</button>

				{#if linuxOpen}
					<div
						class="absolute bottom-full left-1/2 mb-2 -translate-x-1/2 overflow-hidden rounded-lg border-2 border-[#423f37] bg-[#e8e8e3] shadow-md dark:border-[#e8e8e3] dark:bg-[#2b2a2a]"
					>
						{#each ['.AppImage', '.deb', '.rpm'] as fmt}
							<a
								href="#"
								class="block px-5 py-2.5 text-sm font-medium whitespace-nowrap text-[#423f37] hover:bg-[#423f37] hover:text-white dark:text-[#e8e8e3] dark:hover:bg-[#e8e8e3] dark:hover:text-[#2b2a2a]"
								onclick={(e) => { e.stopPropagation(); triggerYeahh(); }}
							>
								Download {fmt}
							</a>
						{/each}
					</div>
				{/if}
			</div>
		</div>
	</div>

	<hr class="mt-12 mb-12 border-gray-200 dark:border-gray-700" />

	<div class="mx-auto max-w-2xl text-center">
		<p class="text-sm leading-relaxed text-gray-400 dark:text-gray-500">
			All releases are open source and available on <a
				href="https://github.com/felixscode/jolly"
				class="font-medium text-[#423f37] transition-colors hover:text-[#960200] dark:text-[#e8e8e3] dark:hover:text-[#ffd046]"
				>GitHub</a
			>. If your platform isn't listed, build from source — it's straightforward.
		</p>
	</div>
</div>
