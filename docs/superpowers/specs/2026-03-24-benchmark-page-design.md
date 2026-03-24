# Benchmark Page Design

## Overview

Add a `/benchmark` page to the Jolly website that helps users choose the right model by presenting benchmark data with technical insights. The page displays pre-aggregated results from the existing benchmark suite, covering all models across English and German text.

## Goals

- Help users understand trade-offs between models (speed, accuracy, memory) so they can pick the right one
- Provide technical insights into how different approaches (local LLM, API, conventional grammar checker) compare
- Present results for both English and German

## Page Structure

### Route

New SvelteKit route: `src/routes/benchmark/+page.svelte`

### Navigation

Add "Benchmark" link to `Navbar.svelte` between "Download" and "About". Current mobile layout uses `gap-4 md:gap-8` — verify this still looks good with three text links on small screens and adjust gap if needed.

### Layout (top to bottom)

1. **BirdScene** — parabolic fly-in animation (bird enters from top-right, arcs down along a curve, lands center)
2. **HR divider** — consistent with About/Download pages
3. **Heading** — "Benchmarks", styled with `text-2xl font-bold text-[#423f37] dark:text-[#e8e8e3]`
4. **Methodology text** — paragraph explaining:
   - What models were benchmarked (all 9 — see Models in scope below)
   - Test setup: 8 test cases total (4 English, 4 German) across short, medium, and email-length texts
   - What each metric means (exact match, score, time, memory)
5. **English results table** — subheading "English", data table with per-model aggregated metrics
6. **German results table** — subheading "German", same table format
7. **Interpretation text** — technical insights summarizing what the data reveals about each model's strengths and weaknesses

### Container & Styling

Follows existing page patterns:
- Container: `mx-auto max-w-4xl px-4 pt-8 pb-12 md:px-6 md:pt-16 md:pb-24`
- Headings: `text-2xl font-bold text-[#423f37] dark:text-[#e8e8e3]`
- Body text: `leading-relaxed text-gray-500 dark:text-gray-400`
- Dark mode support throughout

### BirdScene Animation

Unique parabolic arc fly-in, implemented as a **self-contained inline animation** within the page component (same pattern as About and Download pages — NOT using the shared `BirdScene.svelte` component, which is the interactive correction UI for the Home/App pages):
- Bird starts off-screen (top-right area)
- Follows a parabolic curve (translateX + translateY) sweeping down and left
- Lands at center position
- Implementation via inline style transitions with setTimeout, matching the About/Download pattern
- After landing: normal idle behavior (blinking loop)

### Data Table

- User will provide a Tailwind UI snippet for table styling — implementation should use a simple default table with proper semantic HTML (`<thead>`, `<th scope="col">`, `<tbody>`) until the snippet is provided
- Table columns: Model Name, Exact Match (%), Score (0–1), Time (ms), Memory (MB)
- One table for English results, one for German results
- Data is hardcoded — pre-aggregated averages computed from `benchmark_results.csv`

### Data Source

Static/hardcoded values. The benchmark CSV contains 72 data rows across 9 models and 8 test cases (4 English, 4 German). Values will be averaged per model per language and written directly into the component. No build-time or runtime CSV parsing.

**Models in scope (all 9, using display names from CSV):**
- "Harper" (conventional grammar checker)
- "OpenRouter gpt-4o-mini" (cloud API)
- "GRMR 2B Instruct" (local LLM)
- "GRMR V3 G4B (Q2_K)" (local LLM)
- "GRMR V3 G4B (Q4_K_M)" (local LLM)
- "GRMR V3 G4B (Q8_0)" (local LLM)
- "Mistral 7B Instruct v0.3" (local LLM)
- "Qwen3 1.7B" (local LLM)
- "Qwen3.5 4B" (local LLM)

**Metrics per model per language:**
- Exact match rate: percentage of test cases where output === expected
- Score: 0.0–1.0 measuring closeness to expected output
- Time: wall-clock milliseconds
- Memory: resident set size in MB

## Out of Scope

- Interactive filtering or sorting
- Client-side CSV parsing
- Chart/graph visualizations
- Per-test-case detail view
- Auto-updating from CSV at build time (can be added later)
