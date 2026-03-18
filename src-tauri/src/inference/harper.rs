use std::sync::Arc;

use async_trait::async_trait;
use harper_core::linting::{LintGroup, Linter};
use harper_core::spell::FstDictionary;
use harper_core::Document;

use super::LLMProvider;

pub struct HarperProvider;

impl HarperProvider {
    pub fn new() -> Self {
        Self
    }
}

/// Run Harper grammar/spelling check and auto-apply first suggestion for each lint.
fn harper_correct(text: &str) -> Result<String, String> {
    if text.is_empty() {
        return Ok(String::new());
    }

    let dict = FstDictionary::curated();
    let document = Document::new_plain_english(text, &dict);

    let mut linter = LintGroup::new_curated(Arc::clone(&dict), harper_core::Dialect::American);
    let mut lints = linter.lint(&document);

    if lints.is_empty() {
        return Ok(text.to_string());
    }

    // Sort by span start descending so we apply from back to front,
    // preserving character offsets for earlier spans
    lints.sort_by(|a, b| b.span.start.cmp(&a.span.start));

    let mut chars: Vec<char> = text.chars().collect();

    for lint in &lints {
        if let Some(suggestion) = lint.suggestions.first() {
            suggestion.apply(lint.span, &mut chars);
        }
    }

    Ok(chars.into_iter().collect())
}

#[async_trait]
impl LLMProvider for HarperProvider {
    async fn correct_text(&self, text: &str) -> Result<String, String> {
        let text = text.to_string();
        tokio::task::spawn_blocking(move || harper_correct(&text))
            .await
            .map_err(|e| format!("Harper task failed: {}", e))?
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn empty_input_returns_empty() {
        let provider = HarperProvider::new();
        let result = provider.correct_text("").await.unwrap();
        assert_eq!(result, "");
    }

    #[tokio::test]
    async fn correct_text_unchanged() {
        let provider = HarperProvider::new();
        let result = provider.correct_text("This is correct.").await.unwrap();
        assert_eq!(result, "This is correct.");
    }

    #[tokio::test]
    async fn fixes_spelling_error() {
        let provider = HarperProvider::new();
        let result = provider.correct_text("This is an tset.").await.unwrap();
        assert_ne!(result, "This is an tset.");
    }

    #[tokio::test]
    async fn preserves_multiline_text() {
        let provider = HarperProvider::new();
        let input = "First line.\nSecond line.";
        let result = provider.correct_text(input).await.unwrap();
        assert_eq!(result, input);
    }

    #[test]
    fn harper_correct_empty() {
        assert_eq!(harper_correct("").unwrap(), "");
    }

    #[test]
    fn harper_correct_no_errors() {
        let result = harper_correct("The cat sat on the mat.").unwrap();
        assert_eq!(result, "The cat sat on the mat.");
    }

    #[test]
    fn harper_correct_returns_ok() {
        // Smoke test: Harper doesn't panic on arbitrary input
        let result = harper_correct("somthing is wrng here");
        assert!(result.is_ok());
    }
}
