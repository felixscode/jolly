use serde::Serialize;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelEntry {
    pub id: &'static str,
    pub name: &'static str,
    pub file_name: &'static str,
    pub url: &'static str,
    pub size_bytes: u64,
    pub sha256: &'static str,
    /// Optional prompt template for models without an embedded GGUF chat template.
    /// Uses `{text}` as placeholder for user input.
    #[serde(skip)]
    pub prompt_template: Option<&'static str>,
}

pub static MODELS: &[ModelEntry] = &[
    // ── Grammar-specialized models (GRMR V3 family) ─────────────
    // All use the text/corrected format via embedded GGUF chat template.
    ModelEntry {
        id: "grmr-v3-l3b-q4km",
        name: "GRMR V3 3B",
        file_name: "GRMR-V3-L3B-Q4_K_M.gguf",
        url: "https://huggingface.co/qingy2024/GRMR-V3-L3B-GGUF/resolve/main/GRMR-V3-L3B-Q4_K_M.gguf",
        size_bytes: 2_019_374_624,
        sha256: "",
        prompt_template: None,
    },
    ModelEntry {
        id: "grmr-v3-g4b-q4km",
        name: "GRMR V3 4B (Recommended)",
        file_name: "GRMR-V3-G4B-Q4_K_M.gguf",
        url: "https://huggingface.co/qingy2024/GRMR-V3-G4B-GGUF/resolve/main/GRMR-V3-G4B-Q4_K_M.gguf",
        size_bytes: 2_489_892_960,
        sha256: "",
        prompt_template: None,
    },
    // ── General-purpose instruct models ──────────────────────────
    ModelEntry {
        id: "gemma-3-4b-it-q4km",
        name: "Gemma 3 4B Instruct",
        file_name: "google_gemma-3-4b-it-Q4_K_M.gguf",
        url: "https://huggingface.co/bartowski/google_gemma-3-4b-it-GGUF/resolve/main/google_gemma-3-4b-it-Q4_K_M.gguf",
        size_bytes: 2_489_758_112,
        sha256: "",
        prompt_template: None,
    },
    ModelEntry {
        id: "mistral-7b-instruct-v0.3-q4km",
        name: "Mistral 7B Instruct v0.3 (Best Multilingual)",
        file_name: "Mistral-7B-Instruct-v0.3-Q4_K_M.gguf",
        url: "https://huggingface.co/bartowski/Mistral-7B-Instruct-v0.3-GGUF/resolve/main/Mistral-7B-Instruct-v0.3-Q4_K_M.gguf",
        size_bytes: 4_692_000_000,
        sha256: "",
        prompt_template: None,
    },
];

pub fn find_model(id: &str) -> Option<&'static ModelEntry> {
    MODELS.iter().find(|m| m.id == id)
}
