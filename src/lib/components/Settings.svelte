<script lang="ts">
	import { settings } from '$lib/stores/settings.svelte';
	import type { ModelWithState } from '$lib/types/models';

	let { open = $bindable(false) } = $props();

	// API Key section
	let keyInput = $state('');
	let showKey = $state(false);
	let keySaved = $state(false);

	// Sync input when panel opens
	$effect(() => {
		if (open) {
			keyInput = settings.apiKey;
			keySaved = false;
		}
	});

	async function handleSaveKey() {
		await settings.saveApiKey(keyInput);
		keySaved = true;
		setTimeout(() => {
			keySaved = false;
		}, 2000);
	}

	// Model switch confirmation
	let switchedModelName = $state<string | null>(null);

	async function handleModelSwitch(modelId: string) {
		const model = settings.availableModels.find((m) => m.id === modelId);
		await settings.setActiveModel(modelId);
		if (model) {
			switchedModelName = model.name;
			setTimeout(() => {
				switchedModelName = null;
			}, 2000);
		}
	}

	// Download section
	let selectedModelId = $state('');

	let modelsForDownload = $derived(
		settings.availableModels.filter((m) => m.state !== 'downloaded')
	);

	let downloadedModels = $derived(settings.availableModels.filter((m) => m.state === 'downloaded'));

	// Auto-select first available model for download
	$effect(() => {
		if (modelsForDownload.length > 0 && !modelsForDownload.some((m) => m.id === selectedModelId)) {
			selectedModelId = modelsForDownload[0].id;
		}
	});

	function formatBytes(bytes: number): string {
		if (bytes >= 1_000_000_000) return (bytes / 1_000_000_000).toFixed(1) + ' GB';
		if (bytes >= 1_000_000) return (bytes / 1_000_000).toFixed(0) + ' MB';
		return (bytes / 1_000).toFixed(0) + ' KB';
	}

	function close() {
		open = false;
	}
</script>

<svelte:window
	onkeydown={(e) => {
		if (e.key === 'Escape' && open) close();
	}}
/>

<!-- Backdrop -->
{#if open}
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div
		class="fixed inset-0 z-40 bg-black/30 transition-opacity"
		onclick={close}
		tabindex="-1"
	></div>
{/if}

<!-- Panel -->
<div
	class="fixed top-0 right-0 z-50 flex h-full w-[70%] max-w-[560px] flex-col bg-[#e8e8e3] shadow-xl transition-transform duration-200 ease-out dark:bg-[#423f37] {open
		? 'translate-x-0'
		: 'translate-x-full'}"
