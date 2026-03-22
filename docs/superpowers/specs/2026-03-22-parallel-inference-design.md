# Parallel Inference & Async Model Loading

**Date:** 2026-03-22
**Status:** Draft

## Problem

Two performance issues in Jolly's local inference pipeline:

1. **Inference too slow:** Processing a README (~20 sentences) takes minutes because each sentence triggers a separate `run_inference()` call — full context creation, tokenization, prompt evaluation, and token generation — sequentially. GPU utilization peaks at ~60% then drops between calls.

2. **Startup blocks for 5-10 seconds:** `load_local_model()` runs synchronously in Tauri's `setup()` hook, blocking the window from appearing until a 4.7GB model (Mistral 7B) is fully loaded into VRAM.

## Solution

### 1. Continuous Batching (Parallel Sentence Processing)

Replace the sequential `for batch in parts.chunks(SENTENCES_PER_BATCH)` loop with a single `run_parallel_inference()` call that processes all sentence groups simultaneously within one `LlamaContext` using llama.cpp's sequence ID mechanism.

**Why continuous batching over multiple contexts:**
- Single KV cache (less VRAM) — important for Mistral 7B at 4.7GB
- GPU processes all sequences in one kernel launch — better utilization
- Shared system prompt via KV cache copy — avoids redundant prompt evaluation

### 2. Async Model Loading

Move model loading off the main thread so the app window appears immediately.

## Detailed Design

### `run_parallel_inference(texts: Vec<String>) -> Result<Vec<String>, String>`

New function in `local.rs`. Processes multiple text chunks simultaneously using a single `LlamaContext` with multiple sequence IDs.

**Parameters and limits:**

```
PER_SEQ_CTX  = 2048    // context window per sequence
MAX_TOKENS   = 1024    // max generation tokens per sequence
MAX_PARALLEL = 8       // cap parallel sequences to bound VRAM
```

**Pseudocode:**

```
fn run_parallel_inference(texts: Vec<String>) -> Result<Vec<String>, String> {
    // 1. Clamp parallelism
    let n_parallel = texts.len().min(MAX_PARALLEL)

    // 2. Acquire model read lock
    let model = MODEL.read() -> get ref or error

    // 3. Build the shared system prompt (same for all sequences)
    let system_msg = LlamaChatMessage::new("user", SYSTEM_PROMPT + "\n\n" + "placeholder")
    //    We'll use the chat template to get the prefix that wraps the system prompt.
    //    Actually, each sequence has different user text, so we format per-sequence.

    // 4. Create context sized for all sequences
    let ctx_params = LlamaContextParams::default()
        .with_n_ctx(PER_SEQ_CTX * n_parallel)
        .with_n_seq_max(n_parallel + 1)   // +1 for shared system prompt seq
        .with_n_batch(512)
        .with_n_threads(num_cpus)
        .with_n_threads_batch(num_cpus)
    let ctx = model.new_context(backend, ctx_params)

    // 5. Tokenize and evaluate shared system prompt as seq_id=0
    //    Format: chat template with system prompt only (no user text yet)
    let system_prompt_formatted = apply_chat_template(SYSTEM_PROMPT prefix)
    let system_tokens = model.str_to_token(system_prompt_formatted, AddBos::Always)
    let batch = LlamaBatch::new(...)
    for (pos, tok) in system_tokens:
        batch.add(tok, pos, &[0], pos == last)
    ctx.decode(&batch)

    // 6. Copy system prompt KV cache to all sequence IDs
    for seq_id in 1..=n_parallel:
        ctx.copy_kv_cache_seq(0, seq_id, 0, system_tokens.len())

    // 7. Tokenize each sequence's user-specific suffix and evaluate
    struct SeqState {
        seq_id: i32,
        n_past: i32,         // position cursor (starts after system prompt)
        output: String,
        done: bool,
        sampler: LlamaSampler,
        decoder: Decoder,
        i_batch: i32,        // index in batch for sampling
        original_text: String, // fallback on failure
    }

    let mut seqs: Vec<SeqState> = ...
    // For each sequence: tokenize user text suffix, add to batch with seq_id
    let batch = LlamaBatch::new(...)
    for seq in &seqs:
        let user_suffix_tokens = tokenize(seq.text)
        for (pos, tok) in user_suffix_tokens:
            batch.add(tok, seq.n_past + pos, &[seq.seq_id], pos == last)
        seq.n_past += user_suffix_tokens.len()

    ctx.decode(&batch)

    // 8. Generation loop
    for _ in 0..MAX_TOKENS:
        // Sample one token per active sequence
        for seq in seqs.filter(!done):
            let token = seq.sampler.sample(&ctx, seq.i_batch)
            seq.sampler.accept(token)
            if model.is_eog_token(token):
                seq.done = true
                ctx.clear_kv_cache_seq(seq.seq_id, ...)
                continue
            seq.output += model.token_to_piece(token, &mut seq.decoder)

        if all done: break

        // Feed sampled tokens back as next batch
        batch.clear()
        for seq in seqs.filter(!done):
            seq.i_batch = batch.n_tokens()
            batch.add(seq.last_token, seq.n_past, &[seq.seq_id], true)
            seq.n_past += 1
        ctx.decode(&batch)

    // 9. Collect results in order
    return seqs.map(|s| s.output.trim())
}
```

