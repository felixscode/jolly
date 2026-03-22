# Vulkan Inference Backend Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace mistral.rs with llama-cpp-2 to enable universal GPU inference via Vulkan on Windows/Linux, keeping Metal on macOS.

**Architecture:** Swap the `mistralrs` crate for `llama-cpp-2` (Rust FFI bindings to llama.cpp). Only `local.rs` internals and `Cargo.toml` change. The `LLMProvider` trait boundary, all other providers, commands, frontend, and model files remain untouched.

**Tech Stack:** Rust, llama-cpp-2 0.1.139, Vulkan (Windows/Linux), Metal (macOS), Tauri 2

**Spec:** `docs/superpowers/specs/2026-03-22-vulkan-inference-backend-design.md`

---

## File Structure

| File | Action | Responsibility |
|------|--------|---------------|
| `src-tauri/Cargo.toml` | Modify | Swap dependency, update features |
| `src-tauri/src/inference/local.rs` | Rewrite | Model loading + inference via llama-cpp-2 |
| `.github/workflows/release-app.yml` | Modify | Add Vulkan SDK, pass `--features` flags |

All other files are untouched.

---

### Task 1: Update Cargo.toml Dependencies

**Files:**
- Modify: `src-tauri/Cargo.toml`

- [ ] **Step 1: Replace mistralrs with llama-cpp-2 and update features**

In `src-tauri/Cargo.toml`, make these changes:

Replace the `mistralrs` dependency (line 37):
```toml
# OLD:
mistralrs = { version = "0.7" }

# NEW:
llama-cpp-2 = { version = "0.1.139", features = ["sampler"] }
```

Replace the `[features]` section (lines 46-48):
```toml
# OLD:
[features]
metal = ["mistralrs/metal"]
cuda = ["mistralrs/cuda"]

# NEW:
[features]
metal = ["llama-cpp-2/metal"]
vulkan = ["llama-cpp-2/vulkan"]
```

- [ ] **Step 2: Verify the dependency resolves**

Run from `src-tauri/`:
```bash
cargo check 2>&1 | head -50
```
Expected: Compilation errors in `local.rs` and `benchmark.rs` (they still reference `mistralrs` types), but **no dependency resolution errors**. The `llama-cpp-2` crate should download and compile its `llama-cpp-sys-2` build script (which compiles llama.cpp from source via CMake).

Note: This step requires CMake to be installed. If `cmake` is not found, install it:
- macOS: `brew install cmake`
- Linux: `sudo apt-get install cmake`
- Windows: Install from cmake.org or `winget install cmake`

- [ ] **Step 3: Commit**

```bash
git add src-tauri/Cargo.toml
git commit -m "build: replace mistralrs with llama-cpp-2 for Vulkan support"
```

---

### Task 2: Rewrite local.rs Model Loading

**Files:**
- Modify: `src-tauri/src/inference/local.rs:1-121`

**Important:** Lines 16-23 (`SENTENCES_PER_BATCH`, `SENTENCE_RE`), lines 122-215 (`split_sentences`, `LocalProvider`, `LLMProvider` impl), and lines 262-323 (unit tests) are **preserved unchanged**. Only the imports/statics (lines 1-15) and model loading/inference functions (lines 24-121, 216-260) are replaced.

- [ ] **Step 1: Replace imports and statics**

Replace lines 1-15 of `local.rs` with:

```rust
use std::num::NonZeroU32;
use std::path::Path;
use std::sync::{LazyLock, RwLock};

use regex::Regex;

use async_trait::async_trait;
use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::model::{AddBos, LlamaChatMessage, LlamaChatTemplate, LlamaModel};
use llama_cpp_2::sampling::LlamaSampler;

use super::LLMProvider;
use super::SYSTEM_PROMPT;

/// Llama.cpp backend handle. Initialized once on first use.
static BACKEND: LazyLock<LlamaBackend> = LazyLock::new(|| {
    LlamaBackend::init().expect("Failed to initialize llama backend")
});

/// Wrapper around a loaded LlamaModel.
struct LoadedModel {
    model: LlamaModel,
}

// SAFETY: LlamaModel is internally a pointer to a C struct that llama.cpp
// documents as thread-safe for concurrent reads (inference). All mutation
// (load/swap) goes through the RwLock write guard.
unsafe impl Send for LoadedModel {}
unsafe impl Sync for LoadedModel {}

/// Global model handle. RwLock so models can be swapped at runtime.
static MODEL: RwLock<Option<LoadedModel>> = RwLock::new(None);
```

- [ ] **Step 2: Rewrite swap_model and related functions**

Replace the current `init_model`, `swap_model`, `unload_model`, and `is_model_loaded` functions (lines 24-121) with:

