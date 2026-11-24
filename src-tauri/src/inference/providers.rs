//! Presets for remote (cloud) LLM providers.
//!
//! Every provider except Anthropic speaks the OpenAI chat-completions dialect,
//! so adding one is usually just a new table entry.

use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ApiKind {
    /// `POST {base}/chat/completions`, Bearer auth.
    OpenAiCompatible,
    /// `POST {base}/v1/messages`, `x-api-key` auth.
    Anthropic,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderPreset {
    pub id: &'static str,
    pub name: &'static str,
    pub kind: ApiKind,
    pub base_url: &'static str,
    pub default_model: &'static str,
    /// Environment variable used as a fallback when no key is stored.
    pub key_env: &'static str,
    pub key_placeholder: &'static str,
    pub signup_url: &'static str,
    /// Local servers (Ollama, LM Studio, …) usually need no key.
    pub needs_key: bool,
}

/// Placeholder a base URL may contain that the user has to fill in.
pub const ACCOUNT_ID_PLACEHOLDER: &str = "<ACCOUNT_ID>";

pub static PROVIDERS: &[ProviderPreset] = &[
    ProviderPreset {
        id: "openrouter",
        name: "OpenRouter",
        kind: ApiKind::OpenAiCompatible,
        base_url: "https://openrouter.ai/api/v1",
        default_model: "openai/gpt-4o-mini",
        key_env: "OPENROUTER_API_KEY",
        key_placeholder: "sk-or-...",
        signup_url: "https://openrouter.ai/keys",
        needs_key: true,
    },
    ProviderPreset {
        id: "openai",
        name: "OpenAI",
        kind: ApiKind::OpenAiCompatible,
        base_url: "https://api.openai.com/v1",
        default_model: "gpt-4o-mini",
        key_env: "OPENAI_API_KEY",
        key_placeholder: "sk-...",
        signup_url: "https://platform.openai.com/api-keys",
        needs_key: true,
    },
    ProviderPreset {
        id: "anthropic",
        name: "Anthropic",
        kind: ApiKind::Anthropic,
        base_url: "https://api.anthropic.com",
        default_model: "claude-haiku-5-5",
        key_env: "ANTHROPIC_API_KEY",
        key_placeholder: "sk-ant-...",
        signup_url: "https://console.anthropic.com/settings/keys",
        needs_key: true,
    },
    ProviderPreset {
        id: "mistral",
        name: "Mistral",
        kind: ApiKind::OpenAiCompatible,
        base_url: "https://api.mistral.ai/v1",
        default_model: "mistral-small-latest",
        key_env: "MISTRAL_API_KEY",
        key_placeholder: "...",
        signup_url: "https://console.mistral.ai/api-keys",
        needs_key: true,
    },
    ProviderPreset {
        id: "qwen",
        name: "Qwen (Alibaba DashScope)",
        kind: ApiKind::OpenAiCompatible,
        base_url: "https://dashscope-intl.aliyuncs.com/compatible-mode/v1",
        default_model: "qwen-plus",
        key_env: "DASHSCOPE_API_KEY",
        key_placeholder: "sk-...",
        signup_url: "https://modelstudio.console.alibabacloud.com/",
        needs_key: true,
    },
    ProviderPreset {
        id: "cloudflare",
        name: "Cloudflare Workers AI",
        kind: ApiKind::OpenAiCompatible,
        base_url: "https://api.cloudflare.com/client/v4/accounts/<ACCOUNT_ID>/ai/v1",
        default_model: "@cf/meta/llama-3.3-70b-instruct-fp8-fast",
        key_env: "CLOUDFLARE_API_TOKEN",
        key_placeholder: "Cloudflare API token",
        signup_url: "https://dash.cloudflare.com/profile/api-tokens",
        needs_key: true,
    },
    ProviderPreset {
        id: "vercel",
        name: "Vercel AI Gateway",
        kind: ApiKind::OpenAiCompatible,
        base_url: "https://ai-gateway.vercel.sh/v1",
        default_model: "openai/gpt-4o-mini",
        key_env: "AI_GATEWAY_API_KEY",
        key_placeholder: "AI Gateway API key",
        signup_url: "https://vercel.com/docs/ai-gateway",
        needs_key: true,
    },
    ProviderPreset {
        id: "groq",
        name: "Groq",
        kind: ApiKind::OpenAiCompatible,
        base_url: "https://api.groq.com/openai/v1",
        default_model: "llama-3.1-8b-instant",
        key_env: "GROQ_API_KEY",
        key_placeholder: "gsk_...",
        signup_url: "https://console.groq.com/keys",
        needs_key: true,
    },
    ProviderPreset {
        id: "gemini",
        name: "Google Gemini",
        kind: ApiKind::OpenAiCompatible,
        base_url: "https://generativelanguage.googleapis.com/v1beta/openai",
        default_model: "gemini-2.5-flash",
        key_env: "GEMINI_API_KEY",
        key_placeholder: "AIza...",
        signup_url: "https://aistudio.google.com/apikey",
        needs_key: true,
    },
    ProviderPreset {
        id: "custom",
        name: "Custom (OpenAI-compatible)",
        kind: ApiKind::OpenAiCompatible,
        base_url: "http://localhost:11434/v1",
        default_model: "llama3.2",
        key_env: "JOLLY_CUSTOM_API_KEY",
        key_placeholder: "optional",
        signup_url: "",
        needs_key: false,
    },
];

pub fn find_provider(id: &str) -> Option<&'static ProviderPreset> {
    PROVIDERS.iter().find(|p| p.id == id)
}

/// Keyring account name for a provider's API key. OpenRouter keeps its
/// historical name so keys saved by older versions keep working.
pub fn keyring_account(provider_id: &str) -> String {
    if provider_id == "openrouter" {
        "openrouter_api_key".to_string()
    } else {
        format!("api_key_{provider_id}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique() {
        let mut ids: Vec<_> = PROVIDERS.iter().map(|p| p.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), PROVIDERS.len());
    }

    #[test]
    fn openrouter_keeps_legacy_keyring_account() {
        assert_eq!(keyring_account("openrouter"), "openrouter_api_key");
        assert_eq!(keyring_account("openai"), "api_key_openai");
    }
}
