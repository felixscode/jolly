use std::path::Path;
use std::sync::{LazyLock, RwLock};

use regex::Regex;

use async_trait::async_trait;
use mistralrs::{
    GgufModelBuilder, Model, TextMessageRole, TextMessages,
};

use super::LLMProvider;
use super::SYSTEM_PROMPT;

/// Global model handle. RwLock so models can be swapped at runtime.
static MODEL: RwLock<Option<Model>> = RwLock::new(None);

/// Number of sentences per LLM call. Increase for speed, decrease for quality.
const SENTENCES_PER_BATCH: usize = 1;

/// Regex matching sentence-ending punctuation followed by whitespace.
static SENTENCE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[.!?]\s+").unwrap());

/// Initialize and load a GGUF model from the given path.
/// Call this once during app startup. For subsequent model changes use `swap_model`.
pub fn init_model(model_path: &Path) -> Result<(), String> {
    swap_model(model_path)
}

/// Load a new model, replacing any currently loaded model.
/// Unloads the old model first to free memory before loading the new one.
pub fn swap_model(model_path: &Path) -> Result<(), String> {
    // Drop the old model first to free memory
    {
        let mut slot = MODEL.write().map_err(|e| format!("Model lock poisoned: {}", e))?;
        *slot = None;
    }

    // GgufModelBuilder expects (model_dir, [file_names])
    let dir = model_path
        .parent()
        .ok_or_else(|| format!("Invalid model path: {}", model_path.display()))?
        .to_string_lossy()
        .to_string();
    let file_name = model_path
        .file_name()
        .ok_or_else(|| format!("No file name in path: {}", model_path.display()))?
        .to_string_lossy()
        .to_string();

    // Build model synchronously using a temporary tokio runtime
    // (this function is called from spawn_blocking)
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| format!("Failed to create runtime: {}", e))?;

    // Try default device (GPU if compiled with cuda/metal).
    // mistralrs may panic on CUDA driver mismatch, so catch that.
    let gpu_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        rt.block_on(async {
            GgufModelBuilder::new(&dir, vec![&file_name])
                .with_logging()
                .build()
                .await
        })
    }));

    let model = match gpu_result {
        Ok(Ok(m)) => {
            eprintln!("[jolly] Model loaded with default device (GPU if available)");
            m
        }
        Ok(Err(e)) => {
            eprintln!("[jolly] GPU init failed: {e}");
            eprintln!("[jolly] Falling back to CPU inference");
            rt.block_on(async {
                GgufModelBuilder::new(&dir, vec![&file_name])
                    .with_logging()
                    .with_force_cpu()
                    .build()
                    .await
            })
            .map_err(|e| format!("Failed to load model on CPU: {}", e))?
        }
        Err(panic_info) => {
            let msg = panic_info
                .downcast_ref::<String>()
                .map(|s| s.as_str())
                .or_else(|| panic_info.downcast_ref::<&str>().copied())
                .unwrap_or("unknown panic");
            eprintln!("[jolly] GPU init panicked: {msg}");
            eprintln!("[jolly] Falling back to CPU inference");
            rt.block_on(async {
                GgufModelBuilder::new(&dir, vec![&file_name])
                    .with_logging()
                    .with_force_cpu()
                    .build()
                    .await
            })
            .map_err(|e| format!("Failed to load model on CPU: {}", e))?
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

            let num_batches = (parts.len() + SENTENCES_PER_BATCH - 1) / SENTENCES_PER_BATCH;
            eprintln!(
                "[jolly] Splitting text into {} sentences ({} batches at SENTENCES_PER_BATCH={})",
                parts.len(),
                num_batches,
                SENTENCES_PER_BATCH,
            );

            let mut corrected_parts: Vec<String> = Vec::new();

            for batch in parts.chunks(SENTENCES_PER_BATCH) {
                let batch_text: String = batch
                    .iter()
                    .map(|(sentence, _)| *sentence)
                    .collect::<Vec<_>>()
                    .join(" ");
                let corrected = run_inference(&batch_text)?;
                corrected_parts.push(corrected);
            }

            // Reassemble using original separators between batches.
            // Each batch's trailing separator comes from its last sentence's
            // original separator — this is the whitespace that sat between
            // this batch's end and the next batch's start in the input.
            // Note: when SENTENCES_PER_BATCH > 1, separators *within* a batch
            // are replaced by a single space (the join above). Only inter-batch
            // separators are preserved exactly.
            let mut result = String::new();
            for (i, corrected) in corrected_parts.iter().enumerate() {
                result.push_str(corrected);
                let batch_end_idx =
                    ((i + 1) * SENTENCES_PER_BATCH).min(parts.len()) - 1;
                let (_, sep) = parts[batch_end_idx];
                if !sep.is_empty() {
                    result.push_str(sep);
                }
            }

            Ok(result)
        })
        .await
        .map_err(|e| format!("Inference task failed: {}", e))?
    }
}

/// Runs inference synchronously. Called inside spawn_blocking.
pub fn run_inference(text: &str) -> Result<String, String> {
    eprintln!("[jolly] run_inference called with: {:?}", text);

    let model_guard = MODEL
        .read()
        .map_err(|e| format!("Model lock poisoned: {}", e))?;
    let model = model_guard
        .as_ref()
        .ok_or("Local model not loaded. Download a model in Settings.")?;

    let messages = TextMessages::new()
        .add_message(
            TextMessageRole::User,
            format!("{}\n\n{}", SYSTEM_PROMPT, text),
        );

    // Use a temporary runtime for the async send_chat_request
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| format!("Failed to create runtime: {}", e))?;

    let response = rt
        .block_on(model.send_chat_request(messages))
        .map_err(|e| {
            eprintln!("[jolly] Inference error: {}", e);
            format!("Inference failed: {}", e)
        })?;

    eprintln!("[jolly] Response choices: {}", response.choices.len());
    let content = response
        .choices
        .first()
        .and_then(|c| {
            eprintln!("[jolly] Choice content: {:?}", c.message.content);
            c.message.content.as_deref()
        })
        .unwrap_or("")
        .trim()
        .to_string();

    eprintln!("[jolly] Returning: {:?}", content);
    Ok(content)
}
