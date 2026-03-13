pub mod download;
pub mod local;
pub mod model_manager;
pub mod openrouter;
pub mod registry;

use async_trait::async_trait;

/// System prompt shared by all providers for consistent correction behavior.
pub const SYSTEM_PROMPT: &str =
    "You are a spell checker. Return ONLY the corrected text, no commentary. Preserve the user's writing style and voice.";

/// Trait implemented by all text correction providers.
#[async_trait]
pub trait LLMProvider: Send + Sync {
    async fn correct_text(&self, text: &str) -> Result<String, String>;
}
