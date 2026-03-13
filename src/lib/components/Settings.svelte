<script lang="ts">
	import { onDestroy } from 'svelte';
	import { settings } from '$lib/stores/settings.svelte';
	import { AVAILABLE_MODELS, type Model } from '$lib/types/models';

	let { open = $bindable(false) } = $props();
	let downloadInterval: ReturnType<typeof setInterval> | null = null;

	onDestroy(() => {
		if (downloadInterval) clearInterval(downloadInterval);
	});

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

	// Dark mode section — systemDark is centralized in the settings store
	let darkStatusText = $derived.by(() => {
		if (settings.darkModeOverride) return 'Dark (manual)';
		return settings.systemDark ? 'Dark (system)' : 'Light (system)';
	});

	// Model download section
	let selectedModelId = $state(AVAILABLE_MODELS[0].id);

	let availableForDownload = $derived(
		AVAILABLE_MODELS.filter((m) => !settings.downloadedModels.some((d) => d.id === m.id))
	);

	$effect(() => {
		if (
			availableForDownload.length > 0 &&
			!availableForDownload.some((m) => m.id === selectedModelId)
		) {
			selectedModelId = availableForDownload[0].id;
		}
	});

	async function handleDownload() {
		const model = AVAILABLE_MODELS.find((m) => m.id === selectedModelId);
		if (!model || settings.downloadProgress) return;

		settings.setDownloadProgress({ modelId: model.id, percent: 0 });

		// Fake download: increment over ~5 seconds
		downloadInterval = setInterval(() => {
			const current = settings.downloadProgress;
			if (!current) {
				clearInterval(downloadInterval!);
				downloadInterval = null;
				return;
			}
			const next = Math.min(current.percent + 2, 100);
			if (next >= 100) {
				clearInterval(downloadInterval!);
				downloadInterval = null;
				settings.setDownloadProgress(null);
				settings.addDownloadedModel(model);
			} else {
				settings.setDownloadProgress({ modelId: current.modelId, percent: next });
			}
		}, 100);
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
		<!-- Section 1: OpenRouter API Key -->
		<section>
			<label for="api-key-input" class="block text-sm font-medium text-gray-700 dark:text-gray-300">
				OpenRouter API Key
			</label>
			<div class="mt-2 flex gap-2">
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
		</section>

		<!-- Section 2: Dark Mode Toggle -->
		<section>
			<div class="flex items-center justify-between">
				<span class="text-sm font-medium text-gray-700 dark:text-gray-300">Dark Mode</span>
				<button
					onclick={() => settings.setDarkModeOverride(!settings.darkModeOverride)}
					class="relative inline-flex h-6 w-11 items-center rounded-full transition-colors {settings.darkModeOverride
						? 'bg-[#241e4e]'
						: 'bg-gray-300 dark:bg-gray-600'}"
					role="switch"
					aria-checked={settings.darkModeOverride}
					aria-label="Toggle dark mode"
				>
					<span
						class="inline-block h-4 w-4 rounded-full bg-white transition-transform {settings.darkModeOverride
							? 'translate-x-6'
							: 'translate-x-1'}"
					></span>
				</button>
			</div>
			<p class="mt-1 text-xs text-gray-400 dark:text-gray-500">{darkStatusText}</p>
		</section>

		<!-- Section 3: Local AI Models — Download -->
		<section>
			<h3 class="text-sm font-medium text-gray-700 dark:text-gray-300">Local AI Models</h3>
			{#if availableForDownload.length > 0}
				<div class="mt-2 flex gap-2">
					<select
						bind:value={selectedModelId}
						class="flex-1 rounded-md border border-gray-300 bg-white px-3 py-2 text-sm dark:border-gray-600 dark:bg-gray-700 dark:text-gray-100"
						disabled={!!settings.downloadProgress}
					>
						{#each availableForDownload as model}
							<option value={model.id}>{model.name} — {model.sizeGb} GB</option>
						{/each}
					</select>
					<button
						onclick={handleDownload}
						disabled={!!settings.downloadProgress}
						class="rounded-md bg-[#241e4e] px-4 py-2 text-sm font-medium text-white transition-colors hover:bg-[#1a1639] disabled:cursor-not-allowed disabled:opacity-50"
					>
						Download
					</button>
				</div>
			{:else}
				<p class="mt-2 text-xs text-gray-400 dark:text-gray-500">All models downloaded</p>
			{/if}

			<!-- Download Progress -->
			{#if settings.downloadProgress}
				{@const model = AVAILABLE_MODELS.find((m) => m.id === settings.downloadProgress?.modelId)}
				<div class="mt-3">
					<div class="mb-1 flex justify-between text-xs text-gray-500 dark:text-gray-400">
						<span>{model?.name ?? 'Unknown'}</span>
						<span>{settings.downloadProgress.percent}%</span>
					</div>
					<div class="h-2 w-full rounded-full bg-gray-200 dark:bg-gray-700">
						<div
							class="h-full rounded-full bg-green-500 transition-all duration-100"
							style="width: {settings.downloadProgress.percent}%"
						></div>
					</div>
				</div>
			{/if}
		</section>

		<!-- Section 4: Downloaded Models -->
		<section>
			<h3 class="text-sm font-medium text-gray-700 dark:text-gray-300">Downloaded Models</h3>
			{#if settings.downloadedModels.length === 0}
				<p class="mt-2 text-xs text-gray-400 dark:text-gray-500">No models downloaded yet</p>
			{:else}
				<div class="mt-2 space-y-2">
					{#each settings.downloadedModels as model}
						<button
							onclick={() =>
								settings.setActiveModel(settings.activeModelId === model.id ? null : model.id)}
							class="w-full rounded-md border px-3 py-2 text-left text-sm transition-colors
								{settings.activeModelId === model.id
								? 'border-[#241e4e] bg-[#241e4e]/5 text-[#241e4e] dark:border-[#ffd046] dark:bg-[#ffd046]/10 dark:text-[#ffd046]'
								: 'border-gray-200 text-gray-700 hover:border-gray-300 dark:border-gray-600 dark:text-gray-300 dark:hover:border-gray-500'}"
						>
							<span class="font-medium">{model.name}</span>
							<span class="text-gray-400 dark:text-gray-500"> — {model.sizeGb} GB</span>
							<span class="text-gray-400 dark:text-gray-500"> (~{model.inferenceMs}ms/token)</span>
						</button>
					{/each}
				</div>
			{/if}
		</section>
	</div>
</div>
