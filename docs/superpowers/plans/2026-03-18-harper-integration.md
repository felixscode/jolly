# Harper Integration Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add Harper as a lightweight, zero-setup grammar/spelling correction provider alongside the existing LLM-based providers.

**Architecture:** New `HarperProvider` implements the existing `LLMProvider` trait using `harper-core` for offline rule-based correction. A `useHarper` toggle in Settings enables it with mutual exclusion against OpenRouter. Backend dispatch prioritizes Harper when enabled.

**Tech Stack:** `harper-core` (Rust crate), SvelteKit 5, Tauri 2

**Spec:** `docs/superpowers/specs/2026-03-18-harper-integration-design.md`

---

## Chunk 1: Backend — HarperProvider

### Task 1: Add `harper-core` dependency

**Files:**
- Modify: `src-tauri/Cargo.toml`

- [ ] **Step 1: Add dependency**

Add to the `[dependencies]` section of `src-tauri/Cargo.toml`:

```toml
harper-core = "1"
```

- [ ] **Step 2: Verify it compiles**

Run: `cd src-tauri && cargo check`
Expected: compiles successfully (may take a while to download/build harper-core)

- [ ] **Step 3: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/Cargo.lock
git commit -m "deps: add harper-core for lightweight grammar checking"
```

---

### Task 2: Write failing tests for HarperProvider

**Files:**
- Create: `src-tauri/src/inference/harper.rs`

- [ ] **Step 1: Create harper.rs with tests only**

Create `src-tauri/src/inference/harper.rs`:

```rust
use async_trait::async_trait;

use super::LLMProvider;

pub struct HarperProvider;

impl HarperProvider {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl LLMProvider for HarperProvider {
    async fn correct_text(&self, text: &str) -> Result<String, String> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn empty_input_returns_empty() {
        let provider = HarperProvider::new();
        let result = provider.correct_text("").await.unwrap();
        assert_eq!(result, "");
    }

    #[tokio::test]
    async fn correct_text_unchanged() {
        let provider = HarperProvider::new();
        let result = provider.correct_text("This is correct.").await.unwrap();
        assert_eq!(result, "This is correct.");
    }

    #[tokio::test]
    async fn fixes_spelling_error() {
        let provider = HarperProvider::new();
        let result = provider.correct_text("This is an tset.").await.unwrap();
        // Harper should fix "tset" — exact correction depends on Harper's dictionary,
        // so we just check it changed
        assert_ne!(result, "This is an tset.");
    }

