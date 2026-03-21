# Mobile Responsiveness Design

**Date:** 2026-03-21
**Status:** Draft

## Problem

The Jolly website (Home, About, Download pages) is desktop-only with zero responsive styling. All layouts use fixed multi-column grids (`grid-cols-2`, `grid-cols-3`) designed for 1200px+ viewports. On mobile devices, elements collide, text gets crushed, and the experience is broken.

## Approach

**Tailwind Breakpoint Pass** — add responsive prefixes (`sm:`, `md:`, `lg:`) to existing Tailwind utility classes across all web-facing pages. No new components, no new abstractions. Grids collapse to single columns on mobile and expand at appropriate breakpoints.

### Breakpoint Strategy

Using Tailwind's default breakpoints:
- **Base (0px+):** Mobile-first styles
- **sm (640px+):** Small tablets / large phones in landscape
- **md (768px+):** Tablets and small laptops
- **lg (1024px+):** Desktop (current design target)

## Changes by Component

### Navbar (`src/lib/components/Navbar.svelte`)

- Gap: `gap-4 md:gap-8`
- Text size: `text-sm md:text-base`
- Padding: `px-4 md:px-6`
- Logo height: `h-14 md:h-21` (scales down from ~84px to ~56px on mobile)
- Links remain visible at all sizes (no hamburger menu)

### Homepage (`src/routes/+page.svelte`)

**Hero section:**
- Grid: `grid-cols-1 md:grid-cols-2`
- Gap: `gap-6 md:gap-12`
- Text alignment: `text-center md:text-left`
- BirdScene scales down on mobile (smaller height/width)
- Heading font size: `text-3xl md:text-5xl` (or similar)

**Feature strip (Local / Quiet / Free):**
- Grid: `grid-cols-1 sm:grid-cols-3`
- Gap: `gap-6 sm:gap-12`
- Each feature centers on mobile
- Padding: `py-4 sm:py-6`

**General:**
- Container padding: `px-4 md:px-6`
- The homepage currently uses a hardcoded inline `height: calc(100svh - 116px - 117px)` which assumes fixed navbar/footer heights. On mobile, content will overflow this rigid container. Change to `min-height` instead of `height` so content can grow naturally, and use responsive values or remove the calc entirely on mobile in favor of `min-h-svh` with auto-sizing.

**BirdScene container:**
- The BirdScene has a tree branch image that is `135rem` (~2160px) wide with `max-width: none`. The existing `overflow-x-hidden` on the homepage container will clip this on mobile, which is acceptable. Verify that `overflow-x-hidden` is present on the page wrapper (it is on the homepage; add to other pages if the BirdScene appears there).
- Scale the BirdScene wrapper on mobile: reduce `h-44 w-36` to smaller values (e.g., `h-32 w-28 md:h-44 md:w-36`).

### About Page (`src/routes/about/+page.svelte`)

**Content grid (2x2):**
- Grid: `grid-cols-1 md:grid-cols-2`
- Gap: `gap-8 md:gap-16`
- Padding: `pt-8 md:pt-16`, `pb-12 md:pb-24`
- Section margins: `mb-8 md:mb-16`

**Personal note:** Already `max-w-2xl` centered — no changes needed.

### Download Page (`src/routes/download/+page.svelte`)

**Platform grid (Windows / macOS / Linux):**
- Grid: `grid-cols-1 sm:grid-cols-3`
- Gap: `gap-6 sm:gap-12`
- Each platform stacks vertically on mobile
- Margins: `mb-6 sm:mb-12`

**Header bird:**
- Negative margin: `-mt-4 sm:-mt-8`
- Bottom margin: `mb-6 sm:mb-12`

### Footer (`src/routes/+layout.svelte`)

- Padding: `px-4 md:px-6`
- Text already centered — minimal changes needed.

## Files to Modify

1. `src/lib/components/Navbar.svelte`
2. `src/routes/+page.svelte`
3. `src/routes/about/+page.svelte`
4. `src/routes/download/+page.svelte`
5. `src/routes/+layout.svelte`

## Out of Scope

- **App page** (`src/routes/app/+page.svelte`) — Tauri desktop only, no mobile users
- **Settings/History panels** — desktop app only
- **BirdScene internals** — animation offsets and bubble positions stay as-is; the container sizing and bird image dimensions get responsive variants. The wide branch image will be clipped by `overflow-x-hidden` on mobile, which is acceptable.
- Hamburger menu — user prefers visible links that shrink to fit
