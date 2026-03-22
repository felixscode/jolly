<script lang="ts">
	import BirdScene from '$lib/components/BirdScene.svelte';
	import { onMount } from 'svelte';

	let isDark = $state(false);
	let correctFn = $state<(text: string) => Promise<string>>(async (t) => t);

	onMount(async () => {
		const { harperCorrect } = await import('$lib/harper-web');
		correctFn = harperCorrect;

		const observer = new MutationObserver(() => {
			isDark = document.documentElement.classList.contains('dark');
		});
		isDark = document.documentElement.classList.contains('dark');
		observer.observe(document.documentElement, { attributes: true, attributeFilter: ['class'] });
		return () => observer.disconnect();
	});
</script>

<div
	class="mx-auto flex max-w-4xl flex-col overflow-x-hidden px-4 md:px-6"
	style="min-height: calc(100svh - 116px - 117px);"
>
	<!-- Hero -->
	<div class="grid flex-1 grid-cols-1 items-center gap-6 md:grid-cols-2 md:gap-12">
		<!-- Left: copy -->
		<div class="text-center md:text-left">
			<p
				class="mb-2 text-xs font-semibold tracking-widest text-[#960200] uppercase dark:text-[#ffd046]"
			>
				Your spell check parrot
			</p>
			<h1
				class="mb-3 text-3xl leading-tight font-extrabold text-[#423f37] md:text-4xl dark:text-[#e8e8e3]"
			>
				Catch Typos<br /> Private and Fun
			</h1>
			<p class="mb-5 text-sm leading-relaxed text-gray-500 dark:text-gray-400">
				Jolly repeats what you're saying — just like a real parrot. Unlike a real parrot, he fixes
				your typos on the fly. No red squiggles. No extra clicking. Copy something, hit Enter, and
				paste back in.
			</p>
			<a
				href="/download"
				class="inline-block rounded-lg border-4 border-[#960200] bg-transparent px-6 py-3 text-sm font-bold text-[#423f37] transition-colors hover:bg-[#ffd046] hover:text-[#960200] dark:border-[#ffd046] dark:text-[#e8e8e3] dark:hover:bg-[#960200] dark:hover:text-[#ffd046]"
			>
				Download Jolly — it's free
			</a>
		</div>

		<!-- Right: character scene -->
		<div class="flex items-center justify-center overflow-visible pt-20 pb-24 md:pt-0 md:pb-0">
			<BirdScene onCorrect={correctFn} />
		</div>
	</div>

	<p class="relative z-10 py-2 text-xs text-gray-400 dark:text-gray-500">
		* This is a web demo using <a
			href="https://github.com/Automattic/harper"
			target="_blank"
			rel="noopener noreferrer"
			class="underline hover:text-[#960200] dark:hover:text-[#ffd046]">Harper</a
		> for English spelling and grammar. Its webassembly - nothing leaves your machine.
	</p>

	<hr class="border-gray-200 dark:border-gray-700" />

	<!-- Feature strip -->
	<div class="grid grid-cols-1 gap-6 py-4 sm:grid-cols-3 sm:gap-12 sm:py-6">
		<div class="text-center sm:text-left">
			<h2 class="mb-1 text-lg font-bold text-[#423f37] dark:text-[#e8e8e3]">Local</h2>
			<p class="text-sm leading-relaxed text-gray-500 dark:text-gray-400">
				Runs entirely on your machine. Nothing leaves your computer — no cloud, no account required.
			</p>
		</div>
		<div class="text-center sm:text-left">
			<h2 class="mb-1 text-lg font-bold text-[#423f37] dark:text-[#e8e8e3]">Quiet</h2>
			<p class="text-sm leading-relaxed text-gray-500 dark:text-gray-400">
				No notifications, no interruptions. Jolly sits there silently, waiting for your typo.
			</p>
		</div>
		<div class="text-center sm:text-left">
			<h2 class="mb-1 text-lg font-bold text-[#423f37] dark:text-[#e8e8e3]">Free</h2>
			<p class="text-sm leading-relaxed text-gray-500 dark:text-gray-400">
				Open source and free. Download it, use it, read the source code if you're curious.
			</p>
		</div>
	</div>
</div>
