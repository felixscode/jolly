# Parallel Inference & Async Model Loading Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Speed up multi-sentence inference by processing sentence batches in parallel via llama.cpp continuous batching, and eliminate startup blocking by loading the model in a background thread.

**Architecture:** A new `run_parallel_inference()` function uses a single `LlamaContext` with multiple sequence IDs to process N sentence groups simultaneously. Each sequence gets its own sampler and decoder. The generation loop samples all active sequences per step, feeding their tokens back in one batch. `correct_text()` is updated to call this instead of looping `run_inference()`. Model loading is moved to a background `std::thread::spawn` with a Tauri event emitted on completion.

**Tech Stack:** Rust, llama-cpp-2 v0.1.139 (continuous batching via `LlamaBatch` sequence IDs), Tauri 2, encoding_rs

**Spec:** `docs/superpowers/specs/2026-03-22-parallel-inference-design.md`

---

### Task 1: Add `run_parallel_inference()` function

**Files:**
- Modify: `src-tauri/src/inference/local.rs:1-16` (imports), then add new function after `run_inference()` (after line 290)

- [ ] **Step 1: Add new constants and imports**

At the top of `local.rs`, add the `LlamaToken` import (needed for `SeqState.last_token`). Update `SENTENCES_PER_BATCH` from 1 to 4. Add `MAX_PARALLEL` constant.

```rust
// Add to existing imports (line 12):
use llama_cpp_2::model::{AddBos, LlamaChatMessage, LlamaModel};
// becomes:
use llama_cpp_2::model::{AddBos, LlamaChatMessage, LlamaModel};
use llama_cpp_2::token::LlamaToken;

// Change line 29:
const SENTENCES_PER_BATCH: usize = 4;

// Add after SENTENCES_PER_BATCH:
/// Maximum number of parallel sequences in a single context to bound VRAM usage.
const MAX_PARALLEL: usize = 8;

/// Context window size per sequence.
const PER_SEQ_CTX: u32 = 2048;

/// Maximum generation tokens per sequence.
const MAX_GEN_TOKENS: i32 = 1024;
```

- [ ] **Step 2: Add the `run_parallel_inference` function**

Add after `run_inference()` (after line 290, before `#[cfg(test)]`):

