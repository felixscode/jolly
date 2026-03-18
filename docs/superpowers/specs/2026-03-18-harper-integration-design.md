# Harper Integration Design

**Date:** 2026-03-18
**Status:** Approved

## Overview

Add [Harper](https://github.com/Automattic/harper) as a standalone lightweight grammar/spelling correction provider in Jolly. Harper is an offline, Rust-native grammar checker that requires no model download and no API key — it works instantly using an embedded dictionary.

## Motivation

Currently Jolly offers two correction providers: local LLM inference (requires downloading a 1–4 GB model) and OpenRouter (requires an API key + network). Harper fills a gap as a zero-setup option for users who want fast, private spelling and grammar correction without the overhead of either existing provider.

## Design Decisions

- **Harper as a separate toggle**, not a model in the dropdown — it's a fundamentally different kind of provider (rule-based vs. generative) and deserves its own UI treatment
- **Mutual exclusion with OpenRouter** — toggling Harper on disables OpenRouter and vice versa, keeping dispatch logic simple and predictable
- **Harper takes highest dispatch priority** when enabled — it's the lightest option (no model loading, no network)
- **No visual distinction** between Harper and LLM corrections — corrected text replaces clipboard the same way regardless of provider
- **American English dialect hardcoded** for now — can be made configurable later
- **Auto-apply first suggestion** from each lint — highest confidence fix, applied in reverse span order to preserve offsets

## Architecture

### Backend

#### New file: `src-tauri/src/inference/harper.rs`

`HarperProvider` struct implementing the existing `LLMProvider` trait:

1. Parse input text with `PlainEnglish` parser
2. Create curated `FstDictionary` and `LintGroup` (American dialect)
3. Run linting to get `Vec<Lint>`
4. Sort lints by span position in reverse order
5. Apply first suggestion from each lint back-to-front (preserves character offsets)
6. Return corrected string

Since Harper is synchronous and `LLMProvider::correct_text` is async, the work is wrapped in `tokio::task::spawn_blocking`.

#### Dependency: `harper-core`

```toml
harper-core = { version = "1", default-features = false }
```

Ships with an embedded FST dictionary — no runtime downloads needed. Adds a few MB to binary size.

#### Dispatch logic update: `src-tauri/src/commands.rs`

New priority order in `correct_text()`:

1. `useHarper == true` → `HarperProvider`
2. Local model loaded AND `useOpenRouter == false` → `LocalProvider`
3. Model selected but not loaded → error
4. Otherwise → `OpenRouterProvider`

New helper `get_use_harper(app) -> bool` reads `"useHarper"` from the settings store.

### Frontend

#### Settings store: `src/lib/stores/settings.svelte.ts`

- New `useHarper: boolean` state, persisted to `settings.json`
- New `setUseHarper(value: boolean)` method
- Mutual exclusion: `setUseHarper(true)` also sets `useOpenRouter = false`; `setUseOpenRouter(true)` also sets `useHarper = false`

#### Settings UI: `src/lib/components/Settings.svelte`

New section between "Downloaded Models" and "OpenRouter":

- Section title: "Harper (Lightweight)"
- Toggle switch (same style as OpenRouter)
- Description: "Fast grammar & spelling correction. No downloads, no API key — works instantly. English only."
- When Harper is active, "Downloaded Models" radio buttons are greyed out (same pattern as when OpenRouter is active)

## Limitations

- English only (Harper limitation)
- Rule-based — won't catch style, tone, or complex rewrite issues the way LLMs do
- Auto-applies first suggestion per lint, which may not always be the best fix (but is the highest-confidence one)

## Files Changed

| File | Change |
|------|--------|
| `src-tauri/Cargo.toml` | Add `harper-core` dependency |
| `src-tauri/src/inference/mod.rs` | Add `pub mod harper;` |
| `src-tauri/src/inference/harper.rs` | New `HarperProvider` implementing `LLMProvider` |
| `src-tauri/src/commands.rs` | Add Harper dispatch branch + `get_use_harper` helper |
| `src/lib/stores/settings.svelte.ts` | Add `useHarper` state + `setUseHarper` method + mutual exclusion |
| `src/lib/components/Settings.svelte` | Add Harper toggle section |
