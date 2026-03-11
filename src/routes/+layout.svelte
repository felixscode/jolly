<script lang="ts">
	import './layout.css';
	import Navbar from '$lib/components/Navbar.svelte';

	let { children } = $props();
</script>

<svelte:head>
	<link rel="icon" href="/jolly_normal.svg" />
</svelte:head>

<div class="flex min-h-screen flex-col">
	<Navbar />
	<main class="flex-grow">
		{@render children()}
	</main>
	<footer class="mt-auto border-t border-gray-100 py-8">
		<div class="mx-auto max-w-4xl px-6 text-center">
			<p class="text-sm text-gray-400">&copy; 2025 Jolly — made by [your name]. Free and open source.</p>
		</div>
	</footer>
</div>
