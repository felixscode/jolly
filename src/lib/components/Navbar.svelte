<script lang="ts">
	let {
		isDark = false,
		onToggleDark
	}: {
		isDark?: boolean;
		onToggleDark?: () => void;
	} = $props();
</script>

<nav class="px-4 py-4 md:px-6">
	<div class="mx-auto flex max-w-4xl items-center justify-between">
		<a href="/">
			<img src="/jolly_heading.svg" alt="Jolly" class="h-14 md:h-21" />
		</a>
		<div class="flex items-center gap-4 md:gap-8">
			<a
				href="/download"
				class="text-sm font-bold md:text-base text-[#423f37] transition-colors hover:text-[#960200] dark:text-[#e8e8e3] dark:hover:text-[#ffd046]"
			>
				Download
			</a>
			<a
				href="/about"
				class="text-sm font-bold md:text-base text-[#423f37] transition-colors hover:text-[#960200] dark:text-[#e8e8e3] dark:hover:text-[#ffd046]"
			>
				About
			</a>
			<button
				onclick={() => onToggleDark?.()}
				class="text-[#423f37] transition-colors hover:text-[#960200] dark:text-[#e8e8e3] dark:hover:text-[#ffd046]"
				aria-label={isDark ? 'Switch to light mode' : 'Switch to dark mode'}
			>
				{#if isDark}
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
						<circle cx="12" cy="12" r="5" /><line x1="12" y1="1" x2="12" y2="3" /><line
							x1="12"
							y1="21"
							x2="12"
							y2="23"
						/><line x1="4.22" y1="4.22" x2="5.64" y2="5.64" /><line
							x1="18.36"
							y1="18.36"
							x2="19.78"
							y2="19.78"
						/><line x1="1" y1="12" x2="3" y2="12" /><line x1="21" y1="12" x2="23" y2="12" /><line
							x1="4.22"
							y1="19.78"
							x2="5.64"
							y2="18.36"
						/><line x1="18.36" y1="5.64" x2="19.78" y2="4.22" />
					</svg>
				{:else}
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
						<path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z" />
					</svg>
				{/if}
			</button>
			</div>
	</div>
</nav>
