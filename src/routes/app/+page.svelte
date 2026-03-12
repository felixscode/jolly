<script lang="ts">
	import BirdScene from '$lib/components/BirdScene.svelte';

	let hasCorrepted = $state(false);

	async function tauriCorrect(text: string): Promise<string> {
		const { invoke } = await import('@tauri-apps/api/core');
		const result = await invoke<string>('correct_text', { text });
		hasCorrepted = true;
		return result;
	}
</script>

<!-- Header bar -->
<div class="flex items-center justify-between px-4 pt-3 pb-1">
	<img src="/jolly_heading.svg" alt="Jolly" class="h-8" />
	<button
		class="rounded-md p-1.5 text-gray-400 transition-colors hover:text-[#241e4e]"
		aria-label="Settings"
		disabled
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

<!-- Bird scene (centered, fills remaining space) -->
<div class="flex flex-1 items-center justify-center">
	<BirdScene onCorrect={tauriCorrect} compact={true} />
</div>

<!-- Hint text -->
{#if !hasCorrepted}
	<p class="pb-4 text-center text-xs text-gray-400">press Enter to fix your clipboard</p>
{/if}
