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
- Min-height adjusted to avoid overflow on small screens

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
- **BirdScene internals** — animation offsets stay as-is; only the container sizing changes
- Hamburger menu — user prefers visible links that shrink to fit
