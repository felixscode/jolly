<script lang="ts">
	import './layout.css';
	import Navbar from '$lib/components/Navbar.svelte';
	import { page } from '$app/state';
	import { settings } from '$lib/stores/settings.svelte';
	import { onMount } from 'svelte';

	let { children } = $props();

	let isApp = $derived(page.url.pathname.startsWith('/app'));

	// Website dark mode state (separate from Tauri app settings)
	let websiteDark = $state(false);

	function toggleWebsiteDark() {
		websiteDark = !websiteDark;
		localStorage.setItem('jolly-theme', websiteDark ? 'dark' : 'light');
		if (websiteDark) {
			document.documentElement.classList.add('dark');
		} else {
			document.documentElement.classList.remove('dark');
		}
	}

	onMount(() => {
		if (isApp) {
			// Tauri app: load settings from store
			settings.loadAll();
			return settings.initSystemDarkListener();
		} else {
			// Website: read initial state from DOM (set by anti-flash script)
			websiteDark = document.documentElement.classList.contains('dark');
		}
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
	<div class="relative h-screen overflow-hidden bg-white dark:bg-[#0f0d1e]">
		{@render children()}
	</div>
{:else}
	<div class="flex min-h-screen flex-col bg-white dark:bg-[#0f0d1e]">
		<Navbar isDark={websiteDark} onToggleDark={toggleWebsiteDark} />
		<main class="flex-grow">
			{@render children()}
		</main>
		<footer class="mt-auto border-t border-gray-100 py-8 dark:border-gray-700">
			<div class="mx-auto max-w-4xl px-6 text-center">
				<p class="text-sm text-gray-400 dark:text-gray-500">
					&copy; 2025 Jolly — made by Felix Schelling. Free and open source.
				</p>
			</div>
		</footer>
	</div>
{/if}
