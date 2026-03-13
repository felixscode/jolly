<script lang="ts">
	import './layout.css';
	import Navbar from '$lib/components/Navbar.svelte';
	import { page } from '$app/state';
	import { settings } from '$lib/stores/settings';
	import { onMount } from 'svelte';

	let { children } = $props();

	let isApp = $derived(page.url.pathname.startsWith('/app'));

	onMount(() => {
		if (!isApp) return;

		// Load settings from Tauri storage and start listening to OS color scheme
		settings.loadAll();
		return settings.initSystemDarkListener();
	});

	$effect(() => {
		if (!isApp) return;
		if (settings.isDark) {
			document.documentElement.classList.add('dark');
		} else {
			document.documentElement.classList.remove('dark');
		}
	});
</script>

<svelte:head>
	<link rel="icon" href="/jolly_normal.svg" />
</svelte:head>

{#if isApp}
	<div class="relative h-screen overflow-hidden bg-white dark:bg-gray-900">
		{@render children()}
	</div>
{:else}
	<div class="flex min-h-screen flex-col">
		<Navbar />
		<main class="flex-grow">
			{@render children()}
		</main>
		<footer class="mt-auto border-t border-gray-100 py-8">
			<div class="mx-auto max-w-4xl px-6 text-center">
				<p class="text-sm text-gray-400">
					&copy; 2025 Jolly — made by Felix Schelling. Free and open source.
				</p>
			</div>
		</footer>
	</div>
{/if}
