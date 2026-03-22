# Vulkan Inference Backend Design

**Date:** 2026-03-22
**Status:** Draft
**Scope:** Replace mistral.rs with llama-cpp-2 for universal GPU inference via Vulkan

## Problem

Jolly's local LLM inference uses mistral.rs, which only supports CUDA (NVIDIA) and Metal (Apple) as compile-time feature flags. This creates two problems:

1. **CUDA version matrix:** Each CUDA toolkit version requires a separate build, and users must have a matching CUDA runtime installed. Different NVIDIA driver versions support different CUDA versions.
2. **No AMD/Intel GPU support:** Users with AMD or Intel GPUs on Windows/Linux get CPU-only inference with no acceleration.

## Solution

Replace mistral.rs with llama-cpp-2 (Rust FFI bindings to llama.cpp) and use Vulkan as the GPU backend on Windows and Linux. Keep Metal on macOS.

**Vulkan advantages:**
- Ships with every modern GPU driver (NVIDIA, AMD, Intel) — no user-side SDK installation
- One build per OS covers all GPU vendors
- Supports integrated GPUs (Intel UHD, AMD APU) — a net improvement
- Vulkan 1.0 shipped in 2016; any GPU from ~2012+ has driver support
- Performance difference vs native CUDA is negligible for Jolly's use case (small models, short text corrections)

## Architecture

### Backend Selection Per Platform

| Platform | Cargo Feature | GPU API | Fallback |
|----------|--------------|---------|----------|
| macOS    | `metal`      | Metal   | CPU      |
| Windows  | `vulkan`     | Vulkan  | CPU      |
| Linux    | `vulkan`     | Vulkan  | CPU      |

### What Changes

| File | Change |
|------|--------|
| `src-tauri/Cargo.toml` | Replace `mistralrs` dependency with `llama-cpp-2`; update feature flags |
| `src-tauri/src/inference/local.rs` | Rewrite model loading and inference internals (~150-200 lines changed) |
| CI workflows | Replace CUDA SDK with Vulkan SDK on Windows/Linux |

### What Stays the Same

- `LLMProvider` trait and all other providers (Harper, OpenRouter)
- `commands.rs` — calls `init_model()`, `swap_model()`, `is_model_loaded()`, `run_inference()` with unchanged signatures
- `lib.rs` startup flow
- Sentence splitting logic (`split_sentences()`) and all its unit tests
- Model registry, download system, settings store
- All frontend code
- GGUF model files (same format — llama.cpp created the GGUF format)
- Benchmark binary (`src-tauri/src/bin/benchmark.rs`) — uses `run_inference()` public API

## Detailed Design

### Cargo.toml Changes

```toml
# Remove:
mistralrs = { version = "0.7" }

# Add:
llama-cpp-2 = { version = "0.1.139" }

# Note: async-trait dependency stays (LLMProvider trait still uses it),
# but local.rs internals no longer need async — llama-cpp-2 is synchronous C FFI.

# Features become:
[features]
metal = ["llama-cpp-2/metal"]       # macOS
vulkan = ["llama-cpp-2/vulkan"]     # Windows/Linux
```

The `cuda` feature is removed entirely.

### local.rs Rewrite

The public API stays identical — same function signatures, same behavior from the outside. Estimated ~150-200 lines of changed code due to llama-cpp-2's lower-level API (vs ~60 lines of mistral.rs-specific code being replaced).

#### Backend Initialization

llama-cpp-2 requires a one-time `LlamaBackend::init()` call before any model operations. This is stored in a separate static:

```rust
static BACKEND: LazyLock<LlamaBackend> = LazyLock::new(|| {
    LlamaBackend::init().expect("Failed to initialize llama backend")
});
```

#### Static Model Storage

The `MODEL` static changes from `RwLock<Option<mistralrs::Model>>` to a wrapper struct:

```rust
struct LoadedModel {
    model: LlamaModel,
}

static MODEL: RwLock<Option<LoadedModel>> = RwLock::new(None);
```

A new `LlamaContext` is created per-inference call rather than stored, since contexts are lightweight and this avoids lifetime/thread-safety issues with the RwLock.

#### Model Loading (`swap_model`)

