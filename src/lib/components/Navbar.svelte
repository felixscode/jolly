<script lang="ts">
	import { afterNavigate } from '$app/navigation';
	import { NAV_LINKS } from '$lib/nav';

	let {
		isDark = false,
		onToggleDark
	}: {
		isDark?: boolean;
		onToggleDark?: () => void;
	} = $props();

	let menuOpen = $state(false);

	afterNavigate(() => {
		menuOpen = false;
	});

	let menuEl: HTMLElement | undefined = $state();

	// pointerdown (not click): iOS Safari sends no click for taps on plain content,
	// so an outside tap would never close the menu.
	function onWindowPointerDown(e: PointerEvent) {
		if (menuEl && !e.composedPath().includes(menuEl)) menuOpen = false;
	}

	const linkClass =
		'font-bold text-[#423f37] transition-colors hover:text-[#960200] dark:text-[#e8e8e3] dark:hover:text-[#ffd046]';
</script>

<svelte:window onpointerdown={onWindowPointerDown} onkeydown={(e) => e.key === 'Escape' && (menuOpen = false)} />

{#snippet themeIcon()}
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
			aria-hidden="true"
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
			aria-hidden="true"
		>
			<path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z" />
		</svg>
	{/if}
{/snippet}

<nav class="px-4 py-4 md:px-6">
	<!-- Phones: centered logo, everything else in the menu on the right.
	     sm and up: logo left, links and theme toggle in a row on the right. -->
	<div
		class="relative mx-auto flex max-w-4xl items-center justify-center sm:justify-between sm:gap-4"
	>
		<a href="/" class="shrink-0">
			<img src="/jolly_heading.svg" alt="Jolly" class="h-12 sm:h-14 md:h-21" />
		</a>

		<div class="hidden items-center gap-4 sm:flex md:gap-8">
			{#each NAV_LINKS as link}
				<a href={link.href} class="py-2 text-sm md:text-base {linkClass}">{link.label}</a>
			{/each}
			<button
				onclick={() => onToggleDark?.()}
				class="-m-2 p-2 {linkClass}"
				aria-label={isDark ? 'Switch to light mode' : 'Switch to dark mode'}
			>
				{@render themeIcon()}
			</button>
		</div>

		<div class="absolute top-1/2 right-0 -translate-y-1/2 sm:hidden" bind:this={menuEl}>
			<button
				onclick={() => (menuOpen = !menuOpen)}
				class="-m-2 p-2 {linkClass}"
				aria-label={menuOpen ? 'Close menu' : 'Open menu'}
				aria-expanded={menuOpen}
				aria-controls="mobile-menu"
			>
				<svg
					xmlns="http://www.w3.org/2000/svg"
					width="24"
					height="24"
					viewBox="0 0 24 24"
					fill="none"
					stroke="currentColor"
					stroke-width="2.5"
					stroke-linecap="round"
					aria-hidden="true"
				>
					{#if menuOpen}
						<line x1="6" y1="6" x2="18" y2="18" /><line x1="18" y1="6" x2="6" y2="18" />
					{:else}
						<line x1="4" y1="7" x2="20" y2="7" /><line x1="4" y1="12" x2="20" y2="12" /><line
							x1="4"
							y1="17"
							x2="20"
							y2="17"
						/>
					{/if}
				</svg>
			</button>

			{#if menuOpen}
				<div
					id="mobile-menu"
					class="absolute top-full right-0 z-50 mt-4 w-48 overflow-hidden rounded-lg border-2 border-[#423f37] bg-[#e8e8e3] shadow-md dark:border-[#e8e8e3] dark:bg-[#2b2a2a]"
				>
					{#each NAV_LINKS as link}
						<a href={link.href} class="block px-5 py-3 text-base {linkClass}">{link.label}</a>
					{/each}
					<button
						onclick={() => onToggleDark?.()}
						class="flex w-full items-center gap-3 border-t border-gray-300 px-5 py-3 text-base dark:border-gray-600 {linkClass}"
					>
						{@render themeIcon()}
						{isDark ? 'Light mode' : 'Dark mode'}
					</button>
				</div>
			{/if}
		</div>
	</div>
</nav>
