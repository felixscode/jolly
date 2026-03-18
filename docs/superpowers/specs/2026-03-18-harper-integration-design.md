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

1. Convert input `&str` to `Vec<char>` (Harper operates on character indices via `Span<char>`, not byte offsets)
2. Parse text with `PlainEnglish` parser into a `Document`
3. Create curated `FstDictionary` and `LintGroup` (American dialect)
4. Run linting to get `Vec<Lint>`
5. Sort lints by span position in reverse order
6. For each lint: skip if `suggestions` is empty, otherwise call `suggestion.apply(lint.span, &mut chars)` with the first suggestion (Harper's `Suggestion::apply` handles splice/replace/remove internally)
7. Collect `Vec<char>` back to `String` and return
8. If zero lints are found, return the original text unchanged (note: unlike LLM providers which may normalize text, Harper only modifies text when it finds actual errors)

Since Harper is synchronous and `LLMProvider::correct_text` is async, the work is wrapped in `tokio::task::spawn_blocking`.

#### Dependency: `harper-core`

```toml
harper-core = "1"
```

Default features are kept (includes thesaurus support). The `concurrent` feature is not in the default set but should not be needed since we run Harper inside `spawn_blocking` rather than across async boundaries. Ships with an embedded FST dictionary — no runtime downloads needed. Adds a few MB to binary size.

#### Dispatch logic update: `src-tauri/src/commands.rs`

New priority order in `correct_text()`:

1. `useHarper == true` → `HarperProvider` (even if `useOpenRouter` is also true — Harper wins as highest priority, defensive against store corruption)
2. Local model loaded AND `useOpenRouter == false` → `LocalProvider`
3. Model selected but not loaded → error `"model_not_loaded"`
4. `useOpenRouter == true` → `OpenRouterProvider` (requires API key)
5. None of the above → error `"no_provider_configured"`

New helper `get_use_harper(app) -> bool` reads `"useHarper"` from the settings store.

### Frontend

#### Settings store: `src/lib/stores/settings.svelte.ts`

- New `useHarper: boolean` state, persisted to `settings.json`
- New `setUseHarper(value: boolean)` method
- Mutual exclusion: `setUseHarper(true)` also sets `useOpenRouter = false`; `setUseOpenRouter(true)` also sets `useHarper = false`
- `loadAll()` must hydrate `useHarper` from the store on app start (same pattern as `useOpenRouter` at line 48)

#### Settings UI: `src/lib/components/Settings.svelte`

New section between "Downloaded Models" and "OpenRouter":

- Section title: "Harper (Lightweight)"
- Toggle switch (same style as OpenRouter)
- Description: "Fast grammar & spelling correction. No downloads, no API key — works instantly. English only."
- When Harper is active, the "Downloaded Models" section shows a "Disabled while Harper is active" message instead of the model radio buttons (matching the existing pattern used when OpenRouter is active — this is a conditional hide, not a grey-out)

## Limitations

- English only (Harper limitation)
- Rule-based — won't catch style, tone, or complex rewrite issues the way LLMs do
- Auto-applies first suggestion per lint, which may not always be the best fix (but is the highest-confidence one)
- Some lints may have zero suggestions — these are skipped (the error is detected but no automatic fix is available)
- Unlike LLM providers which may normalize or rephrase text, Harper returns the original text unchanged when no errors are found

## Files Changed

| File | Change |
|------|--------|
| `src-tauri/Cargo.toml` | Add `harper-core` dependency |
| `src-tauri/src/inference/mod.rs` | Add `pub mod harper;` |
| `src-tauri/src/inference/harper.rs` | New `HarperProvider` implementing `LLMProvider` |
| `src-tauri/src/commands.rs` | Add Harper dispatch branch + `get_use_harper` helper |
| `src/lib/stores/settings.svelte.ts` | Add `useHarper` state + `setUseHarper` method + mutual exclusion |
| `src/lib/components/Settings.svelte` | Add Harper toggle section |
