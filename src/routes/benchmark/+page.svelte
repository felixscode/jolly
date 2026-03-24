<script lang="ts">
	import { onMount } from 'svelte';

	let flyFrame = $state<1 | 2>(1);
	let flying = $state(true);
	let flyingIn = $state(true);
	let blinking = $state(false);

	const englishData = [
		{ model: 'OpenRouter gpt-4o-mini', exact: 75, errorsFixed: 100, time: 1526, mem: 225 },
		{ model: 'GRMR V3 3B', exact: 50, errorsFixed: 98, time: 2746, mem: 3496 },
		{ model: 'GRMR V3 4B', exact: 75, errorsFixed: 98, time: 3910, mem: 4118 },
		{ model: 'Gemma 3 4B Instruct', exact: 25, errorsFixed: 92, time: 4411, mem: 4119 },
		{ model: 'Mistral 7B Instruct v0.3', exact: 50, errorsFixed: 92, time: 6517, mem: 7666 },
		{ model: 'Harper', exact: 50, errorsFixed: 90, time: 100, mem: 187 }
	];

	const germanData = [
		{ model: 'OpenRouter gpt-4o-mini', exact: 75, errorsFixed: 100, time: 1501, mem: 226 },
		{ model: 'Mistral 7B Instruct v0.3', exact: 50, errorsFixed: 95, time: 9395, mem: 7666 },
		{ model: 'GRMR V3 4B', exact: 0, errorsFixed: 68, time: 4532, mem: 4118 },
		{ model: 'GRMR V3 3B', exact: 0, errorsFixed: 32, time: 4157, mem: 3496 },
		{ model: 'Gemma 3 4B Instruct', exact: 0, errorsFixed: 32, time: 4728, mem: 4119 },
		{ model: 'Harper', exact: 0, errorsFixed: 0, time: 278, mem: 202 }
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
	<h1 class="mb-4 text-center text-2xl font-bold text-[#423f37] dark:text-[#e8e8e3]">Benchmarks</h1>

	<!-- Methodology -->
	<div class="mx-auto mb-8 max-w-2xl text-center md:mb-16">
		<p class="leading-relaxed text-gray-500 dark:text-gray-400">
			Each model was tested on 8 cases — 4 English, 4 German — spanning short sentences, medium
			paragraphs, and email-length texts with intentional typos.
		</p>
	</div>

	{#snippet dataTable(data: typeof englishData)}
		<div class="mb-8 overflow-x-auto md:mb-16">
			<table class="w-full text-left text-sm text-gray-500 dark:text-gray-400">
				<thead
					class="border-b border-gray-200 text-xs text-[#423f37] uppercase dark:border-gray-700 dark:text-[#e8e8e3]"
				>
					<tr>
						<th scope="col" class="py-3 pr-6">Model</th>
						<th scope="col" class="py-3 pr-6">Exact Match (%)</th>
						<th scope="col" class="py-3 pr-6">Errors Fixed (%)</th>
						<th scope="col" class="py-3 pr-6">Time (ms)</th>
						<th scope="col" class="py-3">Memory (MB)</th>
					</tr>
				</thead>
				<tbody>
					{#each data as row}
						<tr class="border-b border-gray-100 dark:border-gray-800">
							<td class="py-3 pr-6 font-medium whitespace-nowrap text-[#423f37] dark:text-[#e8e8e3]"
								>{row.model}</td
							>
							<td class="py-3 pr-6">{row.exact}%</td>
							<td class="py-3 pr-6">{row.errorsFixed}%</td>
							<td class="py-3 pr-6">{row.time.toLocaleString()}</td>
							<td class="py-3">{row.mem.toLocaleString()}</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>
	{/snippet}

	<h2 class="mb-4 text-xl font-bold text-[#423f37] dark:text-[#e8e8e3]">English</h2>
	{@render dataTable(englishData)}

	<h2 class="mb-4 text-xl font-bold text-[#423f37] dark:text-[#e8e8e3]">German</h2>
	{@render dataTable(germanData)}

	<div
		class="mb-8 grid grid-cols-2 gap-x-8 gap-y-1 text-xs text-gray-400 md:mb-16 dark:text-gray-500"
	>
		<span
			><strong>Exact Match</strong> — corrected output matched the expected text character-for-character</span
		>
		<span
			><strong>Errors Fixed</strong> — percentage of individual typos the model caught and corrected</span
		>
		<span><strong>Time</strong> — wall-clock milliseconds from input to corrected output</span>
		<span><strong>Memory</strong> — resident set size in megabytes while the model is loaded</span>
	</div>

	<hr class="mb-8 border-gray-200 md:mb-16 dark:border-gray-700" />

	<!-- Interpretation -->
	<div class="mx-auto max-w-2xl text-center">
		<h2 class="mb-4 text-xl font-bold text-[#423f37] dark:text-[#e8e8e3]">What this means</h2>
		<p class="mb-4 leading-relaxed text-gray-500 dark:text-gray-400">
			The models fall into three categories. Harper is a rule-based grammar checker — it matches
			words against a dictionary and applies fixes instantly, but it only knows English and
			struggles with context-dependent errors. Is a linter at the end. The GRMR models are small
			LLMs fine-tuned specifically for grammar correction — they understand context and fix more
			errors than Harper, but they were trained primarily on English data, so German accuracy is
			limited. The general-purpose models (Gemma, Mistral) are larger instruction-following LLMs
			that use a system prompt to correct text — they aren't specialized for grammar but their
			broader training data gives them better multilingual support.
		</p>
		<p class="mb-4 leading-relaxed text-gray-500 dark:text-gray-400">
			OpenRouter's GPT-4o-mini fixes every single error across both languages because it is a much
			larger model running on powerful remote hardware. The tradeoff is that your text leaves your
			device and you need an API key.
		</p>
		<p class="leading-relaxed text-gray-500 dark:text-gray-400">
			For local English-only use, GRMR V3 4B (2.5 GB) is the recommended choice — it fixes 87% of
			errors and is fast. For multilingual use, Mistral 7B (4.7 GB) is the only local model that
			handles German well. For the best results with no hardware constraints, OpenRouter is
			unbeatable.
		</p>
	</div>
</div>
