<script lang="ts">
	import { onMount } from 'svelte';

	let flyFrame = $state<1 | 2>(1);
	let flying = $state(true);
	let flyingIn = $state(true);
	let blinking = $state(false);

	onMount(() => {
		let mounted = true;

		// Alternate fly frames
		const flyInterval = setInterval(() => {
			flyFrame = flyFrame === 1 ? 2 : 1;
		}, 150);

		// Kick off fly-in transition
		setTimeout(() => { flyingIn = false; }, 50);

		// Land after flight
		setTimeout(() => {
			if (!mounted) return;
			clearInterval(flyInterval);
			flying = false;
		}, 2300);

		// Blink loop
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

<div class="mx-auto max-w-4xl px-4 pt-8 pb-12 md:px-6 md:pt-16 md:pb-24">
	<!-- Header: Jolly flies in horizontally from the right -->
	<div class="mb-8 flex items-center justify-center md:mb-16">
		<div
			class="relative h-28 w-24"
			style="
				transform: {flyingIn ? 'translateX(600px) scale(0.75)' : 'translateX(0) scale(1)'};
				opacity: {flyingIn ? 0 : 1};
				transition: transform 1800ms cubic-bezier(0.3, 0.3, 0.8, 0.9), opacity 500ms ease-out;
			"
		>
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

	<hr class="mb-8 border-gray-200 md:mb-16 dark:border-gray-700" />

	<!-- 4-section grid -->
	<div class="grid grid-cols-1 gap-8 md:grid-cols-2 md:gap-16">
		<div>
			<h2 class="mb-3 text-2xl font-bold text-[#423f37] dark:text-[#e8e8e3]">How it works</h2>
			<p class="leading-relaxed text-gray-500 dark:text-gray-400">
				Jolly runs locally on your PC. If you feel like a text needs spell checking, just copy it
				and hit Enter. Jolly reads your clipboard, passes it through a local LLM, and pastes the
				corrected text back.
			</p>
		</div>

		<div>
			<h2 class="mb-3 text-2xl font-bold text-[#423f37] dark:text-[#e8e8e3]">Why it exists</h2>
			<p class="leading-relaxed text-gray-500 dark:text-gray-400">
				Spell checkers are annoying — squiggly lines and too much clicking. Nowadays I tend to just
				paste text into an AI with the prompt "fix spelling." A lot of times I feel uneasy about
				my mails and notes being collected by LLM providers. So why not do it locally and make it fun?
			</p>
		</div>

		<div>
			<h2 class="mb-3 text-2xl font-bold text-[#423f37] dark:text-[#e8e8e3]">Under the hood</h2>
			<p class="leading-relaxed text-gray-500 dark:text-gray-400">
				Built with <a href="https://github.com/sveltejs/kit" target="_blank" rel="noopener noreferrer" class="underline hover:text-[#960200] dark:hover:text-[#ffd046]">SvelteKit</a>, Tailwind, <a href="https://github.com/tauri-apps/tauri" target="_blank" rel="noopener noreferrer" class="underline hover:text-[#960200] dark:hover:text-[#ffd046]">Tauri</a>, and <a href="https://github.com/EricLBuehler/mistral.rs" target="_blank" rel="noopener noreferrer" class="underline hover:text-[#960200] dark:hover:text-[#ffd046]">mistral.rs</a> — in Rust and TypeScript. You can select
				and download your favourite model in the app. If your machine doesn't allow local AI inference,
				you can also use your API keys. Everything runs locally — nothing leaves your machine.
			</p>
		</div>

		<div>
			<h2 class="mb-3 text-2xl font-bold text-[#423f37] dark:text-[#e8e8e3]">Open source</h2>
			<p class="leading-relaxed text-gray-500 dark:text-gray-400">
				Free to download, free to use. No account, no subscription. Read the source if you're
				curious.
			</p>
		</div>
	</div>

	<hr class="mt-8 mb-8 border-gray-200 md:mt-16 md:mb-16 dark:border-gray-700" />

	<!-- Personal note -->
	<div class="mx-auto max-w-2xl text-center">
		<h2 class="mb-4 text-2xl font-bold text-[#423f37] dark:text-[#e8e8e3]">Made to learn</h2>
		<p class="mb-4 leading-relaxed text-gray-500 dark:text-gray-400">
			Jolly started as a way to get to grips with frontend development — routing, components,
			styling, all of it. SvelteKit and Tailwind made that a lot less painful than expected.
		</p>
		<p class="leading-relaxed text-gray-500 dark:text-gray-400">
			The source is open. If something looks wrong or could be better, pull requests are welcome.
		</p>
	</div>
</div>
