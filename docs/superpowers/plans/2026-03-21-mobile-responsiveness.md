# Mobile Responsiveness Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the Jolly website (Home, About, Download) fully responsive on mobile and tablet screens using Tailwind breakpoint prefixes.

**Architecture:** Add responsive utility classes (`sm:`, `md:`) to existing Tailwind markup across 5 files. No new components, no structural changes. Grids collapse to single columns on mobile, spacing/sizing scales down.

**Tech Stack:** SvelteKit, Tailwind CSS v4, Svelte 5

---

### Task 1: Navbar — responsive spacing, text, and logo

**Files:**
- Modify: `src/lib/components/Navbar.svelte`

- [ ] **Step 1: Update outer nav padding**

Change line 11:
```svelte
<!-- FROM -->
<nav class="px-6 py-4">
<!-- TO -->
<nav class="px-4 py-4 md:px-6">
```

- [ ] **Step 2: Update link/button group gap**

Change line 16:
```svelte
<!-- FROM -->
<div class="flex items-center gap-8">
<!-- TO -->
<div class="flex items-center gap-4 md:gap-8">
```

- [ ] **Step 3: Update nav link text sizes**

Change the two `<a>` links (lines 17-22 and 23-28) — replace `text-base` with `text-sm md:text-base` in both:
```svelte
<!-- FROM (both links) -->
class="text-base font-bold ..."
<!-- TO -->
class="text-sm font-bold md:text-base ..."
```

- [ ] **Step 4: Scale logo height**

Change line 14:
```svelte
<!-- FROM -->
<img src="/jolly_heading.svg" alt="Jolly" class="h-21" />
<!-- TO -->
<img src="/jolly_heading.svg" alt="Jolly" class="h-14 md:h-21" />
```

- [ ] **Step 5: Verify in browser at 375px and 1200px widths**

Run: `npm run dev` and check both viewport sizes. Logo should shrink, links should stay visible but with tighter spacing.

- [ ] **Step 6: Commit**

```bash
git add src/lib/components/Navbar.svelte
git commit -m "feat: make navbar responsive for mobile"
```

---

### Task 2: Homepage — responsive hero, feature strip, and container

**Files:**
- Modify: `src/routes/+page.svelte`

- [ ] **Step 1: Fix the container — change height to min-height and add responsive padding**

Change lines 19-21:
```svelte
<!-- FROM -->
<div
	class="mx-auto flex max-w-4xl flex-col overflow-x-hidden px-6"
	style="height: calc(100svh - 116px - 117px);"
>
<!-- TO -->
<div
	class="mx-auto flex max-w-4xl flex-col overflow-x-hidden px-4 md:px-6"
	style="min-height: calc(100svh - 116px - 117px);"
>
```

- [ ] **Step 2: Make hero grid responsive**

Change line 24:
```svelte
<!-- FROM -->
<div class="grid flex-1 grid-cols-2 items-center gap-12">
<!-- TO -->
<div class="grid flex-1 grid-cols-1 items-center gap-6 md:grid-cols-2 md:gap-12">
```

- [ ] **Step 3: Center hero text on mobile**

Change line 26:
```svelte
<!-- FROM -->
<div>
<!-- TO -->
<div class="text-center md:text-left">
```

- [ ] **Step 4: Make heading font size responsive**

Change line 32:
```svelte
<!-- FROM -->
<h1 class="mb-3 text-4xl leading-tight font-extrabold text-[#423f37] dark:text-[#e8e8e3]">
<!-- TO -->
<h1 class="mb-3 text-3xl leading-tight font-extrabold text-[#423f37] md:text-4xl dark:text-[#e8e8e3]">
```

- [ ] **Step 5: Center download button on mobile**

Change line 40-44 — add `text-center md:text-left` wrapper or make the button block on mobile. Simplest: the parent div already gets `text-center md:text-left` from Step 3, so the inline-block button will center automatically. No additional change needed.

- [ ] **Step 6: Make feature strip responsive**

Change line 61:
```svelte
<!-- FROM -->
<div class="grid grid-cols-3 gap-12 py-6">
<!-- TO -->
<div class="grid grid-cols-1 gap-6 py-4 sm:grid-cols-3 sm:gap-12 sm:py-6">
```

- [ ] **Step 7: Center feature items on mobile**

Add `text-center sm:text-left` to each of the three feature `<div>`s (lines 62, 68, 74):
```svelte
<!-- FROM (each of the three) -->
<div>
<!-- TO -->
<div class="text-center sm:text-left">
```

- [ ] **Step 8: Verify in browser at 375px and 1200px**

Hero should stack vertically on mobile (text on top, bird below). Feature strip should be single column. Desktop should look unchanged.

- [ ] **Step 9: Commit**

```bash
git add src/routes/+page.svelte
git commit -m "feat: make homepage responsive for mobile"
```

---

### Task 3: BirdScene — responsive bird dimensions

**Files:**
- Modify: `src/lib/components/BirdScene.svelte`

- [ ] **Step 1: Make the bird image container responsive**

Change line 375:
```svelte
<!-- FROM -->
<div class="relative h-44 w-36">
<!-- TO -->
<div class="relative h-32 w-28 md:h-44 md:w-36">
```

