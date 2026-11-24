//! Provider-agnostic remote inference (OpenAI-compatible APIs and Anthropic).

use async_trait::async_trait;
use serde::Deserialize;

use super::providers::{find_provider, ApiKind, ACCOUNT_ID_PLACEHOLDER};
use super::{LLMProvider, SYSTEM_PROMPT};

const ANTHROPIC_VERSION: &str = "2023-06-01";
const TIMEOUT_SECS: u64 = 60;

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Deserialize)]
struct ChatChoice {
    message: ChatMessage,
}

#[derive(Deserialize)]
struct ChatMessage {
    content: Option<String>,
}

#[derive(Deserialize)]
struct AnthropicResponse {
    content: Vec<AnthropicBlock>,
}

#[derive(Deserialize)]
struct AnthropicBlock {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    text: String,
}

pub struct RemoteProvider {
    pub kind: ApiKind,
    pub base_url: String,
    pub model: String,
    pub api_key: Option<String>,
}

impl RemoteProvider {
    pub fn new(kind: ApiKind, base_url: String, model: String, api_key: Option<String>) -> Self {
        Self {
            kind,
            base_url: base_url.trim().trim_end_matches('/').to_string(),
            model: model.trim().to_string(),
            api_key: api_key.filter(|k| !k.trim().is_empty()),
        }
    }

    /// OpenRouter with its preset defaults (used by the benchmark binary).
    pub fn openrouter(api_key: String) -> Self {
        let p = find_provider("openrouter").expect("openrouter preset");
        Self::new(
            p.kind,
            p.base_url.into(),
            p.default_model.into(),
            Some(api_key),
        )
    }

    fn endpoint(&self) -> String {
        match self.kind {
            ApiKind::OpenAiCompatible => format!("{}/chat/completions", self.base_url),
            ApiKind::Anthropic => {
                // Accept both "https://api.anthropic.com" and ".../v1".
                if self.base_url.ends_with("/v1") {
                    format!("{}/messages", self.base_url)
                } else {
                    format!("{}/v1/messages", self.base_url)
                }
            }
        }
    }

    fn body(&self, text: &str) -> serde_json::Value {
        match self.kind {
            ApiKind::OpenAiCompatible => serde_json::json!({
                "model": self.model,
                "temperature": 0,
                "messages": [
                    { "role": "system", "content": SYSTEM_PROMPT },
                    { "role": "user", "content": text }
                ]
            }),
            ApiKind::Anthropic => serde_json::json!({
                "model": self.model,
                "max_tokens": 4096,
                "temperature": 0,
                "system": SYSTEM_PROMPT,
                "messages": [ { "role": "user", "content": text } ]
            }),
        }
    }
}

/// Map an HTTP error to one of the error codes the UI understands.
/// The returned string starts with the code, followed by details.
pub fn classify_http_error(status: u16, body: &str) -> String {
    let lower = body.to_lowercase();
    let code = match status {
        401 | 403 => "bad_api_key",
        404 => "bad_model",
        400 | 422 if lower.contains("model") => "bad_model",
        _ => "api_failed",
    };
    format!("{code}: HTTP {status}: {body}")
}

fn parse_response(kind: ApiKind, body: &str) -> Result<String, String> {
    let text = match kind {
        ApiKind::OpenAiCompatible => {
            let data: ChatResponse = serde_json::from_str(body)
                .map_err(|e| format!("api_failed: failed to parse response: {e}"))?;
            data.choices
                .into_iter()
                .next()
                .and_then(|c| c.message.content)
                .unwrap_or_default()
        }
        ApiKind::Anthropic => {
            let data: AnthropicResponse = serde_json::from_str(body)
                .map_err(|e| format!("api_failed: failed to parse response: {e}"))?;
            data.content
                .into_iter()
                .filter(|b| b.kind == "text")
                .map(|b| b.text)
                .collect::<String>()
        }
    };
    let text = text.trim().to_string();
    if text.is_empty() {
        return Err("api_failed: empty response".to_string());
    }
    Ok(text)
}

