<script lang="ts">
	import BirdScene from '$lib/components/BirdScene.svelte';
	import { onMount } from 'svelte';
	import { harperCorrect } from '$lib/harper-web';

	let isDark = $state(false);

	onMount(() => {
		const observer = new MutationObserver(() => {
			isDark = document.documentElement.classList.contains('dark');
		});
		isDark = document.documentElement.classList.contains('dark');
		observer.observe(document.documentElement, { attributes: true, attributeFilter: ['class'] });
		return () => observer.disconnect();
	});


</script>

<div
	class="mx-auto flex max-w-4xl flex-col overflow-x-hidden px-6"
	style="height: calc(100svh - 116px - 117px);"
>
	<!-- Hero -->
	<div class="grid flex-1 grid-cols-2 items-center gap-12">
		<!-- Left: copy -->
		<div>
			<p
				class="mb-2 text-xs font-semibold tracking-widest text-[#960200] uppercase dark:text-[#ffd046]"
			>
				Your spell check parrot
			</p>
			<h1 class="mb-3 text-4xl leading-tight font-extrabold text-[#423f37] dark:text-[#e8e8e3]">
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
		<div class="flex items-center justify-center overflow-visible">
			<BirdScene onCorrect={harperCorrect} />
		</div>
	</div>

	<p class="py-2 text-xs text-gray-400 dark:text-gray-500">
		* The web demo uses Harper for English spelling and grammar only.
	</p>

	<hr class="border-gray-200 dark:border-gray-700" />

	<!-- Feature strip -->
	<div class="grid grid-cols-3 gap-12 py-6">
		<div>
			<h2 class="mb-1 text-lg font-bold text-[#423f37] dark:text-[#e8e8e3]">Local</h2>
			<p class="text-sm leading-relaxed text-gray-500 dark:text-gray-400">
				Runs entirely on your machine. Nothing leaves your computer — no cloud, no account required.
			</p>
		</div>
		<div>
			<h2 class="mb-1 text-lg font-bold text-[#423f37] dark:text-[#e8e8e3]">Quiet</h2>
			<p class="text-sm leading-relaxed text-gray-500 dark:text-gray-400">
				No notifications, no interruptions. Jolly sits there silently, waiting for your typo.
			</p>
		</div>
		<div>
			<h2 class="mb-1 text-lg font-bold text-[#423f37] dark:text-[#e8e8e3]">Free</h2>
			<p class="text-sm leading-relaxed text-gray-500 dark:text-gray-400">
				Open source and free forever. Download it, use it, read the source code if you're curious.
			</p>
		</div>
	</div>
</div>