- [ ] **Step 2: Verify the BirdScene renders correctly at 375px**

The bird should appear smaller on mobile. The branch will be clipped by `overflow-x-hidden` on the homepage container — this is expected and acceptable.

- [ ] **Step 3: Commit**

```bash
git add src/lib/components/BirdScene.svelte
git commit -m "feat: make BirdScene responsive for mobile"
```

---

### Task 4: About page — responsive grid and spacing

**Files:**
- Modify: `src/routes/about/+page.svelte`

- [ ] **Step 1: Update outer container padding and spacing**

Change line 59:
```svelte
<!-- FROM -->
<div class="mx-auto max-w-4xl px-6 pt-16 pb-24">
<!-- TO -->
<div class="mx-auto max-w-4xl px-4 pt-8 pb-12 md:px-6 md:pt-16 md:pb-24">
```

- [ ] **Step 2: Reduce bird header margin on mobile**

Change line 61:
```svelte
<!-- FROM -->
<div class="mb-16 flex items-center justify-center">
<!-- TO -->
<div class="mb-8 flex items-center justify-center md:mb-16">
```

- [ ] **Step 3: Reduce first hr margin on mobile**

Change line 100:
```svelte
<!-- FROM -->
<hr class="mb-16 border-gray-200 dark:border-gray-700" />
<!-- TO -->
<hr class="mb-8 border-gray-200 md:mb-16 dark:border-gray-700" />
```

- [ ] **Step 4: Make content grid responsive**

Change line 103:
```svelte
<!-- FROM -->
<div class="grid grid-cols-2 gap-16">
<!-- TO -->
<div class="grid grid-cols-1 gap-8 md:grid-cols-2 md:gap-16">
```

- [ ] **Step 5: Reduce bottom hr margins on mobile**

Change line 158:
```svelte
<!-- FROM -->
<hr class="mt-16 mb-16 border-gray-200 dark:border-gray-700" />
<!-- TO -->
<hr class="mt-8 mb-8 border-gray-200 md:mt-16 md:mb-16 dark:border-gray-700" />
```

- [ ] **Step 6: Verify at 375px and 1200px**

Content should stack single-column on mobile. Desktop should look unchanged.

- [ ] **Step 7: Commit**

```bash
git add src/routes/about/+page.svelte
git commit -m "feat: make about page responsive for mobile"
```

---

### Task 5: Download page — responsive platform grid and spacing

**Files:**
- Modify: `src/routes/download/+page.svelte`

- [ ] **Step 1: Update container padding**

Change line 64:
```svelte
<!-- FROM -->
<div class="mx-auto flex max-w-4xl flex-col justify-center px-6" style="min-height: calc(100svh - 116px - 117px);">
<!-- TO -->
<div class="mx-auto flex max-w-4xl flex-col justify-center px-4 md:px-6" style="min-height: calc(100svh - 116px - 117px);">
```

- [ ] **Step 2: Reduce bird header margins on mobile**

Change line 66:
```svelte
<!-- FROM -->
<div class="-mt-8 mb-12 flex items-center justify-center">
<!-- TO -->
<div class="-mt-4 mb-6 flex items-center justify-center sm:-mt-8 sm:mb-12">
```

- [ ] **Step 3: Reduce first hr margin on mobile**

Change line 124:
```svelte
<!-- FROM -->
<hr class="mb-12 border-gray-200 dark:border-gray-700" />
<!-- TO -->
<hr class="mb-6 border-gray-200 sm:mb-12 dark:border-gray-700" />
```

- [ ] **Step 4: Make platform grid responsive**

Change line 127:
```svelte
<!-- FROM -->
<div class="grid grid-cols-3 gap-12">
<!-- TO -->
<div class="grid grid-cols-1 gap-6 sm:grid-cols-3 sm:gap-12">
```

- [ ] **Step 5: Reduce bottom hr margins on mobile**

Change line 205:
```svelte
<!-- FROM -->
<hr class="mt-12 mb-12 border-gray-200 dark:border-gray-700" />
<!-- TO -->
<hr class="mt-6 mb-6 border-gray-200 sm:mt-12 sm:mb-12 dark:border-gray-700" />
```

- [ ] **Step 6: Verify at 375px and 1200px**

Platforms should stack vertically on mobile. Linux dropdown should still open upward correctly. Desktop should look unchanged.

- [ ] **Step 7: Commit**

```bash
git add src/routes/download/+page.svelte
git commit -m "feat: make download page responsive for mobile"
```

---

### Task 6: Layout footer — responsive padding

**Files:**
- Modify: `src/routes/+layout.svelte`

- [ ] **Step 1: Update footer container padding**

Change line 62:
```svelte
<!-- FROM -->
class="mx-auto max-w-4xl border-t border-gray-100 px-6 pt-8 text-center dark:border-gray-700"
<!-- TO -->
class="mx-auto max-w-4xl border-t border-gray-100 px-4 pt-8 text-center md:px-6 dark:border-gray-700"
```

- [ ] **Step 2: Verify footer at 375px**

Footer should have slightly tighter horizontal padding but otherwise look the same.

- [ ] **Step 3: Commit**

```bash
git add src/routes/+layout.svelte
git commit -m "feat: make footer responsive for mobile"
```
