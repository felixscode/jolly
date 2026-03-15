use std::path::Path;
use std::sync::RwLock;

use async_trait::async_trait;
use mistralrs::{
    GgufModelBuilder, Model, TextMessageRole, TextMessages,
};

use super::LLMProvider;
use super::SYSTEM_PROMPT;

/// Global model handle. RwLock so models can be swapped at runtime.
static MODEL: RwLock<Option<Model>> = RwLock::new(None);

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
        tauri::async_runtime::spawn_blocking(move || run_inference(&text))
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