```rust
/// Processes multiple text chunks simultaneously using continuous batching.
/// Each text gets its own sequence ID within a single LlamaContext.
/// Returns corrected text for each input, in the same order.
fn run_parallel_inference(texts: Vec<String>) -> Result<Vec<String>, String> {
    let n_parallel = texts.len().min(MAX_PARALLEL);
    eprintln!(
        "[jolly] run_parallel_inference: {} texts, {} parallel sequences",
        texts.len(),
        n_parallel
    );

    let backend = &*BACKEND;

    let model_guard = MODEL
        .read()
        .map_err(|e| format!("Model lock poisoned: {}", e))?;
    let model = model_guard
        .as_ref()
        .ok_or("Local model not loaded. Download a model in Settings.")?;

    // Get the chat template once (shared across all sequences)
    let chat_template = model.chat_template(None).ok();

    // Tokenize all sequences independently
    struct SeqState {
        seq_id: i32,
        tokens: Vec<LlamaToken>,
        last_token: LlamaToken,
        n_past: i32,
        output: String,
        done: bool,
        sampler: LlamaSampler,
        decoder: encoding_rs::Decoder,
        i_batch: i32,
        original_text: String,
    }

    let mut seqs: Vec<SeqState> = texts
        .iter()
        .enumerate()
        .map(|(i, text)| {
            let prompt = format!("{}\n\n{}", SYSTEM_PROMPT, text);
            let formatted = match &chat_template {
                Some(tmpl) => {
                    let msg = LlamaChatMessage::new("user".to_string(), prompt.clone())
                        .map_err(|e| format!("Failed to create chat message: {}", e))?;
                    model
                        .apply_chat_template(tmpl, &[msg], true)
                        .map_err(|e| format!("Failed to apply chat template: {}", e))?
                }
                None => prompt,
            };

            let tokens = model
                .str_to_token(&formatted, AddBos::Always)
                .map_err(|e| format!("Failed to tokenize seq {}: {}", i, e))?;

            Ok(SeqState {
                seq_id: i as i32,
                tokens,
                last_token: LlamaToken::new(0),
                n_past: 0,
                output: String::new(),
                done: false,
                sampler: LlamaSampler::chain_simple([
                    LlamaSampler::temp(0.1),
                    LlamaSampler::top_p(0.9, 1),
                    LlamaSampler::greedy(),
                ]),
                decoder: encoding_rs::UTF_8.new_decoder(),
                i_batch: 0,
                original_text: text.clone(),
            })
        })
        .collect::<Result<Vec<_>, String>>()?;

    // Guard: skip sequences whose prompt exceeds PER_SEQ_CTX
    for seq in &mut seqs {
        if seq.tokens.len() as u32 >= PER_SEQ_CTX {
            eprintln!(
                "[jolly] Seq {} prompt too long ({} tokens), returning original",
                seq.seq_id,
                seq.tokens.len()
            );
            seq.done = true;
        }
    }

    // If all sequences are done (all too long), return originals
    if seqs.iter().all(|s| s.done) {
        return Ok(seqs.into_iter().map(|s| s.original_text).collect());
    }

    // Create context sized for all active sequences
    let n_active = seqs.iter().filter(|s| !s.done).count() as u32;
    let total_ctx = PER_SEQ_CTX.saturating_mul(n_active);
    let n_threads = std::thread::available_parallelism()
        .map(|n| n.get() as i32)
        .unwrap_or(4);

    // n_batch must be >= total prompt tokens so we can decode all prompts at once.
    // For the generation phase, each decode only has n_active tokens (one per seq).
    let total_prompt_tokens: usize = seqs.iter().filter(|s| !s.done).map(|s| s.tokens.len()).sum();
    let n_batch = total_prompt_tokens.max(512) as u32;

    let ctx_params = LlamaContextParams::default()
        .with_n_ctx(NonZeroU32::new(total_ctx))
        .with_n_seq_max(n_active)
        .with_n_batch(n_batch)
        .with_n_threads(n_threads)
        .with_n_threads_batch(n_threads);
    let mut ctx = model
        .new_context(backend, ctx_params)
        .map_err(|e| format!("Failed to create parallel context: {}", e))?;

    // Feed all prompt tokens into one batch
    let mut batch = LlamaBatch::new(total_prompt_tokens.max(512), 1);

    for seq in seqs.iter_mut().filter(|s| !s.done) {
        let last_pos = (seq.tokens.len() - 1) as i32;
        for (pos, tok) in seq.tokens.iter().enumerate() {
            let is_last = pos as i32 == last_pos;
            batch
                .add(*tok, pos as i32, &[seq.seq_id], is_last)
                .map_err(|e| format!("Failed to add prompt token: {}", e))?;
        }
        seq.n_past = seq.tokens.len() as i32;
    }

    ctx.decode(&mut batch)
        .map_err(|e| format!("Failed to decode prompts: {}", e))?;

    // Set initial i_batch for sampling (each seq samples from its last prompt token)
    // The batch was filled sequentially: seq0 tokens then seq1 tokens etc.
    // Each seq's last token is at the cumulative offset.
    let mut offset = 0i32;
    for seq in seqs.iter_mut().filter(|s| !s.done) {
        seq.i_batch = offset + (seq.tokens.len() as i32 - 1);
        offset += seq.tokens.len() as i32;
    }

    // Generation loop
    for _ in 0..MAX_GEN_TOKENS {
        // Sample one token per active sequence
        for seq in seqs.iter_mut().filter(|s| !s.done) {
            let token = seq.sampler.sample(&ctx, seq.i_batch);
            seq.sampler.accept(token);

            if model.is_eog_token(token) {
                seq.done = true;
                let _ = ctx.clear_kv_cache_seq(Some(seq.seq_id as u32), None, None);
                continue;
            }

            let piece = model
                .token_to_piece(token, &mut seq.decoder, true, None)
                .map_err(|e| format!("Failed to detokenize seq {}: {}", seq.seq_id, e))?;
            seq.output.push_str(&piece);
            seq.last_token = token;
        }

        if seqs.iter().all(|s| s.done) {
            break;
        }

        // Feed sampled tokens back as next batch
        batch.clear();
        for seq in seqs.iter_mut().filter(|s| !s.done) {
            seq.i_batch = batch.n_tokens();
            batch
                .add(seq.last_token, seq.n_past, &[seq.seq_id], true)
                .map_err(|e| format!("Failed to add gen token: {}", e))?;
            seq.n_past += 1;
        }
        ctx.decode(&mut batch)
            .map_err(|e| format!("Failed to decode generation step: {}", e))?;
    }

    // Return results: use output if non-empty, else original text
    Ok(seqs
        .into_iter()
        .map(|s| {
            let trimmed = s.output.trim().to_string();
            if trimmed.is_empty() {
                s.original_text
            } else {
                trimmed
            }
        })
        .collect())
}
```

