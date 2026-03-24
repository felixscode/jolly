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

Add "Benchmark" link to `Navbar.svelte` between "Download" and "About".

### Layout (top to bottom)

1. **BirdScene** — parabolic fly-in animation (bird enters from top-right, arcs down along a curve, lands center)
2. **HR divider** — consistent with About/Download pages
3. **Heading** — "Benchmarks", styled with `text-2xl font-bold text-[#423f37] dark:text-[#e8e8e3]`
4. **Methodology text** — paragraph explaining:
   - What models were benchmarked (Harper, OpenRouter GPT-4o-mini, GRMR 2B Q4_K_M, GRMR V3 G4B Q2_K, GRMR V3 G4B Q4_K_M)
   - Test setup: 8 test cases across short, medium, and email-length texts in English and German
   - What each metric means (exact match, similarity, time, memory)
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

Unique parabolic arc fly-in:
- Bird starts off-screen (top-right area)
- Follows a parabolic curve (translateX + translateY) sweeping down and left
- Lands at center position
- Implementation via CSS keyframes or inline style transitions with setTimeout, matching the pattern used on About/Download pages
- After landing: normal idle behavior (blinking loop)

### Data Table

- User will provide a Tailwind UI snippet for table styling
- Table columns: Model Name, Exact Match Rate (%), Avg Similarity, Avg Time (ms), Avg Memory (MB)
- One table for English results, one for German results
- Data is hardcoded — pre-aggregated averages computed from `benchmark_results.csv`

### Data Source

Static/hardcoded values. The benchmark CSV contains 234 rows across 5 models and 8 test cases. Values will be averaged per model per language and written directly into the component. No build-time or runtime CSV parsing.

**Models in scope:**
- Harper (conventional grammar checker)
- OpenRouter GPT-4o-mini (cloud API)
- GRMR 2B Instruct Q4_K_M (local, ~2.8 GB)
- GRMR V3 G4B Q2_K (local, ~1.9 GB)
- GRMR V3 G4B Q4_K_M (local, ~4.1 GB)

**Metrics per model per language:**
- Exact match rate: percentage of test cases where output === expected
- Average similarity: 0.0–1.0 score
- Average time: milliseconds
- Average memory: RSS in MB

## Out of Scope

- Interactive filtering or sorting
- Client-side CSV parsing
- Chart/graph visualizations
- Per-test-case detail view
- Auto-updating from CSV at build time (can be added later)
