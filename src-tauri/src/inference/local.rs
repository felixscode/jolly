use std::num::NonZeroU32;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{LazyLock, Mutex, RwLock};

use regex::Regex;

use async_trait::async_trait;
use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::model::{AddBos, LlamaChatMessage, LlamaChatTemplate, LlamaModel};
use llama_cpp_2::sampling::LlamaSampler;
use llama_cpp_2::token::LlamaToken;

use super::LLMProvider;
use super::SYSTEM_PROMPT;

/// Llama.cpp backend handle. Initialized once on first use.
static BACKEND: LazyLock<LlamaBackend> = LazyLock::new(|| {
    LlamaBackend::init().expect("Failed to initialize llama backend")
});

/// Global model handle. RwLock so models can be swapped at runtime.
/// Stores (model, model_id, optional_prompt_template).
/// Note: swap_model takes a write lock; run_inference takes a read lock.
/// Model swaps block until in-flight inferences complete.
static MODEL: RwLock<Option<(LlamaModel, String, Option<String>)>> = RwLock::new(None);

/// Serialises model loads so a startup load and a user switch can't race
/// (both loading into VRAM, last one silently winning).
static LOAD_LOCK: Mutex<()> = Mutex::new(());

/// Lock-free readiness flag for async callers (avoids blocking a runtime
/// thread on `MODEL.read()` while a swap waits behind a long inference).
static LOADED: AtomicBool = AtomicBool::new(false);

/// Number of sentences per LLM call. Increase for speed, decrease for quality.
const SENTENCES_PER_BATCH: usize = 4;
const MAX_PARALLEL: usize = 8;
const PER_SEQ_CTX: u32 = 2048;
const MAX_GEN_TOKENS: i32 = 1024;
/// Upper bound for the single-sequence context (prompt + generation).
const MAX_CTX: u32 = 16384;

/// Regex matching sentence-ending punctuation followed by whitespace.
static SENTENCE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[.!?]\s+").unwrap());

/// Load a model at startup unless one is already loaded (e.g. the user
/// switched models while the startup load was queued).
pub fn init_model(model_path: &Path, model_id: &str, prompt_template: Option<&str>) -> Result<(), String> {
    let _load = LOAD_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    if LOADED.load(Ordering::SeqCst) {
        eprintln!("[jolly] A model is already loaded; skipping startup load of {}", model_id);
        return Ok(());
    }
    load_into_slot(model_path, model_id, prompt_template)
}

/// Load a new model, replacing any currently loaded model.
pub fn swap_model(model_path: &Path, model_id: &str, prompt_template: Option<&str>) -> Result<(), String> {
    let _load = LOAD_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    load_into_slot(model_path, model_id, prompt_template)
}

/// Id of the currently loaded model, if any.
pub fn loaded_model_id() -> Option<String> {
    MODEL.read().ok()?.as_ref().map(|(_, id, _)| id.clone())
}