```rust
/// Initialize and load a GGUF model from the given path.
/// Call this once during app startup. For subsequent model changes use `swap_model`.
pub fn init_model(model_path: &Path) -> Result<(), String> {
    swap_model(model_path)
}

/// Load a new model, replacing any currently loaded model.
/// Unloads the old model first to free memory before loading the new one.
pub fn swap_model(model_path: &Path) -> Result<(), String> {
    // Force backend initialization
    let backend = &*BACKEND;

    // Drop the old model first to free VRAM
    {
        let mut slot = MODEL.write().map_err(|e| format!("Model lock poisoned: {}", e))?;
        *slot = None;
    }

    // Try GPU first (n_gpu_layers = 999 offloads all layers to Vulkan/Metal)
    let model = {
        let params = LlamaModelParams::default().with_n_gpu_layers(999);
        match LlamaModel::load_from_file(backend, model_path, &params) {
            Ok(m) => {
                eprintln!("[jolly] Model loaded with GPU acceleration (Vulkan/Metal)");
                m
            }
            Err(e) => {
                eprintln!("[jolly] GPU init failed: {e}");
                eprintln!("[jolly] Falling back to CPU inference");
                let cpu_params = LlamaModelParams::default().with_n_gpu_layers(0);
                LlamaModel::load_from_file(backend, model_path, &cpu_params)
                    .map_err(|e| format!("Failed to load model on CPU: {}", e))?
            }
        }
    };

    let mut slot = MODEL.write().map_err(|e| format!("Model lock poisoned: {}", e))?;
    *slot = Some(LoadedModel { model });

    Ok(())
}

/// Unload the current model, freeing its memory.
pub fn unload_model() {
    if let Ok(mut slot) = MODEL.write() {
        *slot = None;
    }
}

/// Check if a model is loaded and ready for inference.
pub fn is_model_loaded() -> bool {
    MODEL.read().map(|m| m.is_some()).unwrap_or(false)
}
```

- [ ] **Step 3: Verify model loading compiles**

Run from `src-tauri/`:
```bash
cargo check 2>&1 | head -50
```
Expected: Errors only in `run_inference` function (not yet rewritten) and `benchmark.rs`. The model loading functions should compile cleanly.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/inference/local.rs
git commit -m "refactor: rewrite model loading to use llama-cpp-2"
```

---

### Task 3: Rewrite local.rs Inference

**Files:**
- Modify: `src-tauri/src/inference/local.rs:216-260` (the `run_inference` function)

- [ ] **Step 1: Rewrite run_inference**

Replace the current `run_inference` function (lines 216-260) with:

```rust
/// Runs inference synchronously. Called inside spawn_blocking.
pub fn run_inference(text: &str) -> Result<String, String> {
    eprintln!("[jolly] run_inference called with: {:?}", text);

    let backend = &*BACKEND;

    let model_guard = MODEL
        .read()
        .map_err(|e| format!("Model lock poisoned: {}", e))?;
    let loaded = model_guard
        .as_ref()
        .ok_or("Local model not loaded. Download a model in Settings.")?;

    // Format prompt using the model's built-in chat template.
    // This ensures correct prompt formatting for each model architecture.
    let prompt = format!("{}\n\n{}", SYSTEM_PROMPT, text);
    let formatted = match loaded.model.chat_template(None) {
        Ok(tmpl) => {
            let msg = LlamaChatMessage::new(
                "user".to_string(),
                prompt.clone(),
            ).map_err(|e| format!("Failed to create chat message: {}", e))?;
            loaded
                .model
                .apply_chat_template(&tmpl, &[msg], true)
                .map_err(|e| format!("Failed to apply chat template: {}", e))?
        }
        Err(_) => {
            // No chat template in model metadata — use raw prompt
            eprintln!("[jolly] No chat template found, using raw prompt");
            prompt
        }
    };

    // Create context for this inference call
    let ctx_params = LlamaContextParams::default()
        .with_n_ctx(Some(NonZeroU32::new(2048).unwrap()));
    let mut ctx = loaded
        .model
        .new_context(backend, ctx_params)
        .map_err(|e| format!("Failed to create context: {}", e))?;

    // Tokenize
    let tokens = loaded
        .model
        .str_to_token(&formatted, AddBos::Always)
        .map_err(|e| format!("Failed to tokenize: {}", e))?;

    eprintln!("[jolly] Input tokens: {}", tokens.len());

    // Feed tokens into context (batch size must accommodate full prompt)
    let mut batch = LlamaBatch::new(tokens.len().max(512), 1);
    let last_index = (tokens.len() - 1) as i32;
    for (i, token) in (0_i32..).zip(tokens.iter()) {
        let is_last = i == last_index;
        batch
            .add(*token, i, &[0], is_last)
            .map_err(|e| format!("Failed to add token to batch: {}", e))?;
    }
    ctx.decode(&mut batch)
        .map_err(|e| format!("Failed to decode prompt: {}", e))?;

    // Sample tokens until EOS or max length
    let mut sampler = LlamaSampler::chain_simple([
        LlamaSampler::temp(0.1),
        LlamaSampler::top_p(0.9, 1),
        LlamaSampler::greedy(),
    ]);

    let max_tokens: i32 = 1024;
    let mut n_cur = tokens.len() as i32;
    let mut output = String::new();

    for _ in 0..max_tokens {
        let token = sampler.sample(&ctx, batch.n_tokens() - 1);
        sampler.accept(token);

        if loaded.model.is_eog_token(token) {
            break;
        }

        // Detokenize this token
        let piece = loaded
            .model
            .token_to_str(token, llama_cpp_2::model::Special::Tokenize)
            .map_err(|e| format!("Failed to detokenize: {}", e))?;
        output.push_str(&piece);

        // Feed token back for next iteration
        batch.clear();
        batch
            .add(token, n_cur, &[0], true)
            .map_err(|e| format!("Failed to add token to batch: {}", e))?;
        ctx.decode(&mut batch)
            .map_err(|e| format!("Failed to decode: {}", e))?;
        n_cur += 1;
    }

    let result = output.trim().to_string();
    eprintln!("[jolly] Returning: {:?}", result);
    Ok(result)
}
```

**Note on detokenization:** `token_to_str` may be deprecated in favor of `token_to_piece` in some llama-cpp-2 versions. If so, switch to:
```rust
let piece = loaded.model.token_to_piece(token, &mut decoder, true, None)?;
```
This requires adding `encoding_rs = "0.8"` to `Cargo.toml` and `let mut decoder = encoding_rs::UTF_8.new_decoder();` before the loop. The compiler will guide exact fixes.

**Note on `sampler` feature:** The spec's Cargo.toml does not include `features = ["sampler"]`, but the `LlamaSampler` API requires it. This is a deliberate deviation from the spec.

- [ ] **Step 2: Verify the full file compiles**

Run from `src-tauri/`:
```bash
cargo check 2>&1 | head -50
```
Expected: May have warnings about unused imports or deprecated methods. Fix any compilation errors. The `encoding_rs` line can be removed if `token_to_str` works without a decoder.

**Likely adjustments needed:**
- If `LlamaChatMessage::new` has a different signature, adapt (e.g., it may take `String` or `&str`)
- If `token_to_str` is deprecated, switch to `token_to_piece(token, &mut decoder, true, None)`
  - This requires adding `encoding_rs = "0.8"` to `Cargo.toml` dependencies
- If `Special` enum is at a different path, fix the import
- If `with_n_gpu_layers` takes `u32` instead of `i32`, cast accordingly

These are API details that vary across llama-cpp-2 versions. The compiler will guide exact fixes.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/inference/local.rs
git commit -m "feat: rewrite inference to use llama-cpp-2 with Vulkan/Metal support"
```