#[async_trait]
impl LLMProvider for RemoteProvider {
    async fn correct_text(&self, text: &str) -> Result<String, String> {
        if self.base_url.contains(ACCOUNT_ID_PLACEHOLDER) {
            return Err("bad_base_url: replace <ACCOUNT_ID> in the base URL".to_string());
        }
        if self.model.is_empty() {
            return Err("bad_model: no model set".to_string());
        }

        let client = reqwest::Client::new();
        let mut req = client
            .post(self.endpoint())
            .timeout(std::time::Duration::from_secs(TIMEOUT_SECS))
            .json(&self.body(text));

        req = match (self.kind, &self.api_key) {
            (ApiKind::Anthropic, Some(key)) => req
                .header("x-api-key", key)
                .header("anthropic-version", ANTHROPIC_VERSION),
            (ApiKind::Anthropic, None) => req.header("anthropic-version", ANTHROPIC_VERSION),
            (ApiKind::OpenAiCompatible, Some(key)) => req.bearer_auth(key),
            (ApiKind::OpenAiCompatible, None) => req,
        };

        let response = req
            .send()
            .await
            .map_err(|e| format!("api_failed: request failed: {e}"))?;

        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(classify_http_error(status.as_u16(), &body));
        }
        parse_response(self.kind, &body)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    /// One-shot HTTP server: returns (base_url, handle resolving to the raw request).
    async fn mock(status: u16, body: &'static str) -> (String, tokio::task::JoinHandle<String>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let handle = tokio::spawn(async move {
            let (mut sock, _) = listener.accept().await.unwrap();
            let mut buf = Vec::new();
            let mut chunk = [0u8; 4096];
            // Read headers, then the body according to Content-Length.
            loop {
                let n = sock.read(&mut chunk).await.unwrap();
                buf.extend_from_slice(&chunk[..n]);
                let s = String::from_utf8_lossy(&buf).to_string();
                if let Some(idx) = s.find("\r\n\r\n") {
                    let len = s
                        .lines()
                        .find_map(|l| {
                            l.to_lowercase()
                                .strip_prefix("content-length:")
                                .map(|v| v.trim().parse::<usize>().unwrap())
                        })
                        .unwrap_or(0);
                    if buf.len() >= idx + 4 + len {
                        break;
                    }
                }
                if n == 0 {
                    break;
                }
            }
            let resp = format!(
                "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            sock.write_all(resp.as_bytes()).await.unwrap();
            String::from_utf8_lossy(&buf).to_string()
        });
        (format!("http://{addr}"), handle)
    }

    #[tokio::test]
    async fn openai_compatible_roundtrip() {
        let (base, req) = mock(
            200,
            r#"{"choices":[{"message":{"content":" Fixed text. "}}]}"#,
        )
        .await;
        let p = RemoteProvider::new(
            ApiKind::OpenAiCompatible,
            format!("{base}/v1/"),
            "m1".into(),
            Some("k1".into()),
        );
        assert_eq!(p.correct_text("fixd text").await.unwrap(), "Fixed text.");
        let raw = req.await.unwrap();
        assert!(raw.starts_with("POST /v1/chat/completions "));
        assert!(raw.to_lowercase().contains("authorization: bearer k1"));
        assert!(raw.contains(r#""model":"m1""#));
    }

    #[tokio::test]
    async fn keyless_custom_sends_no_auth() {
        let (base, req) = mock(200, r#"{"choices":[{"message":{"content":"ok"}}]}"#).await;
        let p = RemoteProvider::new(
            ApiKind::OpenAiCompatible,
            base,
            "m".into(),
            Some("  ".into()),
        );
        p.correct_text("x").await.unwrap();
        assert!(!req.await.unwrap().to_lowercase().contains("authorization"));
    }

    #[tokio::test]
    async fn anthropic_roundtrip() {
        let (base, req) = mock(
            200,
            r#"{"content":[{"type":"text","text":"Hello "},{"type":"text","text":"world."}]}"#,
        )
        .await;
        let p = RemoteProvider::new(
            ApiKind::Anthropic,
            base,
            "claude-x".into(),
            Some("ak".into()),
        );
        assert_eq!(p.correct_text("helo wrld").await.unwrap(), "Hello world.");
        let raw = req.await.unwrap();
        let lower = raw.to_lowercase();
        assert!(raw.starts_with("POST /v1/messages "));
        assert!(lower.contains("x-api-key: ak"));
        assert!(lower.contains("anthropic-version: 2023-06-01"));
        assert!(raw.contains(r#""max_tokens":4096"#));
    }

    #[tokio::test]
    async fn http_errors_are_classified() {
        let (base, _r) = mock(401, r#"{"error":"nope"}"#).await;
        let p = RemoteProvider::new(
            ApiKind::OpenAiCompatible,
            base,
            "m".into(),
            Some("k".into()),
        );
        assert!(p
            .correct_text("x")
            .await
            .unwrap_err()
            .starts_with("bad_api_key"));

        let (base, _r) = mock(400, r#"{"error":{"message":"model foo does not exist"}}"#).await;
        let p = RemoteProvider::new(
            ApiKind::OpenAiCompatible,
            base,
            "foo".into(),
            Some("k".into()),
        );
        assert!(p
            .correct_text("x")
            .await
            .unwrap_err()
            .starts_with("bad_model"));

        let (base, _r) = mock(500, "boom").await;
        let p = RemoteProvider::new(
            ApiKind::OpenAiCompatible,
            base,
            "m".into(),
            Some("k".into()),
        );
        assert!(p
            .correct_text("x")
            .await
            .unwrap_err()
            .starts_with("api_failed"));
    }

    #[tokio::test]
    async fn unfilled_account_id_is_rejected() {
        let p = RemoteProvider::new(
            ApiKind::OpenAiCompatible,
            "https://api.cloudflare.com/client/v4/accounts/<ACCOUNT_ID>/ai/v1".into(),
            "m".into(),
            Some("k".into()),
        );
        assert!(p
            .correct_text("x")
            .await
            .unwrap_err()
            .starts_with("bad_base_url"));
    }
}
