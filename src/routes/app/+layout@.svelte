<script lang="ts">
	import '../layout.css';

	let { children } = $props();
</script>

<svelte:head>
	<link rel="icon" href="/jolly_normal.svg" />
</svelte:head>

<div class="flex h-screen flex-col overflow-hidden bg-white">
	{@render children()}
</div>