>
	<!-- Header -->
	<div
		class="flex items-center justify-between border-b border-gray-200 px-5 py-4 dark:border-white/10"
	>
		<h2 class="text-lg font-semibold text-[#423f37] dark:text-[#e8e8e3]">Settings</h2>
		<button
			onclick={close}
			class="rounded-md p-1 text-gray-400 hover:text-[#423f37] dark:text-[#e8e8e3]/60 dark:hover:text-[#e8e8e3]"
			aria-label="Close settings"
		>
			<svg
				xmlns="http://www.w3.org/2000/svg"
				width="20"
				height="20"
				viewBox="0 0 24 24"
				fill="none"
				stroke="currentColor"
				stroke-width="2"
				stroke-linecap="round"
				stroke-linejoin="round"
			>
				<line x1="18" y1="6" x2="6" y2="18" /><line x1="6" y1="6" x2="18" y2="18" />
			</svg>
		</button>
	</div>

	<!-- Scrollable content -->
	<div class="flex-1 space-y-6 overflow-y-auto px-5 py-4">
		<!-- Section 1: Theme -->
		<section>
			<div class="flex items-center justify-between">
				<h3 class="text-sm font-bold text-[#423f37] dark:text-[#e8e8e3]">Theme</h3>
				<button
					onclick={() => settings.setThemeMode(settings.isDark ? 'light' : 'dark')}
					class="text-gray-400 transition-colors hover:text-[#960200] dark:text-[#e8e8e3]/60 dark:hover:text-[#ffd046]"
					aria-label={settings.isDark ? 'Switch to light mode' : 'Switch to dark mode'}
				>
					{#if settings.isDark}
						<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
							<circle cx="12" cy="12" r="5" /><line x1="12" y1="1" x2="12" y2="3" /><line x1="12" y1="21" x2="12" y2="23" /><line x1="4.22" y1="4.22" x2="5.64" y2="5.64" /><line x1="18.36" y1="18.36" x2="19.78" y2="19.78" /><line x1="1" y1="12" x2="3" y2="12" /><line x1="21" y1="12" x2="23" y2="12" /><line x1="4.22" y1="19.78" x2="5.64" y2="18.36" /><line x1="18.36" y1="5.64" x2="19.78" y2="4.22" />
						</svg>
					{:else}
						<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
							<path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z" />
						</svg>
					{/if}
				</button>
			</div>
		</section>

		<!-- Section 2: Local AI Models — Download -->
		<section>
			<h3 class="text-sm font-bold text-[#423f37] dark:text-[#e8e8e3]">Local AI Models</h3>

			{#if modelsForDownload.length > 0}
				<div class="mt-2 flex gap-2">
					<select
						bind:value={selectedModelId}
						class="flex-1 rounded-md border border-gray-300 bg-white px-3 py-2 text-sm text-[#423f37] dark:border-white/20 dark:bg-white/10 dark:text-[#e8e8e3]"
						disabled={!!settings.downloadProgress}
					>
						{#each modelsForDownload as model}
							<option value={model.id}>
								{model.name} — {formatBytes(model.sizeBytes)}
								{#if model.state === 'partial'}(resumable){/if}
							</option>
						{/each}
					</select>

					{#if settings.downloadProgress}
						<button
							onclick={() => settings.cancelDownload()}
							class="rounded-md border border-red-300 px-4 py-2 text-sm font-medium text-red-500 transition-colors hover:bg-red-50 dark:border-red-400 dark:text-red-400 dark:hover:bg-red-500/20"
						>
							Cancel
						</button>
					{:else}
						{@const selected = modelsForDownload.find((m) => m.id === selectedModelId)}
						<button
							onclick={() => settings.startDownload(selectedModelId)}
							class="rounded-md border-2 border-[#960200] bg-transparent px-4 py-2 text-sm font-medium text-[#423f37] transition-colors hover:bg-[#ffd046] hover:text-[#960200] disabled:cursor-not-allowed disabled:opacity-50 dark:border-[#ffd046] dark:text-[#e8e8e3] dark:hover:bg-[#960200] dark:hover:text-[#ffd046]"
						>
							{selected?.state === 'partial' ? 'Resume' : 'Download'}
						</button>
					{/if}
				</div>
			{:else}
				<p class="mt-2 text-xs text-gray-400 dark:text-[#e8e8e3]/50">All models downloaded</p>
			{/if}

			<!-- Download Progress -->
			{#if settings.downloadProgress}
				{@const p = settings.downloadProgress}
				{@const percent = p.totalBytes > 0 ? Math.round((p.bytesReceived / p.totalBytes) * 100) : 0}
				{@const model = settings.availableModels.find((m) => m.id === p.modelId)}
				<div class="mt-3">
					<div class="mb-1 flex justify-between text-xs text-gray-500 dark:text-[#e8e8e3]/50">
						<span>{model?.name ?? 'Unknown'}</span>
						<span>{formatBytes(p.bytesReceived)} / {formatBytes(p.totalBytes)} ({percent}%)</span>
					</div>
					<div class="h-2 w-full rounded-full bg-gray-200 dark:bg-white/10">
						<div
							class="h-full rounded-full bg-green-500 transition-all duration-200"
							style="width: {percent}%"
						></div>
					</div>
				</div>
			{/if}

			<!-- Download Error -->
			{#if settings.downloadError}
				<p class="mt-2 text-xs text-red-500 dark:text-red-400">{settings.downloadError}</p>
			{/if}
		</section>

		<!-- Section 3: Downloaded Models -->
		<section>
			<h3 class="text-sm font-bold text-[#423f37] dark:text-[#e8e8e3]">Downloaded Models</h3>
			{#if downloadedModels.length === 0}
				<p class="mt-2 text-xs text-gray-400 dark:text-[#e8e8e3]/50">No models downloaded yet</p>
			{:else if !settings.useOpenRouter && !settings.useHarper}
				<div class="mt-2 space-y-1">
					{#each downloadedModels as model}
						<div
							class="flex items-center gap-2 rounded-md px-3 py-2 transition-colors hover:bg-gray-50 dark:hover:bg-white/5"
						>
							<label class="flex flex-1 cursor-pointer items-center gap-3">
								<input
									type="radio"
									name="active-model"
									value={model.id}
									checked={settings.activeModelId === model.id}
									onchange={() => handleModelSwitch(model.id)}
									class="h-4 w-4 accent-[#960200] dark:accent-[#ffd046]"
								/>
								<span class="text-sm text-[#423f37] dark:text-[#e8e8e3]">
									<span class="font-medium">{model.name}</span>
									<span class="text-gray-400 dark:text-[#e8e8e3]/40">
										— {formatBytes(model.sizeBytes)}
									</span>
								</span>
							</label>
							<button
								onclick={() => settings.deleteModel(model.id)}
								class="rounded p-1 text-gray-300 transition-colors hover:text-red-500 dark:text-[#e8e8e3]/30 dark:hover:text-red-400"
								aria-label="Delete {model.name}"
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
									<polyline points="3 6 5 6 21 6" />
									<path
										d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"
									/>
								</svg>
							</button>
						</div>
					{/each}
				</div>
				<p class="mt-1 text-xs text-gray-400 dark:text-[#e8e8e3]/50">
					Restart app after downloading a new model to activate it.
				</p>
			{:else}
				<p class="mt-2 text-xs text-gray-400 dark:text-[#e8e8e3]/50">
					Disabled while {settings.useHarper ? 'Harper' : 'OpenRouter'} is active
				</p>
			{/if}
			{#if switchedModelName}
				<p class="mt-2 text-xs font-medium text-green-600 dark:text-green-400">
					Switched to {switchedModelName}
				</p>
			{/if}
		</section>

		<!-- Section: Harper (Lightweight) -->
		<section>
			<div class="flex items-center justify-between">
				<h3 class="text-sm font-bold text-[#423f37] dark:text-[#e8e8e3]">
					Harper (Lightweight)
				</h3>
				<button
					onclick={() => settings.setUseHarper(!settings.useHarper)}
					class="relative inline-flex h-6 w-11 items-center rounded-full transition-colors {settings.useHarper
						? 'bg-[#960200] dark:bg-[#ffd046]'
						: 'bg-gray-300 dark:bg-white/20'}"
					role="switch"
					aria-checked={settings.useHarper}
					aria-label="Toggle Harper"
				>
					<span
						class="inline-block h-4 w-4 rounded-full bg-white transition-transform {settings.useHarper
							? 'translate-x-6'
							: 'translate-x-1'}"
					></span>
				</button>
			</div>
			<p class="mt-1 text-xs text-gray-400 dark:text-[#e8e8e3]/50">
				Fast grammar & spelling correction. No downloads, no API key — works instantly. English
				only.
			</p>
		</section>

		<!-- Section 4: OpenRouter -->
		<section>
			<div class="flex items-center justify-between">
				<h3 class="text-sm font-bold text-[#423f37] dark:text-[#e8e8e3]">OpenRouter</h3>
				<button
					onclick={() => settings.setUseOpenRouter(!settings.useOpenRouter)}
					class="relative inline-flex h-6 w-11 items-center rounded-full transition-colors {settings.useOpenRouter
						? 'bg-[#960200] dark:bg-[#ffd046]'
						: 'bg-gray-300 dark:bg-white/20'}"
					role="switch"
					aria-checked={settings.useOpenRouter}
					aria-label="Toggle OpenRouter"
				>
					<span
						class="inline-block h-4 w-4 rounded-full bg-white transition-transform {settings.useOpenRouter
							? 'translate-x-6'
							: 'translate-x-1'}"
					></span>
				</button>
			</div>
			<p class="mt-1 text-xs text-gray-400 dark:text-[#e8e8e3]/50">
				Jolly is slow? This might be limited resources on your machine. Consider running Jolly
				through
				<a
					href="https://openrouter.ai"
					target="_blank"
					rel="noopener noreferrer"
					class="text-[#960200] underline dark:text-[#ffd046]">OpenRouter</a
				>.
			</p>
			{#if settings.useOpenRouter}
				<div class="mt-3">
					<label
						for="api-key-input"
						class="block text-sm font-medium text-[#423f37] dark:text-[#e8e8e3]"
					>
						API Key
					</label>
					<div class="mt-1 flex gap-2">
						<div class="relative flex-1">
							<input
								id="api-key-input"
								type={showKey ? 'text' : 'password'}
								bind:value={keyInput}
								placeholder="sk-or-..."
								class="w-full rounded-md border border-gray-300 bg-white px-3 py-2 pr-10 text-sm text-[#423f37] focus:border-[#960200] focus:ring-1 focus:ring-[#960200] focus:outline-none dark:border-white/20 dark:bg-white/10 dark:text-[#e8e8e3] dark:focus:border-[#ffd046] dark:focus:ring-[#ffd046]"
							/>
							<button
								onclick={() => {
									showKey = !showKey;
								}}
								class="absolute top-1/2 right-2 -translate-y-1/2 text-gray-400 hover:text-[#423f37] dark:text-[#e8e8e3]/40 dark:hover:text-[#e8e8e3]"
								aria-label={showKey ? 'Hide key' : 'Show key'}
								type="button"
							>
								{#if showKey}
									<svg
										xmlns="http://www.w3.org/2000/svg"
										width="16"
										height="16"
										viewBox="0 0 24 24"
										fill="none"
										stroke="currentColor"
										stroke-width="2"
										stroke-linecap="round"
										stroke-linejoin="round"
										><path
											d="M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94"
										/><path
											d="M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19"
										/><line x1="1" y1="1" x2="23" y2="23" /></svg
									>
								{:else}
									<svg
										xmlns="http://www.w3.org/2000/svg"
										width="16"
										height="16"
										viewBox="0 0 24 24"
										fill="none"
										stroke="currentColor"
										stroke-width="2"
										stroke-linecap="round"
										stroke-linejoin="round"
										><path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z" /><circle
											cx="12"
											cy="12"
											r="3"
										/></svg
									>
								{/if}
							</button>
						</div>
						<button
							onclick={handleSaveKey}
							class="rounded-md border-2 border-[#960200] bg-transparent px-4 py-2 text-sm font-medium text-[#423f37] transition-colors hover:bg-[#ffd046] hover:text-[#960200] dark:border-[#ffd046] dark:text-[#e8e8e3] dark:hover:bg-[#960200] dark:hover:text-[#ffd046]"
						>
							{keySaved ? 'Saved!' : 'Save'}
						</button>
					</div>
				</div>
			{/if}
		</section>
	</div>
</div>