**Shared system prompt optimization:**
The system prompt is ~50 tokens. For 8 parallel sequences, this saves evaluating 350 tokens (7 × 50) — a minor but free optimization. We evaluate it once as sequence 0, then `copy_kv_cache_seq(0, N, 0, system_prompt_len)` clones the KV cache to each real sequence.

Note: The chat template wraps user text with special tokens. Since each sequence has different user text, we need to handle this carefully:
- Option A: Format each sequence's full prompt independently (system + user text), tokenize fully, no shared prefix optimization.
- Option B: Split the chat template into prefix (before user text) and suffix (after user text), share the prefix via KV copy.

**Recommendation: Option A** for simplicity. The system prompt is only ~50 tokens — the savings from sharing are small compared to the complexity of splitting chat templates. Each sequence gets its full formatted prompt tokenized independently. The main speedup comes from parallel generation, not shared prefixes.

**Revised approach (Option A):**

**API notes for implementer:**
- `with_n_ctx()` takes `Option<NonZeroU32>` — use `NonZeroU32::new(value)`
- `with_n_seq_max()` takes `u32` — cast `n_parallel as u32`
- `batch.add()` returns `Result<(), BatchAddError>` — must handle with `.map_err()?`
- `ctx.decode()` takes `&mut LlamaBatch` — both `ctx` and `batch` must be `mut`
- `LlamaSampler::chain_simple([...])` is the correct constructor, not `chain(...)`
- `top_p` second arg is `usize`, not `i32`
- The `RwLockReadGuard` holding the model must outlive the `LlamaContext` (which borrows `&'a self` from the model). Keep the guard alive for the entire function scope.

