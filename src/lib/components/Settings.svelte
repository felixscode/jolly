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

	let downloadedModels = $derived(
		settings.availableModels.filter((m) => m.state === 'downloaded')
	);

	// Auto-select first available model for download
	$effect(() => {
		if (
			modelsForDownload.length > 0 &&
			!modelsForDownload.some((m) => m.id === selectedModelId)
		) {
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
	class="fixed top-0 right-0 z-50 flex h-full w-[70%] max-w-[560px] flex-col bg-white shadow-xl transition-transform duration-200 ease-out dark:bg-gray-800 {open
		? 'translate-x-0'
		: 'translate-x-full'}"
>
	<!-- Header -->
	<div
		class="flex items-center justify-between border-b border-gray-200 px-5 py-4 dark:border-gray-700"
	>
		<h2 class="text-lg font-semibold text-gray-900 dark:text-gray-100">Settings</h2>
		<button
			onclick={close}
			class="rounded-md p-1 text-gray-400 hover:text-gray-600 dark:hover:text-gray-200"
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
			<h3 class="text-sm font-bold text-gray-900 dark:text-gray-100">Theme</h3>
			<div class="mt-2 inline-flex rounded-md border border-gray-300 dark:border-gray-600">
				{#each [{ value: 'system', label: 'System' }, { value: 'light', label: 'Light' }, { value: 'dark', label: 'Dark' }] as opt}
					<button
						onclick={() => settings.setThemeMode(opt.value as 'system' | 'light' | 'dark')}
						class="px-4 py-1.5 text-sm font-medium transition-colors first:rounded-l-md last:rounded-r-md {settings.themeMode ===
						opt.value
							? 'bg-[#241e4e] text-white'
							: 'text-gray-600 hover:bg-gray-100 dark:text-gray-300 dark:hover:bg-gray-700'}"
					>
						{opt.label}
					</button>
				{/each}
			</div>
		</section>

		<!-- Section 2: Local AI Models — Download -->
		<section>
			<h3 class="text-sm font-bold text-gray-900 dark:text-gray-100">Local AI Models</h3>

			{#if modelsForDownload.length > 0}
				<div class="mt-2 flex gap-2">
					<select
						bind:value={selectedModelId}
						class="flex-1 rounded-md border border-gray-300 bg-white px-3 py-2 text-sm dark:border-gray-600 dark:bg-gray-700 dark:text-gray-100"
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
							class="rounded-md border border-red-300 px-4 py-2 text-sm font-medium text-red-600 transition-colors hover:bg-red-50 dark:border-red-700 dark:text-red-400 dark:hover:bg-red-900/20"
						>
							Cancel
						</button>
					{:else}
						{@const selected = modelsForDownload.find((m) => m.id === selectedModelId)}
						<button
							onclick={() => settings.startDownload(selectedModelId)}
							class="rounded-md bg-[#241e4e] px-4 py-2 text-sm font-medium text-white transition-colors hover:bg-[#1a1639] disabled:cursor-not-allowed disabled:opacity-50"
						>
							{selected?.state === 'partial' ? 'Resume' : 'Download'}
						</button>
					{/if}
				</div>
			{:else}
				<p class="mt-2 text-xs text-gray-400 dark:text-gray-500">All models downloaded</p>
			{/if}

			<!-- Download Progress -->
			{#if settings.downloadProgress}
				{@const p = settings.downloadProgress}
				{@const percent = p.totalBytes > 0 ? Math.round((p.bytesReceived / p.totalBytes) * 100) : 0}
				{@const model = settings.availableModels.find((m) => m.id === p.modelId)}
				<div class="mt-3">
					<div class="mb-1 flex justify-between text-xs text-gray-500 dark:text-gray-400">
						<span>{model?.name ?? 'Unknown'}</span>
						<span
							>{formatBytes(p.bytesReceived)} / {formatBytes(p.totalBytes)} ({percent}%)</span
						>
					</div>
					<div class="h-2 w-full rounded-full bg-gray-200 dark:bg-gray-700">
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
			<h3 class="text-sm font-bold text-gray-900 dark:text-gray-100">Downloaded Models</h3>
			{#if downloadedModels.length === 0}
				<p class="mt-2 text-xs text-gray-400 dark:text-gray-500">No models downloaded yet</p>
			{:else if !settings.useOpenRouter}
				<div class="mt-2 space-y-1">
					{#each downloadedModels as model}
						<div
							class="flex items-center gap-2 rounded-md px-3 py-2 transition-colors hover:bg-gray-50 dark:hover:bg-gray-700/50"
						>
							<label class="flex flex-1 cursor-pointer items-center gap-3">
								<input
									type="radio"
									name="active-model"
									value={model.id}
									checked={settings.activeModelId === model.id}
									onchange={() => handleModelSwitch(model.id)}
									class="h-4 w-4 accent-[#241e4e]"
								/>
								<span class="text-sm text-gray-700 dark:text-gray-300">
									<span class="font-medium">{model.name}</span>
									<span class="text-gray-400 dark:text-gray-500">
										— {formatBytes(model.sizeBytes)}
									</span>
								</span>
							</label>
							<button
								onclick={() => settings.deleteModel(model.id)}
								class="rounded p-1 text-gray-300 transition-colors hover:text-red-500 dark:text-gray-600 dark:hover:text-red-400"
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
				<p class="mt-1 text-xs text-gray-400 dark:text-gray-500">
					Restart app after downloading a new model to activate it.
				</p>
			{:else}
				<p class="mt-2 text-xs text-gray-400 dark:text-gray-500">
					Disabled while OpenRouter is active
				</p>
			{/if}
			{#if switchedModelName}
				<p class="mt-2 text-xs font-medium text-green-600 dark:text-green-400">
					Switched to {switchedModelName}
				</p>
			{/if}
		</section>

		<!-- Section 4: OpenRouter -->
		<section>
			<div class="flex items-center justify-between">
				<h3 class="text-sm font-bold text-gray-900 dark:text-gray-100">OpenRouter</h3>
				<button
					onclick={() => settings.setUseOpenRouter(!settings.useOpenRouter)}
					class="relative inline-flex h-6 w-11 items-center rounded-full transition-colors {settings.useOpenRouter
						? 'bg-[#241e4e]'
						: 'bg-gray-300 dark:bg-gray-600'}"
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
			<p class="mt-1 text-xs text-gray-400 dark:text-gray-500">
				Jolly is slow? This might be limited resources on your machine. Consider running Jolly
				through
				<a
					href="https://openrouter.ai"
					target="_blank"
					rel="noopener noreferrer"
					class="text-[#241e4e] underline dark:text-[#ffd046]">OpenRouter</a
				>.
			</p>
			{#if settings.useOpenRouter}
				<div class="mt-3">
					<label
						for="api-key-input"
						class="block text-sm font-medium text-gray-700 dark:text-gray-300"
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
								class="w-full rounded-md border border-gray-300 bg-white px-3 py-2 pr-10 text-sm focus:border-[#241e4e] focus:ring-1 focus:ring-[#241e4e] focus:outline-none dark:border-gray-600 dark:bg-gray-700 dark:text-gray-100"
							/>
							<button
								onclick={() => {
									showKey = !showKey;
								}}
								class="absolute top-1/2 right-2 -translate-y-1/2 text-gray-400 hover:text-gray-600 dark:hover:text-gray-200"
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
							class="rounded-md bg-[#241e4e] px-4 py-2 text-sm font-medium text-white transition-colors hover:bg-[#1a1639]"
						>
							{keySaved ? 'Saved!' : 'Save'}
						</button>
					</div>
				</div>
			{/if}
		</section>
	</div>
</div>
