<script lang="ts">
	import BirdScene from '$lib/components/BirdScene.svelte';

	async function webCorrect(text: string): Promise<string> {
		const res = await fetch('/api/correct', {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify({ text })
		});
		if (!res.ok) throw new Error(`API error ${res.status}`);
		const { correctedText } = await res.json();
		return correctedText;
	}
</script>

<div class="mx-auto flex max-w-4xl flex-col px-6" style="min-height: calc(100svh - 130px);">
	<!-- Hero: fills available height, branch scene centered vertically -->
	<div class="grid flex-1 grid-cols-2 items-center gap-16">
		<!-- Left: copy -->
		<div>
			<p class="mb-3 text-xs font-semibold tracking-widest text-[#960200] uppercase">
				Your spell check parrot
			</p>
			<h1 class="mb-6 text-4xl leading-tight font-extrabold text-[#241e4e]">
				Catch typos.<br />Keep your voice.
			</h1>
			<p class="mb-10 leading-relaxed text-gray-500">
				Jolly repleats what your saing just like a real Parrot. Unlike a real Parrot he fixes your
				typos on the fly. No red squiggles. No extra clicking. Copy something hit enter and paste
				back in.
			</p>
			<a
				href="/download"
				class="inline-block rounded-lg border-4 border-[#241e4e] bg-[#960200] px-6 py-3 text-sm font-bold text-white transition-opacity hover:opacity-90"
			>
				Download Jolly — it's free
			</a>
		</div>

		<!-- Right: character scene -->
		<div class="mb-16 self-end">
			<BirdScene onCorrect={webCorrect} compact={false} />
		</div>
	</div>

	<p class="mb-8 text-xs text-gray-400">
		* The web demo uses a free public AI — do not paste sensitive or private data.
	</p>
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
				No notifications, no interruptions. Jolly sits there silently, waiting for your typo.
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
