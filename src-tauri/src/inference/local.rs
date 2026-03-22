use std::num::NonZeroU32;
use std::path::Path;
use std::sync::{LazyLock, RwLock};

use regex::Regex;

use async_trait::async_trait;
use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::model::{AddBos, LlamaChatMessage, LlamaModel};
use llama_cpp_2::sampling::LlamaSampler;
use llama_cpp_2::token::LlamaToken;

use super::LLMProvider;
use super::SYSTEM_PROMPT;

/// Llama.cpp backend handle. Initialized once on first use.
static BACKEND: LazyLock<LlamaBackend> = LazyLock::new(|| {
    LlamaBackend::init().expect("Failed to initialize llama backend")
});

/// Global model handle. RwLock so models can be swapped at runtime.
/// Note: swap_model takes a write lock; run_inference takes a read lock.
/// Model swaps block until in-flight inferences complete.
static MODEL: RwLock<Option<LlamaModel>> = RwLock::new(None);

/// Number of sentences per LLM call. Increase for speed, decrease for quality.
const SENTENCES_PER_BATCH: usize = 4;
const MAX_PARALLEL: usize = 8;
const PER_SEQ_CTX: u32 = 2048;
const MAX_GEN_TOKENS: i32 = 1024;

/// Regex matching sentence-ending punctuation followed by whitespace.
static SENTENCE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[.!?]\s+").unwrap());

/// Initialize and load a GGUF model from the given path.
pub fn init_model(model_path: &Path) -> Result<(), String> {
    swap_model(model_path)
}

/// Load a new model, replacing any currently loaded model.
pub fn swap_model(model_path: &Path) -> Result<(), String> {
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
    *slot = Some(model);
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

/// Runs inference synchronously. Called inside spawn_blocking.
pub fn run_inference(text: &str) -> Result<String, String> {
    eprintln!("[jolly] run_inference called with: {:?}", text);

    let backend = &*BACKEND;

    let model_guard = MODEL
        .read()
        .map_err(|e| format!("Model lock poisoned: {}", e))?;
    let model = model_guard
        .as_ref()
        .ok_or("Local model not loaded. Download a model in Settings.")?;

    // Format prompt using the model's built-in chat template
    let prompt = format!("{}\n\n{}", SYSTEM_PROMPT, text);
    let formatted = match model.chat_template(None) {
        Ok(tmpl) => {
            let msg = LlamaChatMessage::new(
                "user".to_string(),
                prompt.clone(),
            ).map_err(|e| format!("Failed to create chat message: {}", e))?;
            model
                .apply_chat_template(&tmpl, &[msg], true)
                .map_err(|e| format!("Failed to apply chat template: {}", e))?
        }
        Err(_) => {
            eprintln!("[jolly] No chat template found, using raw prompt");
            prompt
        }
    };

    // Context and generation limits
    let n_ctx: u32 = 2048;
    let max_tokens: i32 = 1024;

    // Create context for this inference call
    let ctx_params = LlamaContextParams::default()
        .with_n_ctx(NonZeroU32::new(n_ctx));
    let mut ctx = model
        .new_context(backend, ctx_params)
        .map_err(|e| format!("Failed to create context: {}", e))?;

    // Tokenize
    let tokens = model
        .str_to_token(&formatted, AddBos::Always)
        .map_err(|e| format!("Failed to tokenize: {}", e))?;

    eprintln!("[jolly] Input tokens: {}", tokens.len());

    if tokens.is_empty() {
        return Ok(String::new());
    }

    // Ensure prompt fits in context window
    if tokens.len() as u32 >= n_ctx {
        return Err(format!(
            "Input too long: {} tokens exceeds context window of {}",
            tokens.len(),
            n_ctx
        ));
    }

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
}