- [ ] **Step 3: Verify it compiles**

Run: `cd /home/dev/jolly/src-tauri && cargo check 2>&1 | head -30`

Expected: Compiles with no errors. Warnings about unused `run_parallel_inference` are fine (we'll wire it up in Task 2).

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/inference/local.rs
git commit -m "feat: add run_parallel_inference with continuous batching"
```

---

### Task 2: Wire `correct_text()` to use parallel inference

**Files:**
- Modify: `src-tauri/src/inference/local.rs:121-176` (the `LLMProvider` impl block)

- [ ] **Step 1: Replace the sequential loop in `correct_text()`**

Replace the entire `LLMProvider` impl (lines 121-176) with:

```rust
#[async_trait]
impl LLMProvider for LocalProvider {
    async fn correct_text(&self, text: &str) -> Result<String, String> {
        let text = text.to_string();
        tauri::async_runtime::spawn_blocking(move || {
            let parts = split_sentences(&text);

            // Fast path: 0 or 1 sentences — no splitting needed
            if parts.len() <= 1 {
                return run_inference(&text);
            }

            // Group sentences into chunks of SENTENCES_PER_BATCH
            let chunks: Vec<String> = parts
                .chunks(SENTENCES_PER_BATCH)
                .map(|batch| {
                    batch
                        .iter()
                        .map(|(sentence, _)| *sentence)
                        .collect::<Vec<_>>()
                        .join(" ")
                })
                .collect();

            eprintln!(
                "[jolly] Processing {} sentences in {} chunks (batch size {})",
                parts.len(),
                chunks.len(),
                SENTENCES_PER_BATCH,
            );

            // Process chunks in rounds of MAX_PARALLEL via continuous batching
            // Falls back to sequential run_inference if parallel context creation fails
            let mut results: Vec<String> = Vec::new();
            for round in chunks.chunks(MAX_PARALLEL) {
                match run_parallel_inference(round.to_vec()) {
                    Ok(round_results) => results.extend(round_results),
                    Err(e) => {
                        eprintln!("[jolly] Parallel inference failed: {}, falling back to sequential", e);
                        for chunk in round {
                            results.push(run_inference(chunk)?);
                        }
                    }
                }
            }

            // Reassemble using original separators between chunks
            let mut output = String::new();
            for (i, corrected) in results.iter().enumerate() {
                output.push_str(corrected);
                let batch_end_idx =
                    ((i + 1) * SENTENCES_PER_BATCH).min(parts.len()) - 1;
                let (_, sep) = parts[batch_end_idx];
                if !sep.is_empty() {
                    output.push_str(sep);
                }
            }

            Ok(output)
        })
        .await
        .map_err(|e| format!("Inference task failed: {}", e))?
    }
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cd /home/dev/jolly/src-tauri && cargo check 2>&1 | head -30`

Expected: Compiles with no errors or new warnings.

- [ ] **Step 3: Run existing tests**

Run: `cd /home/dev/jolly/src-tauri && cargo test 2>&1 | tail -20`

Expected: All 21 tests pass (9 split_sentences + others). The split_sentences tests don't depend on model loading so they work without changes.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/inference/local.rs
git commit -m "feat: wire correct_text to use parallel inference with fallback"
```

---

### Task 3: Make model loading async at startup

**Files:**
- Modify: `src-tauri/src/lib.rs:13-80` (refactor `load_local_model`), `src-tauri/src/lib.rs:91-94` (the `.setup()` call)

- [ ] **Step 1: Split `load_local_model` into path resolution + loading**

Replace `load_local_model` (lines 13-80) and update `setup` (lines 91-94):

```rust
use tauri::Emitter;

/// Resolve the model path from settings. Runs on main thread (store access).
/// Returns None if no model is configured or file doesn't exist.
fn resolve_startup_model_path(app: &tauri::AppHandle) -> Option<std::path::PathBuf> {
    let app_data = match app.path().app_data_dir() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("[jolly] Failed to get app data directory: {}", e);
            return None;
        }
    };

    let models_path = match model_manager::models_dir(&app_data) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("[jolly] Failed to get models directory: {}", e);
            return None;
        }
    };

    let active_model_id: Option<String> = app
        .store("settings.json")
        .ok()
        .and_then(|store| store.get("activeModelId"))
        .and_then(|v| v.as_str().map(|s| s.to_string()));

    if let Some(ref id) = active_model_id {
        if id.starts_with("custom-") {
            let path_str = commands::get_custom_model_path(app, id);
            match path_str {
                Some(p) => {
                    let path = std::path::PathBuf::from(&p);
                    if path.exists() {
                        Some(path)
                    } else {
                        eprintln!("[jolly] Custom model file not found: {}", p);
                        None
                    }
                }
                None => {
                    eprintln!("[jolly] Custom model ID not found in settings: {}", id);
                    None
                }
            }
        } else {
            match model_manager::resolve_model_path(&models_path, active_model_id.as_deref()) {
                Ok(p) => Some(p),
                Err(e) => {
                    eprintln!("[jolly] No local model available: {}", e);
                    None
                }
            }
        }
    } else {
        match model_manager::resolve_model_path(&models_path, None) {
            Ok(p) => Some(p),
            Err(e) => {
                eprintln!("[jolly] No local model available: {}", e);
                None
            }
        }
    }
}
```

Then update the `.setup()` call (lines 91-94):

```rust
        .setup(|app| {
            let model_path = resolve_startup_model_path(app.handle());
            if let Some(path) = model_path {
                let handle = app.handle().clone();
                std::thread::spawn(move || {
                    match inference::local::init_model(&path) {
                        Ok(()) => {
                            eprintln!("[jolly] Local model loaded: {:?}", path);
                            let _ = handle.emit("model-loaded", ());
                        }
                        Err(e) => eprintln!("[jolly] Failed to load local model: {}", e),
                    }
                });
            }
            Ok(())
        })
```

- [ ] **Step 2: Add `use tauri::Emitter;` to imports**

At the top of `lib.rs`, add after `use tauri::Manager;`:

```rust
use tauri::Emitter;
```

Check if `Emitter` is the correct trait for Tauri 2's `emit()` method. If it doesn't exist as a separate trait (it may be part of `Manager`), remove the import. The compiler will tell us.

- [ ] **Step 3: Verify it compiles**

Run: `cd /home/dev/jolly/src-tauri && cargo check 2>&1 | head -30`

Expected: Compiles. If `Emitter` trait import fails, the `emit` method is likely on `Manager` — adjust import accordingly.

- [ ] **Step 4: Run tests**

Run: `cd /home/dev/jolly/src-tauri && cargo test 2>&1 | tail -20`

Expected: All 21 tests still pass.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "perf: load model in background thread for instant startup"
```

---

### Task 4: Verify build and run all tests

**Files:**
- No files modified — verification only

- [ ] **Step 1: Run full cargo check**

Run: `cd /home/dev/jolly/src-tauri && cargo check 2>&1`

Expected: No errors.

- [ ] **Step 2: Run clippy**

Run: `cd /home/dev/jolly/src-tauri && cargo clippy -- -W clippy::all 2>&1 | tail -30`

Expected: No new warnings from our changes.

- [ ] **Step 3: Run all tests**

Run: `cd /home/dev/jolly/src-tauri && cargo test 2>&1`

Expected: All 21 tests pass.

- [ ] **Step 4: Verify benchmark binary compiles**

Run: `cd /home/dev/jolly/src-tauri && cargo check --bin benchmark 2>&1 | head -10`

Expected: Compiles (benchmark uses `run_inference` directly, which is unchanged).

- [ ] **Step 5: Fix any issues found, then commit fixes if needed**

---

### Task 5: Manual smoke test

**Files:**
- No files modified — manual testing

- [ ] **Step 1: Start the app in dev mode**

Run: `cd /home/dev/jolly && npx tauri dev --features vulkan` (Linux/Windows) or `npx tauri dev --features metal` (macOS)

- [ ] **Step 2: Verify instant startup**

Expected: App window appears immediately. Terminal shows model loading in background. After a few seconds, `[jolly] Local model loaded` appears.

- [ ] **Step 3: Test single sentence**

Copy a single sentence with a typo to clipboard (e.g., "I recieved the packge yesterday."), press Enter. Should correct quickly via `run_inference` fast path.

- [ ] **Step 4: Test multi-sentence text**

Copy the README or a multi-paragraph text to clipboard, press Enter. Watch terminal for:
- `[jolly] Processing N sentences in M chunks (batch size 4)`
- `[jolly] run_parallel_inference: M texts, M parallel sequences`
- Should complete significantly faster than before.

- [ ] **Step 5: Monitor GPU utilization**

Run `nvidia-smi -l 1` or `watch -n 1 nvidia-smi` in a separate terminal during inference. GPU utilization should be higher than before (~80-90% vs previous ~60%).
