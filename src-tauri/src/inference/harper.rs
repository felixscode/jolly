use async_trait::async_trait;

use super::LLMProvider;

pub struct HarperProvider;

impl HarperProvider {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl LLMProvider for HarperProvider {
    async fn correct_text(&self, text: &str) -> Result<String, String> {
        todo!()
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
}