```rust
pub fn swap_model(model_path: &Path) -> Result<(), String> {
    // Force backend initialization
    let _backend = &*BACKEND;

    // Drop old model to free VRAM
    { let mut slot = MODEL.write()...; *slot = None; }

    // Try GPU first (n_gpu_layers = 999 offloads all layers)
    let params = LlamaModelParams::default().with_n_gpu_layers(999);
    let model = match LlamaModel::load_from_file(&_backend, model_path, &params) {
        Ok(model) => {
            eprintln!("[jolly] Model loaded with GPU acceleration");
            model
        }
        Err(e) => {
            eprintln!("[jolly] GPU init failed: {e}, falling back to CPU");
            let params = LlamaModelParams::default().with_n_gpu_layers(0);
            LlamaModel::load_from_file(&_backend, model_path, &params)
                .map_err(|e| format!("Failed to load model on CPU: {e}"))?
        }
    };

    // Store the loaded model
    let mut slot = MODEL.write().map_err(|e| format!("Lock poisoned: {e}"))?;
    *slot = Some(LoadedModel { model });
    Ok(())
}
```

**Key difference from current code:** llama-cpp-2 returns `Result` errors on GPU failure rather than panicking (unlike mistral.rs CUDA). The `catch_unwind` wrapper is no longer needed, simplifying the code.

**Key difference from current code:** The temporary tokio runtime hack (`tokio::runtime::Builder::new_current_thread()`) is no longer needed in `swap_model` because llama-cpp-2's model loading is synchronous C FFI, not async. This eliminates a potential footgun (nested runtimes).

#### Inference (`run_inference`)

```rust
pub fn run_inference(text: &str) -> Result<String, String> {
    let model_guard = MODEL.read()...;
    let loaded = model_guard.as_ref().ok_or("Local model not loaded...")?;

    // 1. Format prompt using model's built-in chat template
    //    (pseudocode — adapt to actual llama-cpp-2 ChatTemplateMessage API)
    let prompt = format!("{}\n\n{}", SYSTEM_PROMPT, text);
    let formatted = loaded.model.apply_chat_template(
        None,  // use model's default template from GGUF metadata
        &[ChatTemplateMessage { role: "user", content: &prompt }],
        true,  // add generation prompt
    )?;

    // 2. Create context for this inference call
    let ctx_params = LlamaContextParams::default()
        .with_n_ctx(NonZero::new(2048));  // 2048 tokens — sufficient for short text correction
    let mut ctx = loaded.model.new_context(&BACKEND, ctx_params)?;

    // 3. Tokenize
    let tokens = loaded.model.str_to_token(&formatted, AddBos::Always)?;

    // 4. Feed tokens into context
    let mut batch = LlamaBatch::new(tokens.len(), 1);
    for (i, token) in tokens.iter().enumerate() {
        let is_last = i == tokens.len() - 1;
        batch.add(*token, i as i32, &[0], is_last)?;
    }
    ctx.decode(&mut batch)?;

    // 5. Sample tokens until EOS or max length
    let mut sampler = LlamaSampler::chain_simple([
        LlamaSampler::temp(0.1),        // near-deterministic for correction
        LlamaSampler::top_p(0.9, 1),    // minimal randomness
        LlamaSampler::greedy(),          // pick best token
    ]);

    let max_tokens = 1024;
    let mut output_tokens = Vec::new();

    for _ in 0..max_tokens {
        let token = sampler.sample(&ctx, -1);
        if loaded.model.is_eog_token(token) { break; }
        output_tokens.push(token);

        // Feed token back for next iteration
        batch.clear();
        batch.add(token, tokens.len() as i32 + output_tokens.len() as i32, &[0], true)?;
        ctx.decode(&mut batch)?;
    }

    // 6. Detokenize
    let text = output_tokens.iter()
        .map(|t| loaded.model.token_to_str(*t))
        .collect::<Result<String, _>>()?;

    Ok(text.trim().to_string())
}
```

**Sampling parameters rationale:**
- `temp(0.1)`: Near-deterministic — spelling correction should be consistent, not creative
- `top_p(0.9, 1)`: Slight nucleus sampling as safety net
- `greedy()`: Final selection picks the highest-probability token
- `n_ctx = 2048`: Sufficient for Jolly's use case (short text corrections, typically <500 tokens)
- `max_tokens = 1024`: Hard cap to prevent runaway generation

**Key difference from current code:** The temporary tokio runtime hack in `run_inference` is also removed — the entire function is now synchronous C FFI calls, which is cleaner inside `spawn_blocking`.

#### Preserved Unchanged