    #[tokio::test]
    async fn preserves_multiline_text() {
        let provider = HarperProvider::new();
        let input = "First line.\nSecond line.";
        let result = provider.correct_text(input).await.unwrap();
        // No errors, should be unchanged
        assert_eq!(result, input);
    }
}
```

- [ ] **Step 2: Register the module**

Add to `src-tauri/src/inference/mod.rs`, after the existing module declarations:

```rust
pub mod harper;
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `cd src-tauri && cargo test --lib inference::harper`
Expected: FAIL — `todo!()` panics

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/inference/harper.rs src-tauri/src/inference/mod.rs
git commit -m "test: add failing tests for HarperProvider"
```

---

### Task 3: Implement HarperProvider

**Files:**
- Modify: `src-tauri/src/inference/harper.rs`

- [ ] **Step 1: Replace the `todo!()` implementation**

Replace the full contents of `src-tauri/src/inference/harper.rs` with:

```rust
use std::sync::Arc;

use async_trait::async_trait;
use harper_core::linting::{LintGroup, Linter};
use harper_core::{Document, FstDictionary};

use super::LLMProvider;

pub struct HarperProvider;

impl HarperProvider {
    pub fn new() -> Self {
        Self
    }
}

/// Run Harper grammar/spelling check and auto-apply first suggestion for each lint.
fn harper_correct(text: &str) -> Result<String, String> {
    if text.is_empty() {
        return Ok(String::new());
    }

    let dict = FstDictionary::curated();
    let document = Document::new_plain_english(text, &dict);

    let mut linter = LintGroup::new_curated(Arc::clone(&dict), harper_core::Dialect::American);
    let mut lints = linter.lint(&document);

    if lints.is_empty() {
        return Ok(text.to_string());
    }

    // Sort by span start descending so we apply from back to front,
    // preserving character offsets for earlier spans
    lints.sort_by(|a, b| b.span.start.cmp(&a.span.start));

    let mut chars: Vec<char> = text.chars().collect();

    for lint in &lints {
        if let Some(suggestion) = lint.suggestions.first() {
            suggestion.apply(lint.span, &mut chars);
        }
    }

    Ok(chars.into_iter().collect())
}

#[async_trait]
impl LLMProvider for HarperProvider {
    async fn correct_text(&self, text: &str) -> Result<String, String> {
        let text = text.to_string();
        tokio::task::spawn_blocking(move || harper_correct(&text))
            .await
            .map_err(|e| format!("Harper task failed: {}", e))?
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn empty_input_returns_empty() {
        let provider = HarperProvider::new();
        let result = provider.correct_text("").await.unwrap();
        assert_eq!(result, "");
    }

    #[tokio::test]
    async fn correct_text_unchanged() {
        let provider = HarperProvider::new();
        let result = provider.correct_text("This is correct.").await.unwrap();
        assert_eq!(result, "This is correct.");
    }

    #[tokio::test]
    async fn fixes_spelling_error() {
        let provider = HarperProvider::new();
        let result = provider.correct_text("This is an tset.").await.unwrap();
        // Harper should fix "tset" — exact correction depends on Harper's dictionary,
        // so we just check it changed
        assert_ne!(result, "This is an tset.");
    }

    #[tokio::test]
    async fn preserves_multiline_text() {
        let provider = HarperProvider::new();
        let input = "First line.\nSecond line.";
        let result = provider.correct_text(input).await.unwrap();
        // No errors, should be unchanged
        assert_eq!(result, input);
    }

    #[test]
    fn harper_correct_empty() {
        assert_eq!(harper_correct("").unwrap(), "");
    }

    #[test]
    fn harper_correct_no_errors() {
        let result = harper_correct("The cat sat on the mat.").unwrap();
        assert_eq!(result, "The cat sat on the mat.");
    }

    #[test]
    fn harper_correct_returns_ok() {
        // Smoke test: Harper doesn't panic on arbitrary input
        let result = harper_correct("somthing is wrng here");
        assert!(result.is_ok());
    }
}
```

- [ ] **Step 2: Run tests to verify they pass**

Run: `cd src-tauri && cargo test --lib inference::harper`
Expected: all tests PASS

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/inference/harper.rs
git commit -m "feat: implement HarperProvider with auto-apply corrections"
```

---

## Chunk 2: Backend — Dispatch Logic

### Task 4: Update dispatch logic in commands.rs

**Files:**
- Modify: `src-tauri/src/commands.rs`

- [ ] **Step 1: Add the `get_use_harper` helper**

Add after the existing `get_use_openrouter` function (line 48) in `src-tauri/src/commands.rs`:

```rust
/// Check if the user has toggled "use Harper" in settings.
fn get_use_harper(app: &AppHandle) -> bool {
    app.store("settings.json")
        .ok()
        .and_then(|store| store.get("useHarper"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}
```

- [ ] **Step 2: Add HarperProvider import**

Add to the imports at the top of `src-tauri/src/commands.rs`:

```rust
use crate::inference::harper::HarperProvider;
```

- [ ] **Step 3: Replace the `correct_text` command body**

Replace the entire `correct_text` function including its `#[tauri::command]` attribute (lines 50–86) with the following. **Note:** This is an intentional behavioral change — the old code fell through to OpenRouter when no provider was configured (resulting in a confusing "no_api_key" error). The new Priority 5 returns an explicit `"no_provider_configured"` error instead.

```rust
#[tauri::command]
pub async fn correct_text(app: AppHandle, text: String) -> Result<String, String> {
    if text.trim().is_empty() {
        return Ok(String::new());
    }

    let use_harper = get_use_harper(&app);
    let use_openrouter = get_use_openrouter(&app);
    let active_model = get_active_model_id(&app);
    let has_local = active_model.is_some() && crate::inference::local::is_model_loaded();

    // Priority 1: Harper (lightweight, instant)
    if use_harper {
        eprintln!("[jolly] Using Harper grammar checker");
        let harper = HarperProvider::new();
        return harper.correct_text(&text).await.map_err(|e| {
            eprintln!("[jolly] Harper error: {}", e);
            "harper_failed".to_string()
        });
    }

    // Priority 2: Local model (loaded and not overridden by OpenRouter)
    if has_local && !use_openrouter {
        eprintln!("[jolly] Using local inference");
        let local = LocalProvider::new();
        return local.correct_text(&text).await.map_err(|e| {
            eprintln!("[jolly] Local inference error: {}", e);
            "local_inference_failed".to_string()
        });
    }

    // Priority 3: Model selected but not loaded
    if !use_openrouter && !has_local && active_model.is_some() {
        return Err("model_not_loaded".to_string());
    }

    // Priority 4: OpenRouter
    if use_openrouter {
        eprintln!("[jolly] Using OpenRouter API");
        let api_key = get_api_key(&app)?;
        let openrouter = OpenRouterProvider::new(api_key);
        return openrouter.correct_text(&text).await.map_err(|e| {
            eprintln!("[jolly] OpenRouter error: {}", e);
            if e.contains("401") || e.contains("403") {
                "bad_api_key".to_string()
            } else {
                "api_failed".to_string()
            }
        });
    }

    // Priority 5: Nothing configured
    Err("no_provider_configured".to_string())
}
```

- [ ] **Step 4: Verify compilation**

Run: `cd src-tauri && cargo check`
Expected: compiles successfully

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/commands.rs
git commit -m "feat: add Harper to provider dispatch logic"
```

---

## Chunk 3: Frontend — Settings Store & UI

### Task 5: Add `useHarper` to the settings store

**Files:**
- Modify: `src/lib/stores/settings.svelte.ts`

- [ ] **Step 1: Add state declaration**

Add after the `useOpenRouter` state declaration (line 23):

```typescript
let useHarper = $state(false);
```

- [ ] **Step 2: Hydrate in `loadAll()`**

In the `loadAll()` function, add after the `useOpenRouter` line (line 48):

```typescript
useHarper = ((await store.get('useHarper')) as boolean | null) ?? false;
```

- [ ] **Step 3: Add `setUseHarper` method with mutual exclusion**

Add after the `setUseOpenRouter` function (after line 167):

```typescript
async function setUseHarper(value: boolean) {
    useHarper = value;
    try {
        const store = await getStore();
        await store.set('useHarper', value);
        // Mutual exclusion: Harper ON → OpenRouter OFF
        if (value && useOpenRouter) {
            useOpenRouter = false;
            await store.set('useOpenRouter', false);
        }
        await store.save();
    } catch (e) {
        console.error('Failed to save Harper preference:', e);
    }
}
```

- [ ] **Step 4: Update `setUseOpenRouter` for mutual exclusion**

In the existing `setUseOpenRouter` function, add after `await store.set('useOpenRouter', value);` (line 162):

```typescript
// Mutual exclusion: OpenRouter ON → Harper OFF
if (value && useHarper) {
    useHarper = false;
    await store.set('useHarper', false);
}
```

- [ ] **Step 5: Expose in the return object**

Add to the return object:

```typescript
get useHarper() {
    return useHarper;
},
```

Add after the `useOpenRouter` getter (line 260). And add `setUseHarper,` to the returned functions list, after `setUseOpenRouter` (line 288).

- [ ] **Step 6: Commit**

```bash
git add src/lib/stores/settings.svelte.ts
git commit -m "feat: add useHarper setting with mutual exclusion"
```

---

### Task 6: Add Harper toggle to Settings UI

**Files:**
- Modify: `src/lib/components/Settings.svelte`

- [ ] **Step 1: Update the "Downloaded Models" conditional**

In `Settings.svelte`, find line 211:

```svelte
{:else if !settings.useOpenRouter}
```

Replace with:

```svelte
{:else if !settings.useOpenRouter && !settings.useHarper}
```

- [ ] **Step 2: Update the disabled message**

Find line 262-264:

```svelte
{:else}
    <p class="mt-2 text-xs text-gray-400 dark:text-[#e8e8e3]/50">
        Disabled while OpenRouter is active
    </p>
```

Replace with:

```svelte
{:else}
    <p class="mt-2 text-xs text-gray-400 dark:text-[#e8e8e3]/50">
        Disabled while {settings.useHarper ? 'Harper' : 'OpenRouter'} is active
    </p>
```

- [ ] **Step 3: Add Harper section**

Add a new `<section>` between the "Downloaded Models" closing `</section>` (after line 271) and the "OpenRouter" `<section>` (line 273):

```svelte
<!-- Section: Harper (Lightweight) -->
<section>
    <div class="flex items-center justify-between">
        <h3 class="text-sm font-bold text-[#423f37] dark:text-[#e8e8e3]">
            Harper (Lightweight)
        </h3>
        <button
            onclick={() => settings.setUseHarper(!settings.useHarper)}
            class="relative inline-flex h-6 w-11 items-center rounded-full transition-colors {settings.useHarper
                ? 'bg-[#960200] dark:bg-[#ffd046]'
                : 'bg-gray-300 dark:bg-white/20'}"
            role="switch"
            aria-checked={settings.useHarper}
            aria-label="Toggle Harper"
        >
            <span
                class="inline-block h-4 w-4 rounded-full bg-white transition-transform {settings.useHarper
                    ? 'translate-x-6'
                    : 'translate-x-1'}"
            ></span>
        </button>
    </div>
    <p class="mt-1 text-xs text-gray-400 dark:text-[#e8e8e3]/50">
        Fast grammar & spelling correction. No downloads, no API key — works instantly. English
        only.
    </p>
