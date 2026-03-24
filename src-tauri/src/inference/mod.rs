pub mod download;
pub mod harper;
pub mod local;
pub mod model_manager;
pub mod openrouter;
pub mod registry;

use async_trait::async_trait;

/// System prompt shared by all providers for consistent correction behavior.
pub const SYSTEM_PROMPT: &str =
    "Fix spelling and grammar errors. Rules: return ONLY the corrected text. No commentary, no explanations, no prefixes, no thinking. Keep the SAME language as the input — if the input is German, output German; if English, output English. Never translate. Preserve formatting, newlines, and tone exactly. Do not add or remove content. /no_think";

/// Trait implemented by all text correction providers.
#[async_trait]
pub trait LLMProvider: Send + Sync {
    async fn correct_text(&self, text: &str) -> Result<String, String>;
}