- `split_sentences()` function and all unit tests
- `LocalProvider` struct and `LLMProvider` trait implementation
- `SENTENCES_PER_BATCH` batching logic
- `is_model_loaded()`, `unload_model()` functions

### CI/CD Changes

**Current state:** The CI workflows (`release-app.yml`) currently run `npx tauri build` with **no `--features` flags** on any platform. This means current release builds are CPU-only (no CUDA, no Metal). GPU features are only used by developers building locally.

**After this change:** CI will pass feature flags to enable GPU acceleration in release builds.

| Platform | Before | After |
|----------|--------|-------|
| macOS | `npx tauri build` (no GPU) | `npx tauri build -- --features metal` |
| Windows | `npx tauri build` (no GPU) | `npx tauri build -- --features vulkan` |
| Linux | `npx tauri build` (no GPU) | `npx tauri build -- --features vulkan` |

**CI dependency installation:**
- Windows: Install LunarG Vulkan SDK (GitHub Action or direct download)
- Linux: Add `libvulkan-dev` to existing `apt-get install` list
- macOS: No change (Metal frameworks are built into Xcode)

**Vulkan runtime linking:** llama-cpp-2 links Vulkan dynamically (`libvulkan.so.1` on Linux, `vulkan-1.dll` on Windows). These libraries ship with GPU drivers and are present on any system with a Vulkan-capable GPU. If the Vulkan runtime is missing at startup (e.g., headless server), the GPU backend simply fails to initialize and falls back to CPU — no crash.

## Error Handling & Edge Cases

**GPU not available (no Vulkan drivers):**
- Try GPU → catch failure → fall back to CPU
- Log warning, user sees no error — just slower inference

**Integrated GPUs (Intel UHD, AMD APU):**
- Vulkan works on integrated GPUs — users with no discrete GPU still get acceleration
- Net improvement over mistral.rs which ignores integrated GPUs

**Old GPUs without Vulkan support:**
- Rare — any GPU from ~2012+ has Vulkan drivers
- Falls back to CPU gracefully

**Model compatibility:**
- GGUF format is identical between mistral.rs and llama.cpp
- All 5 registry models and custom imports work unchanged
- Chat templates are embedded in GGUF metadata — llama-cpp-2 reads them automatically

## Testing Strategy

**Unit tests:**
- `split_sentences` tests — unchanged
- Add test verifying GPU fallback: failed GPU load → CPU path taken

**Integration tests:**
- Load a small GGUF model, run correction, verify non-empty output
- Verify `swap_model`: load model A → swap to model B → run inference
- Verify `unload_model` frees the slot

**CI validation:**
- All platforms: build with respective feature flag, run tests on CPU (CI runners have no GPU, but CPU fallback path is exercised)

**Manual testing checklist:**
- NVIDIA GPU (Vulkan) — Windows and Linux
- AMD GPU (Vulkan) — Windows and Linux
- Intel integrated GPU (Vulkan) — Linux
- Apple Silicon (Metal) — macOS
- No GPU (CPU fallback)
- Model download → activate → correct end-to-end flow
- Custom model import

## Crate Selection: llama-cpp-2 (utilityai)

**Why this crate:**
- Most actively maintained llama.cpp Rust binding (last commit March 2026, 502 stars, 71 contributors, 139 releases)
- Tracks upstream llama.cpp closely — gets performance improvements and new model support quickly
- Explicit Vulkan, Metal, CUDA, and ROCm feature flags in build.rs
- Compiles llama.cpp from source via CMake — always fresh
- Proven adoption in the Rust ecosystem

**Version pinning:** Using `0.1.139` (latest as of March 2026). The crate follows llama.cpp upstream closely with frequent releases. Pin to avoid unexpected breakage; update deliberately.

**Rejected alternatives:**
- `llama_cpp` (edgenai): High-level API but last commit April 2024 — too stale
- `llama-cpp-v3`: Zero-config but very new (v0.1.2), low adoption, depends on GitHub API at build time

## Rollback Plan

This change replaces the entire inference backend. If llama-cpp-2 has unexpected issues in production:

- **Rollback strategy:** `git revert` the migration commit(s). The mistral.rs code is cleanly scoped to `local.rs` + `Cargo.toml`, so reverting is a single atomic operation.
- **Risk mitigation:** The migration will be done on a feature branch with thorough manual testing on multiple GPU vendors before merging to main.