</section>
```

- [ ] **Step 4: Verify the dev server builds**

Run: `npm run check`
Expected: no type errors

- [ ] **Step 5: Commit**

```bash
git add src/lib/components/Settings.svelte
git commit -m "feat: add Harper toggle to Settings UI"
```

---

## Chunk 4: Manual Verification

### Task 7: End-to-end verification

- [ ] **Step 1: Start the dev server**

Run: `npm run dev` (or `cargo tauri dev` for the desktop app)

- [ ] **Step 2: Verify Harper toggle appears in Settings**

Open Settings panel. Confirm:
- Harper (Lightweight) section appears between Downloaded Models and OpenRouter
- Toggle switch works
- Toggling Harper ON disables OpenRouter toggle (and vice versa)
- Downloaded Models section shows "Disabled while Harper is active" when Harper is on

- [ ] **Step 3: Test correction with Harper enabled**

1. Toggle Harper ON in Settings
2. Type text with a spelling error (e.g., "Ths is a tset")
3. Press Enter to correct
4. Verify the text is corrected

- [ ] **Step 4: Test that other providers still work**

1. Toggle Harper OFF
2. Verify local model or OpenRouter still works as before

- [ ] **Step 5: Final commit if any fixes were needed**

```bash
git add -A
git commit -m "fix: address issues found during manual verification"
```
