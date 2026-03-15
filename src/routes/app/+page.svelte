<script lang="ts">
	import BirdScene from '$lib/components/BirdScene.svelte';
	import History from '$lib/components/History.svelte';
	import Settings from '$lib/components/Settings.svelte';
	import { settings } from '$lib/stores/settings.svelte';

	let hasCorrepted = $state(false);
	let settingsOpen = $state(false);
	let historyOpen = $state(false);

	async function tauriCorrect(text: string): Promise<string> {
		const { invoke } = await import('@tauri-apps/api/core');
		const result = await invoke<string>('correct_text', { text });
		hasCorrepted = true;
		await settings.addToHistory(result);
		return result;
	}
</script>

<!-- Bird scene (always dead-center of screen) -->
<div class="flex h-full items-center justify-center">
	<BirdScene onCorrect={tauriCorrect} />
</div>

<!-- Header bar (layered on top) -->
<div class="absolute inset-x-0 top-0 flex items-center justify-center px-4 pt-3 pb-1">
	<button
		class="absolute left-4 rounded-md p-1.5 text-gray-400 transition-colors hover:text-[#960200] dark:text-[#e8e8e3]/40 dark:hover:text-[#ffd046]"
		aria-label="History"
		onclick={() => {
			historyOpen = true;
		}}
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
			<path d="M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8" />
			<path d="M3 3v5h5" />
			<path d="M12 7v5l4 2" />
		</svg>
	</button>
	<img src="/jolly_heading.png" alt="Jolly" class="h-16" />
	<button
		class="absolute right-4 rounded-md p-1.5 text-gray-400 transition-colors hover:text-[#960200] dark:text-[#e8e8e3]/40 dark:hover:text-[#ffd046]"
		aria-label="Settings"
		onclick={() => {
			settingsOpen = true;
		}}
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
			<path
				d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z"
			/>
			<circle cx="12" cy="12" r="3" />
		</svg>
	</button>
</div>

<!-- Hint text (layered on bottom) -->
{#if !hasCorrepted}
	<p class="absolute inset-x-0 bottom-0 pb-4 text-center text-xs text-gray-400 dark:text-gray-500">
		press Enter to fix your clipboard
	</p>
{/if}

<!-- Panels -->
<History bind:open={historyOpen} />
<Settings bind:open={settingsOpen} />
