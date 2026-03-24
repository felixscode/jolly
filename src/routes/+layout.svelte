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
	<link rel="icon" type="image/png" href="/icon.png" />
</svelte:head>

{#if isApp}
	<div class="relative h-screen overflow-hidden bg-[#e8e8e3] dark:bg-[#2b2a2a]">
		{@render children()}
	</div>
{:else}
	<div class="flex min-h-screen flex-col overflow-x-hidden bg-[#e8e8e3] dark:bg-[#2b2a2a]">
		<Navbar isDark={websiteDark} onToggleDark={toggleWebsiteDark} />
		<main class="flex-grow">
			{@render children()}
		</main>
		<footer class="mt-auto py-8">
			<div
				class="mx-auto max-w-4xl border-t border-gray-100 px-4 pt-8 text-center md:px-6 dark:border-gray-700"
			>
				<a
					href="https://github.com/felixscode/jolly"
					target="_blank"
					rel="noopener noreferrer"
					aria-label="GitHub"
					class="mb-3 inline-block text-gray-400 transition-colors hover:text-[#960200] dark:text-gray-500 dark:hover:text-[#ffd046]"
				>
					<svg
						xmlns="http://www.w3.org/2000/svg"
						width="20"
						height="20"
						viewBox="0 0 24 24"
						fill="currentColor"
						aria-hidden="true"
					>
						<path
							d="M12 0C5.374 0 0 5.373 0 12c0 5.302 3.438 9.8 8.207 11.387.599.111.793-.261.793-.577v-2.234c-3.338.726-4.033-1.416-4.033-1.416-.546-1.387-1.333-1.756-1.333-1.756-1.089-.745.083-.729.083-.729 1.205.084 1.839 1.237 1.839 1.237 1.07 1.834 2.807 1.304 3.492.997.107-.775.418-1.305.762-1.604-2.665-.305-5.467-1.334-5.467-5.931 0-1.311.469-2.381 1.236-3.221-.124-.303-.535-1.524.117-3.176 0 0 1.008-.322 3.301 1.23A11.509 11.509 0 0 1 12 5.803c1.02.005 2.047.138 3.006.404 2.291-1.552 3.297-1.23 3.297-1.23.653 1.653.242 2.874.118 3.176.77.84 1.235 1.911 1.235 3.221 0 4.609-2.807 5.624-5.479 5.921.43.372.823 1.102.823 2.222v3.293c0 .319.192.694.801.576C20.566 21.797 24 17.3 24 12c0-6.627-5.373-12-12-12z"
						/>
					</svg>
				</a>
			</div>
		</footer>
	</div>
{/if}
