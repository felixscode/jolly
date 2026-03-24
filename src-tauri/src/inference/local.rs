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

/// Number of sentences per LLM call. Increase for speed, decrease for quality.
const SENTENCES_PER_BATCH: usize = 4;
const MAX_PARALLEL: usize = 8;
const PER_SEQ_CTX: u32 = 2048;
const MAX_GEN_TOKENS: i32 = 1024;

/// Regex matching sentence-ending punctuation followed by whitespace.
static SENTENCE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[.!?]\s+").unwrap());

/// Initialize and load a GGUF model from the given path.
pub fn init_model(model_path: &Path, model_id: &str, prompt_template: Option<&str>) -> Result<(), String> {
    swap_model(model_path, model_id, prompt_template)
}

/// Load a new model, replacing any currently loaded model.
pub fn swap_model(model_path: &Path, model_id: &str, prompt_template: Option<&str>) -> Result<(), String> {
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
    *slot = Some((model, model_id.to_string(), prompt_template.map(|s| s.to_string())));
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
    MODEL.read().map(|slot| slot.is_some()).unwrap_or(false)
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

/// Strip `<think>...</think>` blocks from model output.
/// Some models (Qwen3, Qwen3.5) emit reasoning blocks before the answer.
fn strip_think_tags(output: &str) -> String {
    // Fast path: no think tags
    if !output.contains("<think>") {
        return output.to_string();
    }
    // Remove everything between <think> and </think> (inclusive)
    let mut result = output.to_string();
    while let Some(start) = result.find("<think>") {
        if let Some(end) = result.find("</think>") {
            let end = end + "</think>".len();
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
    output[..max_len].to_string()
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
    model
        .apply_chat_template(template, &[msg], true)
        .map_err(|e| format!("Failed to apply chat template: {}", e))
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
    eprintln!("[jolly] run_inference called with: {:?}", text);

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

    // Context and generation limits
    let n_ctx: u32 = 2048;
    let max_tokens: i32 = 1024;

    // Ensure prompt fits in context window
    if tokens.len() as u32 >= n_ctx {
        return Err(format!(
            "Input too long: {} tokens exceeds context window of {}",
            tokens.len(),
            n_ctx
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

    for _ in 0..max_tokens {
        let token = sampler.sample(&ctx, batch.n_tokens() - 1);
        sampler.accept(token);

        if model.is_eog_token(token) {
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
            break;
        }

        // Stop runaway generation: output should never be much longer than input
        if output.len() > input_len + 200 {
            output = trim_runaway(&output, text);
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

    let result = strip_think_tags(output.trim());
    eprintln!("[jolly] Returning: {:?}", result);
    Ok(result)
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
                original_text: text.clone(),
                input_len: text.len(),
                prompt_kind,
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
                if let Err(e) = ctx.clear_kv_cache_seq(Some(seq.seq_id as u32), None, None) {
                    eprintln!("[jolly] Failed to clear KV cache for seq {}: {}", seq.seq_id, e);
                }
                continue;
            }

            // Stop runaway generation
            if seq.output.len() > seq.input_len + 200 {
                seq.output = trim_runaway(&seq.output, &seq.original_text);
                seq.done = true;
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

    // Return results: use output if non-empty, else original text
    Ok(seqs
        .into_iter()
        .map(|s| {
            let trimmed = strip_think_tags(s.output.trim());
            if trimmed.is_empty() {
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