fn load_into_slot(model_path: &Path, model_id: &str, prompt_template: Option<&str>) -> Result<(), String> {
    let backend = &*BACKEND;

    // Drop the old model first to free VRAM
    {
        let mut slot = MODEL.write().map_err(|e| format!("Model lock poisoned: {}", e))?;
        *slot = None;
        LOADED.store(false, Ordering::SeqCst);
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
    *slot = Some((model, model_id.to_string(), prompt_template.map(|s| s.to_string())));
    LOADED.store(true, Ordering::SeqCst);
    Ok(())
}

/// Unload the current model, freeing its memory.
pub fn unload_model() {
    let _load = LOAD_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    if let Ok(mut slot) = MODEL.write() {
        *slot = None;
        LOADED.store(false, Ordering::SeqCst);
    }
}

/// Check if a model is loaded and ready for inference.
pub fn is_model_loaded() -> bool {
    LOADED.load(Ordering::SeqCst)
}

/// Split text into (sentence, trailing_separator) pairs.
/// Punctuation stays with the sentence; whitespace between sentences is captured
/// so the original spacing can be restored during reassembly.
fn split_sentences(text: &str) -> Vec<(&str, &str)> {
    if text.is_empty() {
        return Vec::new();
    }

    let mut result = Vec::new();
    let mut last_end = 0;

    for m in SENTENCE_RE.find_iter(text) {
        // The match covers e.g. ". " — punctuation char is at m.start(),
        // whitespace is m.start()+1 .. m.end()
        let sentence = &text[last_end..m.start() + 1]; // includes the punctuation
        let separator = &text[m.start() + 1..m.end()]; // the whitespace
        result.push((sentence, separator));
        last_end = m.end();
    }

    // Remainder after last match (or entire text if no matches)
    if last_end < text.len() {
        result.push((&text[last_end..], ""));
    }

    result
}

pub struct LocalProvider;

impl LocalProvider {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl LLMProvider for LocalProvider {
    async fn correct_text(&self, text: &str) -> Result<String, String> {
        let text = text.to_string();
        tokio::task::spawn_blocking(move || {
            let parts = split_sentences(&text);

            // Fast path: 0 or 1 sentences — no splitting needed
            if parts.len() <= 1 {
                return run_inference(&text).map(|out| or_original(out, &text));
            }

            // Group sentences into chunks of SENTENCES_PER_BATCH, keeping the
            // original separators (incl. newlines) between sentences of a chunk.
            let chunks: Vec<String> = parts
                .chunks(SENTENCES_PER_BATCH)
                .map(|batch| {
                    let mut chunk = String::new();
                    for (i, (sentence, sep)) in batch.iter().enumerate() {
                        chunk.push_str(sentence);
                        if i + 1 < batch.len() {
                            chunk.push_str(sep);
                        }
                    }
                    chunk
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
                            results.push(or_original(run_inference(chunk)?, chunk));
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

/// Never hand back an empty correction: an empty model answer means
/// "no usable output", so keep the user's original text instead.
fn or_original(output: String, original: &str) -> String {
    if output.trim().is_empty() {
        original.to_string()
    } else {
        output
    }
}

/// Strip `<think>...</think>` blocks from model output.
/// Some models (Qwen3, Qwen3.5) emit reasoning blocks before the answer.
fn strip_think_tags(output: &str) -> String {
    // Templates that open `<think>` in the prompt produce output that only
    // contains the closing tag: keep what follows the last `</think>`.
    if !output.contains("<think>") {
        return match output.rfind("</think>") {
            Some(end) => output[end + "</think>".len()..].trim().to_string(),
            None => output.to_string(),
        };
    }
    // Remove everything between <think> and </think> (inclusive)
    let mut result = output.to_string();
    while let Some(start) = result.find("<think>") {
        if let Some(rel_end) = result[start..].find("</think>") {
            let end = start + rel_end + "</think>".len();
            result = format!("{}{}", &result[..start], &result[end..]);
        } else {
            // Unclosed <think> — remove everything from <think> onward
            result.truncate(start);
            break;
        }
    }
    result.trim().to_string()
}

/// Trim runaway generation: if the output is much longer than the input,
/// the model is looping. Return only the first meaningful block.
fn trim_runaway(output: &str, input: &str) -> String {
    let max_len = input.len() + 200;
    if output.len() <= max_len {
        return output.to_string();
    }
    // Take the first paragraph (up to double newline) or first line
    if let Some(pos) = output.find("\n\n") {
        if pos > 0 {
            return output[..pos].to_string();
        }
    }
    if let Some(pos) = output.find('\n') {
        if pos > 0 {
            return output[..pos].to_string();
        }
    }
    let mut cut = max_len;
    while !output.is_char_boundary(cut) {
        cut -= 1;
    }
    output[..cut].to_string()
}

/// Detect whether a chat template is GRMR-style (uses "corrected" turn tag).
fn is_grmr_template(template: &LlamaChatTemplate) -> bool {
    template.to_str().map_or(false, |s| s.contains("corrected"))
}

/// Format the prompt for a GRMR V3 model using the manual text/corrected format.
/// llama.cpp's apply_chat_template outputs turn markers as literal text rather
/// than special token IDs, so we format manually for correct tokenization.
fn format_grmr_prompt(text: &str) -> String {
    format!("text\n{}\ncorrected\n", text)
}

/// Format the prompt for a standard instruct model using its chat template.
fn format_instruct_prompt(model: &LlamaModel, template: &LlamaChatTemplate, text: &str) -> Result<String, String> {
    let prompt = format!("{}\n\n{}", SYSTEM_PROMPT, text);
    let msg = LlamaChatMessage::new("user".to_string(), prompt)
        .map_err(|e| format!("Failed to create chat message: {}", e))?;
    let mut formatted = model
        .apply_chat_template(template, &[msg], true)
        .map_err(|e| format!("Failed to apply chat template: {}", e))?;
    // Hybrid reasoning models (Qwen3/3.5): pre-fill an empty think block, which is
    // what their template emits for enable_thinking=false. Saves the token budget.
    if template.to_str().is_ok_and(|t| t.contains("<think>")) && !formatted.ends_with("</think>\n\n") {
        formatted.push_str("<think>\n\n</think>\n\n");
    }
    Ok(formatted)
}

/// Prompt strategy detected for a loaded model.
#[derive(Debug, Clone, Copy, PartialEq)]
enum PromptKind {
    /// Model has a registry prompt_template (e.g. GRMR 2B "### Original Text:" format)
    Registry,
    /// Model has a GRMR-style GGUF chat template (uses "corrected" turn tag)
    Grmr,
    /// Model has a standard GGUF chat template (uses system prompt)
    Standard,
    /// No template at all — raw system prompt fallback
    Fallback,
}

/// Build the tokenized prompt for a model.
/// Priority:
///   1. Registry prompt_template (for models without GGUF template)
///   2. GRMR GGUF template (has "corrected" tag): just the text, no system prompt
///   3. Standard GGUF chat template: system prompt + text
///   4. Raw fallback: system prompt + text
/// Returns (tokens, prompt_kind).
fn build_prompt(model: &LlamaModel, text: &str, registry_template: Option<&str>) -> Result<(Vec<LlamaToken>, PromptKind), String> {
    // Priority 1: Registry template
    if let Some(tmpl) = registry_template {
        eprintln!("[jolly] Using registry prompt template");
        let formatted = tmpl.replace("{text}", text);
        let tokens = model
            .str_to_token(&formatted, AddBos::Always)
            .map_err(|e| format!("Failed to tokenize: {}", e))?;
        return Ok((tokens, PromptKind::Registry));
    }

    // Priority 2-3: GGUF chat template
    let chat_template = model.chat_template(None).ok();
    let is_grmr = chat_template.as_ref().is_some_and(|t| is_grmr_template(t));

    let formatted = if let Some(tmpl) = &chat_template {
        if is_grmr {
            eprintln!("[jolly] Using GRMR text/corrected format");
            format_grmr_prompt(text)
        } else {
            eprintln!("[jolly] Using standard chat template");
            format_instruct_prompt(model, tmpl, text)?
        }
    } else {
        eprintln!("[jolly] No chat template found, using raw prompt");
        format!("{}\n\n{}", SYSTEM_PROMPT, text)
    };

    let kind = if is_grmr {
        PromptKind::Grmr
    } else if chat_template.is_some() {
        PromptKind::Standard
    } else {
        PromptKind::Fallback
    };

    // BOS is handled by apply_chat_template for standard models.
    // GRMR and fallback use manual formatting and need explicit BOS.
    let add_bos = if kind == PromptKind::Standard { AddBos::Never } else { AddBos::Always };
    let tokens = model
        .str_to_token(&formatted, add_bos)
        .map_err(|e| format!("Failed to tokenize: {}", e))?;

    Ok((tokens, kind))
}

/// Runs inference synchronously. Called inside spawn_blocking.
pub fn run_inference(text: &str) -> Result<String, String> {
    eprintln!("[jolly] run_inference called ({} chars)", text.chars().count());

    let backend = &*BACKEND;

    let model_guard = MODEL
        .read()
        .map_err(|e| format!("Model lock poisoned: {}", e))?;
    let (model, _model_id, registry_template) = model_guard
        .as_ref()
        .ok_or("Local model not loaded. Download a model in Settings.")?;

    let input_len = text.len();
    let (tokens, prompt_kind) = build_prompt(model, text, registry_template.as_deref())?;

    eprintln!("[jolly] Input tokens: {}", tokens.len());

    if tokens.is_empty() {
        return Ok(String::new());
    }

    // Generation budget scales with the input (corrections are about as long
    // as the input); the context must hold prompt + generated tokens.
    let max_tokens: i32 = (tokens.len() as i32 + 256).clamp(256, 4096);
    let n_ctx: u32 = tokens.len() as u32 + max_tokens as u32;
    if n_ctx > MAX_CTX {
        return Err(format!(
            "Input too long: {} tokens exceeds the supported context of {}",
            tokens.len(),
            MAX_CTX
        ));
    }

    // Create context for this inference call
    let ctx_params = LlamaContextParams::default()
        .with_n_ctx(NonZeroU32::new(n_ctx));
    let mut ctx = model
        .new_context(backend, ctx_params)
        .map_err(|e| format!("Failed to create context: {}", e))?;

    // Feed tokens into context
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

    let mut n_cur = tokens.len() as i32;
    let mut output = String::new();
    let mut decoder = encoding_rs::UTF_8.new_decoder();
    let mut finished = false;

    for _ in 0..max_tokens {
        let token = sampler.sample(&ctx, batch.n_tokens() - 1);
        sampler.accept(token);

        if model.is_eog_token(token) {
            finished = true;
            break;
        }

        // Detokenize this token
        let piece = model
            .token_to_piece(token, &mut decoder, true, None)
            .map_err(|e| format!("Failed to detokenize: {}", e))?;
        output.push_str(&piece);

        // Model-specific stop conditions for looping
        let stop_pos = match prompt_kind {
            PromptKind::Grmr => output.find("\ntext\n").or_else(|| output.find("\ncorrected\n")),
            PromptKind::Registry => output.find("### Original Text:"),
            _ => None,
        };
        if let Some(pos) = stop_pos {
            output.truncate(pos);
            finished = true;
            break;
        }

        // Stop runaway generation: output should never be much longer than input
        if output.len() > input_len + 200 {
            output = trim_runaway(&output, text);
            finished = true;
            break;
        }

        // Feed token back for next iteration
        batch.clear();
        batch
            .add(token, n_cur, &[0], true)
            .map_err(|e| format!("Failed to add token to batch: {}", e))?;
        ctx.decode(&mut batch)
            .map_err(|e| format!("Failed to decode: {}", e))?;
        n_cur += 1;
    }

    if !finished {
        // Hit the token cap without EOS: the answer is incomplete, so don't
        // return a truncated correction.
        eprintln!("[jolly] Generation hit max_tokens; returning original text");
        return Ok(text.to_string());
    }
    Ok(strip_think_tags(output.trim()))
}

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
    let (model, _model_id, registry_template) = model_guard
        .as_ref()
        .ok_or("Local model not loaded. Download a model in Settings.")?;

    // Tokenize all sequences independently using build_prompt
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
        /// True once the sequence ended on its own (EOS / stop rule).
        finished: bool,
        original_text: String,
        input_len: usize,
        prompt_kind: PromptKind,
    }

    let mut seqs: Vec<SeqState> = texts
        .iter()
        .enumerate()
        .map(|(i, text)| {
            let (tokens, prompt_kind) = build_prompt(model, text, registry_template.as_deref())
                .map_err(|e| format!("Failed to build prompt for seq {}: {}", i, e))?;

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
                finished: false,
                original_text: text.clone(),
                input_len: text.len(),
                prompt_kind,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;

    // Guard: skip sequences whose prompt exceeds PER_SEQ_CTX
    for seq in &mut seqs {
        if seq.tokens.len() as u32 + MAX_GEN_TOKENS as u32 > PER_SEQ_CTX {
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

    // llama.cpp requires seq_id < n_seq_max, so number only the active ones.
    for (active_idx, seq) in seqs.iter_mut().filter(|s| !s.done).enumerate() {
        seq.seq_id = active_idx as i32;
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

    // Feed all prompt tokens into one batch.
    // n_seq_max=1 because each token belongs to exactly one sequence.
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
                seq.finished = true;
                if let Err(e) = ctx.clear_kv_cache_seq(Some(seq.seq_id as u32), None, None) {
                    eprintln!("[jolly] Failed to clear KV cache for seq {}: {}", seq.seq_id, e);
                }
                continue;
            }

            let piece = model
                .token_to_piece(token, &mut seq.decoder, true, None)
                .map_err(|e| format!("Failed to detokenize seq {}: {}", seq.seq_id, e))?;
            seq.output.push_str(&piece);
            seq.last_token = token;

            // Model-specific stop conditions for looping
            let stop_pos = match seq.prompt_kind {
                PromptKind::Grmr => seq.output.find("\ntext\n").or_else(|| seq.output.find("\ncorrected\n")),
                PromptKind::Registry => seq.output.find("### Original Text:"),
                _ => None,
            };
            if let Some(pos) = stop_pos {
                seq.output.truncate(pos);
                seq.done = true;
                seq.finished = true;
                if let Err(e) = ctx.clear_kv_cache_seq(Some(seq.seq_id as u32), None, None) {
                    eprintln!("[jolly] Failed to clear KV cache for seq {}: {}", seq.seq_id, e);
                }
                continue;
            }

            // Stop runaway generation
            if seq.output.len() > seq.input_len + 200 {
                seq.output = trim_runaway(&seq.output, &seq.original_text);
                seq.done = true;
                seq.finished = true;
                if let Err(e) = ctx.clear_kv_cache_seq(Some(seq.seq_id as u32), None, None) {
                    eprintln!("[jolly] Failed to clear KV cache for seq {}: {}", seq.seq_id, e);
                }
                continue;
            }
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

    // Return results: use output if complete and non-empty, else original text
    Ok(seqs
        .into_iter()
        .map(|s| {
            let trimmed = strip_think_tags(s.output.trim());
            if !s.finished || trimmed.is_empty() {
                s.original_text
            } else {
                trimmed
            }
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_empty() {
        assert!(split_sentences("").is_empty());
    }

    #[test]
    fn split_no_punctuation() {
        let result = split_sentences("hello world");
        assert_eq!(result, vec![("hello world", "")]);
    }

    #[test]
    fn split_single_sentence() {
        let result = split_sentences("Hello world.");
        assert_eq!(result, vec![("Hello world.", "")]);
    }

    #[test]
    fn split_two_sentences() {
        let result = split_sentences("First. Second.");
        assert_eq!(result, vec![("First.", " "), ("Second.", "")]);
    }

    #[test]
    fn split_preserves_newlines() {
        let result = split_sentences("First.\n\nSecond.");
        assert_eq!(result, vec![("First.", "\n\n"), ("Second.", "")]);
    }

    #[test]
    fn split_mixed_punctuation() {
        let result = split_sentences("Really? Yes! Done.");
        assert_eq!(
            result,
            vec![("Really?", " "), ("Yes!", " "), ("Done.", "")]
        );
    }

    #[test]
    fn split_no_trailing_whitespace() {
        // No whitespace after last punctuation — no split
        let result = split_sentences("One.Two");
        assert_eq!(result, vec![("One.Two", "")]);
    }

    #[test]
    fn split_multiple_spaces() {
        let result = split_sentences("First.  Second.");
        assert_eq!(result, vec![("First.", "  "), ("Second.", "")]);
    }

    #[test]
    fn split_trailing_whitespace_multi_sentence() {
        // Trailing whitespace after last punctuation in multi-sentence input
        let result = split_sentences("First. Second. ");
        assert_eq!(result, vec![("First.", " "), ("Second.", " ")]);
    }

    // ── trim_runaway tests ──────────────────────────────────────

    #[test]
    fn trim_runaway_short_output_unchanged() {
        let result = trim_runaway("Hello world.", "Hello wrold.");
        assert_eq!(result, "Hello world.");
    }

    #[test]
    fn trim_runaway_cuts_at_double_newline() {
        let input = "Short.";
        let output = format!("Short.\n\n{}", "garbage ".repeat(100));
        let result = trim_runaway(&output, input);
        assert_eq!(result, "Short.");
    }

    #[test]
    fn trim_runaway_cuts_at_single_newline() {
        let input = "Short.";
        let output = format!("Short.\n{}", "garbage ".repeat(100));
        let result = trim_runaway(&output, input);
        assert_eq!(result, "Short.");
    }

    // ── Registry template format tests ──────────────────────────

    #[test]
    fn registry_template_formats_correctly() {
        let template = "Below is the original text.\n\n### Original Text:\n{text}\n\n### Corrected Text:\n";
        let text = "I recieved your messege.";
        let formatted = template.replace("{text}", text);
        assert!(formatted.contains("### Original Text:\nI recieved your messege."));
        assert!(formatted.contains("### Corrected Text:\n"));
        assert!(!formatted.contains("{text}"));
    }

    #[test]
    fn registry_template_stop_condition() {
        // Simulate model looping: output contains "### Original Text:" again
        let output = "I received your message.\n\n### Original Text:\nsome other text";
        let stop_pos = output.find("### Original Text:");
        assert!(stop_pos.is_some());
        let truncated = &output[..stop_pos.unwrap()];
        assert_eq!(truncated.trim(), "I received your message.");
    }

    // ── PromptKind tests ────────────────────────────────────────

    #[test]
    fn prompt_kind_equality() {
        assert_eq!(PromptKind::Registry, PromptKind::Registry);
        assert_eq!(PromptKind::Grmr, PromptKind::Grmr);
        assert_ne!(PromptKind::Registry, PromptKind::Grmr);
        assert_ne!(PromptKind::Standard, PromptKind::Fallback);
    }

    #[test]
    fn strip_think_only_closing_tag() {
        // Template opened <think> in the prompt; output only has the close tag.
        assert_eq!(strip_think_tags("reasoning…\n</think>\n\nFixed text."), "Fixed text.");
    }

    #[test]
    fn strip_think_stray_close_before_open() {
        assert_eq!(strip_think_tags("A </think> B <think>x</think> C"), "A </think> B  C");
    }

    #[test]
    fn trim_runaway_respects_char_boundaries() {
        let input = "ä";
        let output = "ü".repeat(300); // 600 bytes, no newline
        let trimmed = trim_runaway(&output, input);
        assert!(trimmed.len() <= input.len() + 200);
        assert!(trimmed.chars().all(|c| c == 'ü'));
    }

    #[test]
    fn or_original_keeps_input_on_empty_output() {
        assert_eq!(or_original("  ".to_string(), "orig"), "orig");
        assert_eq!(or_original("new".to_string(), "orig"), "new");
    }

    // ── strip_think_tags tests ──────────────────────────────────

    #[test]
    fn strip_think_no_tags() {
        assert_eq!(strip_think_tags("Hello world."), "Hello world.");
    }

    #[test]
    fn strip_think_complete_block() {
        let input = "<think>\nsome reasoning\n</think>\n\nCorrected text.";
        assert_eq!(strip_think_tags(input), "Corrected text.");
    }

    #[test]
    fn strip_think_unclosed_tag() {
        let input = "<think>\nsome reasoning that never ends";
        assert_eq!(strip_think_tags(input), "");
    }

    #[test]
    fn strip_think_empty_block() {
        let input = "<think>\n\n</think>\n\nThe answer.";
        assert_eq!(strip_think_tags(input), "The answer.");
    }

    #[test]
    fn strip_think_preserves_content_before() {
        let input = "Before <think>reasoning</think> After";
        assert_eq!(strip_think_tags(input), "Before  After");
    }
}
