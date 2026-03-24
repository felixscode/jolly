# Benchmark Page Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a `/benchmark` page to the Jolly website that displays aggregated benchmark results per model per language, with methodology explanation and interpretation text.

**Architecture:** New SvelteKit route at `src/routes/benchmark/+page.svelte` with inline bird animation (parabolic arc), hardcoded aggregated data, and two HTML tables (English/German). Navbar gets a new link.

**Tech Stack:** SvelteKit 5, Svelte 5 runes, Tailwind CSS 4, TypeScript

**Spec:** `docs/superpowers/specs/2026-03-24-benchmark-page-design.md`

---

## File Structure

- **Create:** `src/routes/benchmark/+page.svelte` — the benchmark page component (inline bird animation, methodology text, two data tables, interpretation text)
- **Modify:** `src/lib/components/Navbar.svelte` — add "Benchmark" link between "Download" and "About"

---

### Task 1: Add Benchmark link to Navbar

**Files:**
- Modify: `src/lib/components/Navbar.svelte:16-28`

- [ ] **Step 1: Add the Benchmark link**

In `src/lib/components/Navbar.svelte`, add a new `<a>` element for Benchmark between the existing Download and About links. Use the same styling classes as the existing links:

```svelte
<a
	href="/benchmark"
	class="text-sm font-bold md:text-base text-[#423f37] transition-colors hover:text-[#960200] dark:text-[#e8e8e3] dark:hover:text-[#ffd046]"
>
	Benchmark
</a>
```

Insert this between the Download `<a>` and the About `<a>` inside the `<div class="flex items-center gap-4 md:gap-8">`.

- [ ] **Step 2: Verify navbar renders**

Run: `cd /home/dev/jolly && npm run dev`

Open browser, check navbar shows Download | Benchmark | About | dark-mode-toggle. Verify spacing looks good on both desktop and narrow viewport. If `gap-4` feels too tight on mobile with 3 links, reduce to `gap-3` on mobile.

- [ ] **Step 3: Commit**

```bash
git add src/lib/components/Navbar.svelte
git commit -m "feat: add Benchmark link to navbar"
```

---

### Task 2: Create benchmark page with bird animation

**Files:**
- Create: `src/routes/benchmark/+page.svelte`

- [ ] **Step 1: Create the page file with bird animation and page skeleton**

Create `src/routes/benchmark/+page.svelte`. The animation follows the same pattern as `src/routes/about/+page.svelte` (inline bird with `onMount`, `flyFrame`, `flying`, `flyingIn`, `blinking` state, sprite stacking) but with a **parabolic arc** fly-in instead of horizontal.

The parabolic arc: bird starts off-screen top-right (`translateX(500px) translateY(-400px)`), transitions to `translateX(0) translateY(0)` — the CSS easing creates the curve effect.

