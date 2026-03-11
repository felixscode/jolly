<script lang="ts">
	let linuxOpen = $state(false);
</script>

<svelte:window on:click={() => (linuxOpen = false)} />

<div class="mx-auto max-w-4xl px-6 pt-16 pb-24">

	<!-- Header -->
	<div class="mb-12 text-center">
		<h1 class="text-[5rem] font-extrabold leading-none tracking-tight text-[#241e4e]">
			Download
		</h1>
		<p class="mt-4 text-lg text-gray-500">Pick your platform. It's free.</p>
	</div>

	<hr class="mb-12 border-gray-200" />

	<!-- Platform grid -->
	<div class="grid grid-cols-3 gap-12">

		<div class="flex flex-col items-center text-center">
			<h2 class="mb-3 text-2xl font-bold text-[#241e4e]">Windows</h2>
			<p class="mb-8 text-sm leading-relaxed text-gray-500">
				Windows 10 or later. 64-bit.
			</p>
			<a href="#" class="mt-auto inline-block rounded-lg border-4 border-[#241e4e] bg-[#960200] px-5 py-2.5 text-sm font-bold text-white transition-opacity hover:opacity-90">
				Download .exe
			</a>
		</div>

		<div class="flex flex-col items-center text-center">
			<h2 class="mb-3 text-2xl font-bold text-[#241e4e]">macOS</h2>
			<p class="mb-8 text-sm leading-relaxed text-gray-500">
				macOS 12 or later. Apple Silicon &amp; Intel.
			</p>
			<a href="#" class="mt-auto inline-block rounded-lg border-4 border-[#241e4e] bg-[#960200] px-5 py-2.5 text-sm font-bold text-white transition-opacity hover:opacity-90">
				Download .dmg
			</a>
		</div>

		<div class="flex flex-col items-center text-center">
			<h2 class="mb-3 text-2xl font-bold text-[#241e4e]">Linux</h2>
			<p class="mb-8 text-sm leading-relaxed text-gray-500">
				AppImage, .deb, and .rpm available.
			</p>
			<div class="relative mt-auto">
				<button
					class="inline-flex items-center gap-2 rounded-lg border-4 border-[#241e4e] bg-[#960200] px-5 py-2.5 text-sm font-bold text-white transition-opacity hover:opacity-90"
					onclick={(e) => { e.stopPropagation(); linuxOpen = !linuxOpen; }}
				>
					Download
					<svg
						class="h-3 w-3 transition-transform {linuxOpen ? 'rotate-180' : ''}"
						viewBox="0 0 10 6"
						fill="none"
						xmlns="http://www.w3.org/2000/svg"
					>
						<path d="M1 1l4 4 4-4" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
					</svg>
				</button>

				{#if linuxOpen}
					<div class="absolute bottom-full left-1/2 mb-2 -translate-x-1/2 overflow-hidden rounded-lg border-2 border-[#241e4e] bg-white shadow-md">
						{#each ['.AppImage', '.deb', '.rpm'] as fmt}
							<a
								href="#"
								class="block px-5 py-2.5 text-sm font-medium text-[#241e4e] whitespace-nowrap hover:bg-[#241e4e] hover:text-white"
								onclick={(e) => e.stopPropagation()}
							>
								Download {fmt}
							</a>
						{/each}
					</div>
				{/if}
			</div>
		</div>

	</div>

	<hr class="mt-12 mb-12 border-gray-200" />

	<div class="mx-auto max-w-2xl text-center">
		<p class="text-sm leading-relaxed text-gray-400">
			All releases are open source and available on <a href="https://github.com" class="font-medium text-[#241e4e] transition-colors hover:text-[#960200]">GitHub</a>. If your platform isn't listed, build from source — it's straightforward.
		</p>
	</div>

</div>