---

### Task 4: Fix Compilation Errors and Verify Build

**Files:**
- Modify: `src-tauri/src/inference/local.rs` (as needed)
- Modify: `src-tauri/Cargo.toml` (if new deps needed like `encoding_rs`)

- [ ] **Step 1: Run full cargo check and fix all errors**

Run from `src-tauri/`:
```bash
cargo check 2>&1
```

Fix errors iteratively. Common issues:
1. **Deprecated `token_to_str`**: Switch to `token_to_piece`. Add `encoding_rs = "0.8"` to Cargo.toml and use `model.token_to_piece(token, &mut decoder, true, None)?` where `decoder` is `encoding_rs::UTF_8.new_decoder()`.
2. **`Special` enum path**: May be `llama_cpp_2::model::Special` or similar. Check docs.
3. **`LlamaChatMessage::new` signature**: May require `CString` instead of `String`. Adjust.
4. **Lifetime issues with `LoadedModel`**: If `LlamaModel` has lifetime params, the `RwLock<Option<LoadedModel>>` pattern may need `'static`. The `Send`/`Sync` impls already handle this.
5. **`with_n_gpu_layers` type**: May take `u32`. Use `999_u32`.

- [ ] **Step 2: Run cargo build (not just check)**

```bash
cargo build 2>&1 | tail -20
```
Expected: Clean build. This compiles llama.cpp from source (slow first time, ~2-5 minutes).

- [ ] **Step 3: Run existing unit tests**

