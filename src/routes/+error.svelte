<script lang="ts">
	import { page } from '$app/state';
</script>

<div class="flex min-h-[60vh] flex-col items-center justify-center text-center">
	<img src="/jolly_dead.svg" alt="Jolly is dead" class="mb-8 h-48" />
	<h1 class="mb-2 text-6xl font-extrabold text-[#423f37] dark:text-[#e8e8e3]">{page.status}</h1>
	<p class="mb-8 text-lg text-gray-500 dark:text-gray-400">
		{page.error?.message ?? 'Page not found'}
	</p>
	<a
		href="/"
		class="rounded-lg border-4 border-[#960200] bg-transparent px-6 py-3 text-sm font-bold text-[#423f37] transition-colors hover:bg-[#ffd046] hover:text-[#960200] dark:border-[#ffd046] dark:text-[#e8e8e3] dark:hover:bg-[#960200] dark:hover:text-[#ffd046]"
	>
		Go home
	</a>
</div>