```
fn run_parallel_inference(texts: Vec<String>) -> Result<Vec<String>, String> {
    let n_parallel = texts.len().min(MAX_PARALLEL) as u32
    let model_guard = MODEL.read().map_err(|e| ...)?
    let model = model_guard.as_ref().ok_or("Model not loaded")?

    // Create context sized for all sequences
    let total_ctx = PER_SEQ_CTX * n_parallel
    let ctx_params = LlamaContextParams::default()
        .with_n_ctx(NonZeroU32::new(total_ctx))
        .with_n_seq_max(n_parallel)
        .with_n_batch(512)
        .with_n_threads(num_cpus as i32)
        .with_n_threads_batch(num_cpus as i32)
    let mut ctx = model.new_context(backend, ctx_params)
        .map_err(|e| ...)?

    // Tokenize all sequences independently
    let mut seqs: Vec<SeqState> = texts.iter().enumerate().map(|(i, text)| {
        let prompt = format!("{}\n\n{}", SYSTEM_PROMPT, text)
        let formatted = apply_chat_template_or_raw(model, prompt)
        let tokens = model.str_to_token(&formatted, AddBos::Always)?
        Ok(SeqState {
            seq_id: i as i32,
            tokens,
            last_token: LlamaToken(0),  // set during generation
            n_past: 0i32,
            output: String::new(),
            done: false,
            sampler: LlamaSampler::chain_simple([
                LlamaSampler::temp(0.1),
                LlamaSampler::top_p(0.9, 1usize),
                LlamaSampler::greedy(),
            ]),
            decoder: encoding_rs::UTF_8.new_decoder(),
            i_batch: 0i32,
            original_text: text.clone(),
        })
    }).collect::<Result<Vec<_>, String>>()?

    // Guard: skip sequences whose prompt exceeds PER_SEQ_CTX
    for seq in &mut seqs {
        if seq.tokens.len() as u32 >= PER_SEQ_CTX {
            seq.done = true  // will return original text
        }
    }

    // Feed all prompt tokens into one batch
    let max_prompt_tokens: usize = seqs.iter().map(|s| s.tokens.len()).sum()
    let mut batch = LlamaBatch::new(max_prompt_tokens.max(512), n_parallel as i32)
    for seq in &mut seqs {
        if seq.done { continue }
        for (pos, tok) in seq.tokens.iter().enumerate() {
            let is_last = pos == seq.tokens.len() - 1
            batch.add(*tok, pos as i32, &[seq.seq_id], is_last)
                .map_err(|e| format!("Failed to add token: {}", e))?
        }
        seq.n_past = seq.tokens.len() as i32
    }

    ctx.decode(&mut batch)
        .map_err(|e| format!("Failed to decode prompt: {}", e))?

    // Generation loop
    for _ in 0..MAX_TOKENS {
        // Sample one token per active sequence
        for seq in seqs.iter_mut().filter(|s| !s.done) {
            let token = seq.sampler.sample(&ctx, seq.i_batch)
            seq.sampler.accept(token)
            if model.is_eog_token(token) {
                seq.done = true
                // Free KV cache for this sequence
                ctx.clear_kv_cache_seq(Some(seq.seq_id as u32), None, None)
                    .map_err(|e| ...)?
                continue
            }
            let piece = model.token_to_piece(token, &mut seq.decoder, true, None)
                .map_err(|e| ...)?
            seq.output.push_str(&piece)
            seq.last_token = token
        }

        if seqs.iter().all(|s| s.done) { break }

        // Feed sampled tokens back as next batch
        batch.clear()
        for seq in seqs.iter_mut().filter(|s| !s.done) {
            seq.i_batch = batch.n_tokens() - 1  // will be 0-indexed after add
            batch.add(seq.last_token, seq.n_past, &[seq.seq_id], true)
                .map_err(|e| ...)?
            seq.n_past += 1
        }
        ctx.decode(&mut batch)
            .map_err(|e| format!("Failed to decode: {}", e))?
    }

    // Return results: use output if non-empty, else original text
    Ok(seqs.into_iter().map(|s| {
        let trimmed = s.output.trim().to_string()
        if trimmed.is_empty() { s.original_text } else { trimmed }
    }).collect())
}
```

### Modified `LocalProvider::correct_text()`

```rust
async fn correct_text(&self, text: &str) -> Result<String, String> {
    let text = text.to_string();
    tauri::async_runtime::spawn_blocking(move || {
        let parts = split_sentences(&text);

        // Fast path: 0 or 1 sentences
        if parts.len() <= 1 {
            return run_inference(&text);
        }

        // Group sentences into chunks
        let chunks: Vec<String> = parts
            .chunks(SENTENCES_PER_BATCH)
            .map(|batch| batch.iter().map(|(s, _)| *s).collect::<Vec<_>>().join(" "))
            .collect();

        eprintln!(
            "[jolly] Processing {} sentences in {} parallel sequences",
            parts.len(), chunks.len()
        );

        // Process chunks in rounds of MAX_PARALLEL via continuous batching
        let mut results: Vec<String> = Vec::new();
        for round in chunks.chunks(MAX_PARALLEL) {
            let round_results = run_parallel_inference(round.to_vec())?;
            results.extend(round_results);
        }

        // Reassemble with original separators
        let mut output = String::new();
        for (i, corrected) in results.iter().enumerate() {
            output.push_str(corrected);
            let batch_end_idx = ((i + 1) * SENTENCES_PER_BATCH).min(parts.len()) - 1;
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
```