```bash
cargo test 2>&1
```
Expected: All `split_sentences` tests pass (they don't touch inference). Any inference-related tests that require a model file will be skipped or will need model files.

- [ ] **Step 4: Commit any fixes**

```bash
git add src-tauri/
git commit -m "fix: resolve llama-cpp-2 API compatibility issues"
```

---

### Task 5: Update Benchmark Binary

**Files:**
- Modify: `src-tauri/src/bin/benchmark.rs` (if needed)

- [ ] **Step 1: Check if benchmark compiles**

The benchmark uses `local::init_model`, `local::run_inference`, and `local::unload_model` — all public API with unchanged signatures. It should compile without changes.

```bash
cargo check --bin benchmark 2>&1
```
Expected: Clean compilation. If there are errors, they are likely from removed `tokio` runtime dependencies that `local.rs` no longer needs internally.

- [ ] **Step 2: Fix any issues if present**

The benchmark's own tokio runtime (`#[tokio::main]`) is independent of local.rs internals. No changes expected.

- [ ] **Step 3: Commit if changes needed**

```bash
git add src-tauri/src/bin/benchmark.rs
git commit -m "fix: update benchmark for llama-cpp-2 compatibility"
```

---

### Task 6: Update CI Workflows

**Files:**
- Modify: `.github/workflows/release-app.yml`

**Note:** llama-cpp-sys-2 compiles llama.cpp from source and requires CMake. GitHub Actions runners (`ubuntu-22.04`, `macos-latest`, `windows-latest`) all have CMake pre-installed. No additional CI setup needed for CMake.

**Note:** The `humbletim/setup-vulkan-sdk` GitHub Action should be verified to exist and support the specified version. If unavailable, use direct LunarG SDK download as a fallback.

- [ ] **Step 1: Update macOS build to pass Metal feature**

In `release-app.yml`, replace line 55:
```yaml
# OLD:
      - run: npx tauri build

# NEW:
      - run: npx tauri build --features metal
```

- [ ] **Step 2: Update Windows build to install Vulkan SDK and pass feature**

In `release-app.yml`, after the `rust-cache` step (line 84) and before the build step (line 86), add:

```yaml
      - name: Install Vulkan SDK
        uses: humbletim/setup-vulkan-sdk@v1.2.0
        with:
          vulkan-query-version: 1.3.290.0
          vulkan-components: Vulkan-Headers, Vulkan-Loader
          vulkan-use-cache: true

      - run: npx tauri build --features vulkan
```

And remove the old `- run: npx tauri build` line.

- [ ] **Step 3: Update Linux build to install Vulkan dev libraries and pass feature**

In `release-app.yml`, add `libvulkan-dev` to the existing apt-get install line (around line 126-133):

```yaml
      - name: Install system dependencies
        run: |
          sudo apt-get update
          sudo apt-get install -y \
            libgtk-3-dev \
            libwebkit2gtk-4.1-dev \
            libjavascriptcoregtk-4.1-dev \
            libsoup-3.0-dev \
            libappindicator3-dev \
            librsvg2-dev \
            libssl-dev \
            libvulkan-dev \
            patchelf
```

And replace line 135:
```yaml
# OLD:
      - run: npx tauri build

# NEW:
      - run: npx tauri build --features vulkan
```

- [ ] **Step 4: Commit**

```bash
git add .github/workflows/release-app.yml
git commit -m "ci: add Vulkan SDK and GPU feature flags to release builds"
```

---

### Task 7: Manual Smoke Test

This task validates the full flow end-to-end. It requires a downloaded GGUF model.

- [ ] **Step 1: Run the app locally**

From the project root:
```bash
# On Linux/Windows:
npx tauri dev --features vulkan

# On macOS:
npx tauri dev --features metal
```

- [ ] **Step 2: Test correction flow**

1. Open Jolly
2. Ensure a model is downloaded (or download one from Settings)
3. Activate the model
4. Copy some text with spelling errors to clipboard
5. Press Enter to correct
6. Verify corrected text appears

Check terminal output for:
- `[jolly] Model loaded with GPU acceleration (Vulkan/Metal)` — confirms GPU is being used
- `[jolly] run_inference called with: ...` — confirms inference is running
- `[jolly] Returning: ...` — confirms output

- [ ] **Step 3: Test CPU fallback**

If possible, test on a machine without Vulkan drivers or set `n_gpu_layers = 0` temporarily. Verify:
- `[jolly] GPU init failed: ...` appears
- `[jolly] Falling back to CPU inference` appears
- Correction still works (just slower)

- [ ] **Step 4: Run benchmark (optional)**

```bash
cd src-tauri && cargo run --bin benchmark --features vulkan
```

Compare results against previous `benchmark_results.csv` to verify output quality is similar.

---

### Task 8: Final Cleanup and Commit

- [ ] **Step 1: Check for unused dependencies**

Note: `tokio-util` is used by `download.rs` (`CancellationToken`) — do NOT remove it. Check if any other deps became unused after removing `mistralrs` (unlikely, but verify with `cargo check`).

- [ ] **Step 2: Run clippy**

```bash
cargo clippy 2>&1
```

Fix any warnings related to the new code.

- [ ] **Step 3: Final commit**

```bash
git add -A
git commit -m "chore: clean up unused deps after llama-cpp-2 migration"
```
