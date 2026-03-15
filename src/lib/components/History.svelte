<script lang="ts">
	import { settings } from '$lib/stores/settings.svelte';

	let { open = $bindable(false) } = $props();

	function close() {
		open = false;
	}

	async function copyToClipboard(text: string) {
		try {
			const { writeText } = await import('@tauri-apps/plugin-clipboard-manager');
			await writeText(text);
		} catch {
			await navigator.clipboard.writeText(text);
		}
	}
</script>

<!-- Backdrop -->
{#if open}
	<!-- svelte-ignore a11y_no_static_element_interactions, a11y_click_events_have_key_events -->
	<div
		class="fixed inset-0 z-40 bg-black/30 transition-opacity"
		onclick={close}
		tabindex="-1"
	></div>
{/if}

<!-- Panel -->
<div
	class="fixed top-0 left-0 z-50 flex h-full w-[70%] max-w-[560px] flex-col bg-white shadow-xl transition-transform duration-200 ease-out dark:bg-[#423f37] {open
		? 'translate-x-0'
		: '-translate-x-full'}"
>
	<!-- Header -->
	<div
		class="flex items-center justify-between border-b border-gray-200 px-5 py-4 dark:border-white/10"
	>
		<h2 class="text-lg font-semibold text-[#423f37] dark:text-[#e8e8e3]">History</h2>
		<div class="flex items-center gap-2">
			{#if settings.correctionHistory.length > 0}
				<button
					onclick={() => settings.clearHistory()}
					class="rounded-md px-2 py-1 text-xs text-gray-400 transition-colors hover:text-red-500 dark:text-[#e8e8e3]/40 dark:hover:text-red-400"
				>
					Clear
				</button>
			{/if}
			<button
				onclick={close}
				class="rounded-md p-1 text-gray-400 hover:text-[#423f37] dark:text-[#e8e8e3]/60 dark:hover:text-[#e8e8e3]"
				aria-label="Close history"
			>
				<svg
					xmlns="http://www.w3.org/2000/svg"
					width="18"
					height="18"
					viewBox="0 0 24 24"
					fill="none"
					stroke="currentColor"
					stroke-width="2"
					stroke-linecap="round"
					stroke-linejoin="round"
				>
					<path d="M18 6 6 18" />
					<path d="m6 6 12 12" />
				</svg>
			</button>
		</div>
	</div>

	<!-- List -->
	<div class="flex-1 overflow-y-auto px-4 py-3">
		{#if settings.correctionHistory.length === 0}
			<p class="py-8 text-center text-sm text-gray-400 dark:text-[#e8e8e3]/40">
				No corrections yet
			</p>
		{:else}
			<div class="flex flex-col gap-2">
				{#each settings.correctionHistory as text}
					<div
						class="group flex items-start gap-2 rounded-lg border border-gray-200 px-3 py-2.5 dark:border-white/10"
					>
						<p
							class="min-w-0 flex-1 text-sm leading-snug text-[#423f37] line-clamp-2 dark:text-[#e8e8e3]"
						>
							{text}
						</p>
						<button
							onclick={() => copyToClipboard(text)}
							class="mt-0.5 shrink-0 rounded p-1 text-gray-300 transition-colors hover:text-[#960200] dark:text-[#e8e8e3]/30 dark:hover:text-[#ffd046]"
							aria-label="Copy to clipboard"
						>
							<svg
								xmlns="http://www.w3.org/2000/svg"
								width="14"
								height="14"
								viewBox="0 0 24 24"
								fill="none"
								stroke="currentColor"
								stroke-width="2"
								stroke-linecap="round"
								stroke-linejoin="round"
							>
								<rect width="14" height="14" x="8" y="8" rx="2" ry="2" />
								<path d="M4 16c-1.1 0-2-.9-2-2V4c0-1.1.9-2 2-2h10c1.1 0 2 .9 2 2" />
							</svg>
						</button>
					</div>
				{/each}
			</div>
		{/if}
	</div>
</div>