**`SENTENCES_PER_BATCH`:** Bump from 1 to 4. With continuous batching, fewer chunks means fewer parallel sequences which means simpler batch management. 4 sentences per chunk balances quality (model sees enough context) with parallelism (a 20-sentence text = 5 parallel sequences).

### Async Model Loading

**Change in `lib.rs`:**

Read settings on the main thread (guaranteed safe), then spawn the blocking model load on a background thread:

```rust
.setup(|app| {
    // Read settings on main thread (store access requires setup to have run)
    let model_path = resolve_model_path_from_settings(app.handle());

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

- Settings read happens synchronously in `setup()` (thread-safe, fast)
- Only the slow `init_model()` call runs on a background thread
- Emits `model-loaded` event when done — frontend can listen and update UI
- `correct_text` already handles the "model not loaded" case (returns `model_not_loaded` error)
- No changes needed to `commands.rs`
- **Note:** During the async loading window, pressing Enter will show the existing "model not loaded" error. The first inference after startup may also briefly block if it races with the final moments of model loading (the read lock waits for the write lock in `swap_model` to release). This is benign.

**Frontend:** The frontend already shows an appropriate error when no model is loaded. Optionally, listen for `model-loaded` to show a subtle indicator, but not required for MVP.

### `run_inference()` — unchanged

The existing `run_inference()` function stays as-is. It's used by the benchmark binary and as the fast path for single-sentence inputs.

## Error Handling

| Scenario | Behavior |
|----------|----------|
| VRAM exhaustion (context creation fails) | Fall back to sequential `run_inference()` loop with warning log |
| Single sequence's prompt exceeds PER_SEQ_CTX | Mark done before generation, return original text for that chunk |
| Sequence produces empty output | Return original text for that chunk |
| Model not loaded | Error before any work starts (existing behavior) |
| More than MAX_PARALLEL chunks | Process in rounds of MAX_PARALLEL |
| Decode failure mid-generation | Return error for entire call (sequences share one context) |

## Testing Strategy

1. **Unit tests:** Existing `split_sentences` tests unchanged. No new unit tests needed (continuous batching requires a loaded model).

2. **Benchmark comparison:** Run existing benchmark with both old (sequential) and new (parallel) code paths. Compare:
   - Latency (should decrease significantly)
   - Accuracy (should be unchanged — same prompts, same sampling params)
   - GPU utilization (should increase from ~60% toward 80-90%)

3. **Manual smoke test:**
   - Process the README through the app
   - Verify correctness of output
   - Observe GPU utilization via `nvidia-smi` or equivalent
   - Verify startup is instant (model loads in background)

4. **Edge cases to test manually:**
   - Single sentence input (should use `run_inference` fast path)
   - Very long text (>8 chunks, verify round processing)
   - Empty input (should return empty string)

## Files Changed

| File | Change |
|------|--------|
| `src-tauri/src/inference/local.rs` | Add `run_parallel_inference()`, update `correct_text()`, bump `SENTENCES_PER_BATCH` to 4 |
| `src-tauri/src/lib.rs` | Move `load_local_model()` to background thread, emit `model-loaded` event |

## Expected Impact

- **Inference speed:** A 20-sentence README goes from ~20 sequential inference calls to ~5 parallel sequences in 1-2 rounds. Each generation step processes all sequences at once. Estimated 3-5x speedup.
- **GPU utilization:** Continuous batching keeps the GPU busy processing multiple sequences per decode call. Expected increase from ~60% to 80-90%.
- **Startup time:** From 5-10 seconds to near-instant. Model loads in background.
- **VRAM usage:** Slight increase due to larger context (2048 × N sequences), but well within 8GB GPU budget for up to 8 parallel sequences with Mistral 7B.