```svelte
<script lang="ts">
	import { onMount } from 'svelte';

	let flyFrame = $state<1 | 2>(1);
	let flying = $state(true);
	let flyingIn = $state(true);
	let blinking = $state(false);

	const englishData = [
		{ model: 'Harper', exact: 50, score: 0.94, time: 100, mem: 179 },
		{ model: 'OpenRouter gpt-4o-mini', exact: 75, score: 1.00, time: 1301, mem: 213 },
		{ model: 'GRMR 2B Instruct', exact: 50, score: 0.97, time: 5639, mem: 2806 },
		{ model: 'GRMR V3 G4B (Q2_K)', exact: 75, score: 0.98, time: 3567, mem: 1912 },
		{ model: 'GRMR V3 G4B (Q4_K_M)', exact: 75, score: 0.98, time: 3922, mem: 4096 },
		{ model: 'GRMR V3 G4B (Q8_0)', exact: 75, score: 0.98, time: 7060, mem: 4202 },
		{ model: 'Mistral 7B Instruct v0.3', exact: 50, score: 0.96, time: 6606, mem: 7694 },
		{ model: 'Qwen3 1.7B', exact: 50, score: 0.73, time: 2997, mem: 2127 },
		{ model: 'Qwen3.5 4B', exact: 0, score: 0.00, time: 11425, mem: 4364 }
	];

	const germanData = [
		{ model: 'Harper', exact: 0, score: 0.00, time: 276, mem: 190 },
		{ model: 'OpenRouter gpt-4o-mini', exact: 50, score: 1.00, time: 1436, mem: 213 },
		{ model: 'GRMR 2B Instruct', exact: 0, score: 0.03, time: 3651, mem: 2806 },
		{ model: 'GRMR V3 G4B (Q2_K)', exact: 0, score: 0.23, time: 5503, mem: 1912 },
		{ model: 'GRMR V3 G4B (Q4_K_M)', exact: 0, score: 0.44, time: 5224, mem: 4096 },
		{ model: 'GRMR V3 G4B (Q8_0)', exact: 0, score: 0.42, time: 6467, mem: 4202 },
		{ model: 'Mistral 7B Instruct v0.3', exact: 50, score: 0.97, time: 9336, mem: 7694 },
		{ model: 'Qwen3 1.7B', exact: 0, score: 0.32, time: 3618, mem: 2127 },
		{ model: 'Qwen3.5 4B', exact: 50, score: 0.92, time: 6998, mem: 4364 }
	];

	onMount(() => {
		let mounted = true;

		const flyInterval = setInterval(() => {
			flyFrame = flyFrame === 1 ? 2 : 1;
		}, 150);

		setTimeout(() => {
			flyingIn = false;
		}, 50);

		setTimeout(() => {
			if (!mounted) return;
			clearInterval(flyInterval);
			flying = false;
		}, 2300);

		function scheduleBlink() {
			const delay = 2500 + Math.random() * 4000;
			setTimeout(() => {
				if (!mounted) return;
				if (!flying) {
					blinking = true;
					setTimeout(() => {
						if (!mounted) return;
						blinking = false;
						scheduleBlink();
					}, 160);
				} else {
					scheduleBlink();
				}
			}, delay);
		}
		setTimeout(scheduleBlink, 3300);

		return () => {
			mounted = false;
			clearInterval(flyInterval);
		};
	});

	const pose = $derived(
		flying ? (flyFrame === 1 ? 'fly1' : 'fly2') : blinking ? 'blink' : 'normal'
	);
</script>

<div class="mx-auto max-w-4xl px-4 pt-8 pb-12 md:px-6 md:pt-16 md:pb-24">
	<!-- Bird: parabolic arc fly-in from top-right -->
	<div class="mb-8 flex items-center justify-center md:mb-16">
		<div
			class="relative h-28 w-24"
			style="
				transform: {flyingIn ? 'translate(500px, -400px) scale(0.75)' : 'translate(0, 0) scale(1)'};
				opacity: {flyingIn ? 0 : 1};
				transition: transform 2000ms cubic-bezier(0.2, 0.8, 0.3, 1), opacity 500ms ease-out;
			"
		>
			<img
				src="/jolly_normal.svg"
				alt="Jolly"
				class="absolute inset-0 h-full w-full object-contain"
				style="opacity: {pose === 'normal' ? 1 : 0};"
			/>
			<img
				src="/jolly_bilnzel.svg"
				alt=""
				aria-hidden="true"
				class="absolute inset-0 h-full w-full object-contain"
				style="opacity: {pose === 'blink' ? 1 : 0};"
			/>
			<img
				src="/jolly_fly1.svg"
				alt=""
				aria-hidden="true"
				class="absolute inset-0 h-full w-full object-contain"
				style="opacity: {pose === 'fly1' ? 1 : 0};"
			/>
			<img
				src="/jolly_fly2.svg"
				alt=""
				aria-hidden="true"
				class="absolute inset-0 h-full w-full object-contain"
				style="opacity: {pose === 'fly2' ? 1 : 0};"
			/>
		</div>
	</div>

	<hr class="mb-8 border-gray-200 md:mb-16 dark:border-gray-700" />

	<!-- Heading -->
	<h1 class="mb-4 text-center text-2xl font-bold text-[#423f37] dark:text-[#e8e8e3]">
		Benchmarks
	</h1>

	<!-- Methodology -->
	<div class="mx-auto mb-8 max-w-2xl text-center md:mb-16">
		<p class="leading-relaxed text-gray-500 dark:text-gray-400">
			Each model was tested on 8 cases — 4 English, 4 German — spanning short sentences, medium
			paragraphs, and email-length texts with intentional typos. Exact match means the corrected
			output matched the expected text character-for-character. Score measures how close the output
			was on a 0–1 scale. Time is wall-clock milliseconds. Memory is resident set size in MB.
		</p>
	</div>

	<!-- English table -->
	<h2 class="mb-4 text-xl font-bold text-[#423f37] dark:text-[#e8e8e3]">English</h2>
	<div class="mb-8 overflow-x-auto md:mb-16">
		<table class="w-full text-left text-sm text-gray-500 dark:text-gray-400">
			<thead class="border-b border-gray-200 text-xs uppercase text-[#423f37] dark:border-gray-700 dark:text-[#e8e8e3]">
				<tr>
					<th scope="col" class="py-3 pr-6">Model</th>
					<th scope="col" class="py-3 pr-6">Exact Match</th>
					<th scope="col" class="py-3 pr-6">Score</th>
					<th scope="col" class="py-3 pr-6">Time (ms)</th>
					<th scope="col" class="py-3">Memory (MB)</th>
				</tr>
			</thead>
			<tbody>
				{#each englishData as row}
					<tr class="border-b border-gray-100 dark:border-gray-800">
						<td class="py-3 pr-6 font-medium text-[#423f37] whitespace-nowrap dark:text-[#e8e8e3]">{row.model}</td>
						<td class="py-3 pr-6">{row.exact}%</td>
						<td class="py-3 pr-6">{row.score.toFixed(2)}</td>
						<td class="py-3 pr-6">{row.time.toLocaleString()}</td>
						<td class="py-3">{row.mem.toLocaleString()}</td>
					</tr>
				{/each}
			</tbody>
		</table>
	</div>

	<!-- German table -->
	<h2 class="mb-4 text-xl font-bold text-[#423f37] dark:text-[#e8e8e3]">German</h2>
	<div class="mb-8 overflow-x-auto md:mb-16">
		<table class="w-full text-left text-sm text-gray-500 dark:text-gray-400">
			<thead class="border-b border-gray-200 text-xs uppercase text-[#423f37] dark:border-gray-700 dark:text-[#e8e8e3]">
				<tr>
					<th scope="col" class="py-3 pr-6">Model</th>
					<th scope="col" class="py-3 pr-6">Exact Match</th>
					<th scope="col" class="py-3 pr-6">Score</th>
					<th scope="col" class="py-3 pr-6">Time (ms)</th>
					<th scope="col" class="py-3">Memory (MB)</th>
				</tr>
			</thead>
			<tbody>
				{#each germanData as row}
					<tr class="border-b border-gray-100 dark:border-gray-800">
						<td class="py-3 pr-6 font-medium text-[#423f37] whitespace-nowrap dark:text-[#e8e8e3]">{row.model}</td>
						<td class="py-3 pr-6">{row.exact}%</td>
						<td class="py-3 pr-6">{row.score.toFixed(2)}</td>
						<td class="py-3 pr-6">{row.time.toLocaleString()}</td>
						<td class="py-3">{row.mem.toLocaleString()}</td>
					</tr>
				{/each}
			</tbody>
		</table>
	</div>

	<hr class="mb-8 border-gray-200 md:mb-16 dark:border-gray-700" />

	<!-- Interpretation -->
	<div class="mx-auto max-w-2xl text-center">
		<h2 class="mb-4 text-2xl font-bold text-[#423f37] dark:text-[#e8e8e3]">What this means</h2>
		<p class="mb-4 leading-relaxed text-gray-500 dark:text-gray-400">
			Harper is the fastest option by far and handles English well, but it falls apart on German —
			it is an English grammar checker after all. OpenRouter's GPT-4o-mini delivers the best
			overall accuracy across both languages, but it requires an API key and sends text to a
			remote server.
		</p>
		<p class="mb-4 leading-relaxed text-gray-500 dark:text-gray-400">
			Among local models, the GRMR family offers the best balance of speed and English accuracy.
			The V3 G4B variants all score similarly on English but vary in speed and memory — Q2_K is
			the lightest at ~1.9 GB, while Q8_0 uses ~4.2 GB for marginal gains. German remains a weak
			spot for all GRMR variants. Mistral 7B is the only local model that handles German well,
			but it needs ~7.7 GB of RAM and is slower.
		</p>
		<p class="leading-relaxed text-gray-500 dark:text-gray-400">
			If you mostly write in English and want everything local, GRMR V3 G4B (Q2_K) gives you
			good accuracy at the lowest memory cost. If you need German support and can spare the
			RAM, Mistral 7B or Qwen3.5 4B are worth trying. For the best results with no hardware
			requirements, OpenRouter with an API key is the way to go.
		</p>
	</div>
</div>
```

- [ ] **Step 2: Verify the page renders**

Run: `cd /home/dev/jolly && npm run dev`

Open browser to `/benchmark`. Verify:
- Bird flies in with parabolic arc (top-right to center)
- Bird lands and starts blinking
- Heading "Benchmarks" visible
- Methodology paragraph visible
- Two tables render with correct data
- Interpretation section visible
- Dark mode toggle works
- Page is scrollable on mobile

- [ ] **Step 3: Commit**

```bash
git add src/routes/benchmark/+page.svelte
git commit -m "feat: add benchmark page with model comparison tables"
```

---

## Follow-up

- **Table styling:** User mentioned they have a Tailwind UI snippet for the tables. Once provided, swap the default table markup in `+page.svelte`.
- **Benchmark data:** User noted the benchmark is still being updated. Hardcoded values may need refreshing once the CSV is finalized.
