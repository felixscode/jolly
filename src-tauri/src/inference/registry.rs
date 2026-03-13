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
}

pub static MODELS: &[ModelEntry] = &[
    ModelEntry {
        id: "qwen2.5-1.5b-instruct-q4km",
        name: "Qwen 2.5 1.5B Instruct",
        file_name: "Qwen2.5-1.5B-Instruct-Q4_K_M.gguf",
        url: "https://huggingface.co/bartowski/Qwen2.5-1.5B-Instruct-GGUF/resolve/main/Qwen2.5-1.5B-Instruct-Q4_K_M.gguf",
        size_bytes: 1_063_000_000,
        sha256: "",
    },
    ModelEntry {
        id: "qwen2.5-3b-instruct-q4km",
        name: "Qwen 2.5 3B Instruct",
        file_name: "Qwen2.5-3B-Instruct-Q4_K_M.gguf",
        url: "https://huggingface.co/bartowski/Qwen2.5-3B-Instruct-GGUF/resolve/main/Qwen2.5-3B-Instruct-Q4_K_M.gguf",
        size_bytes: 2_073_000_000,
        sha256: "",
    },
    ModelEntry {
        id: "phi-3.5-mini-instruct-q4km",
        name: "Phi 3.5 Mini Instruct",
        file_name: "Phi-3.5-mini-instruct-Q4_K_M.gguf",
        url: "https://huggingface.co/bartowski/Phi-3.5-mini-instruct-GGUF/resolve/main/Phi-3.5-mini-instruct-Q4_K_M.gguf",
        size_bytes: 2_566_000_000,
        sha256: "",
    },
    ModelEntry {
        id: "gemma-2-2b-it-q4km",
        name: "Gemma 2 2B IT",
        file_name: "gemma-2-2b-it-Q4_K_M.gguf",
        url: "https://huggingface.co/bartowski/gemma-2-2b-it-GGUF/resolve/main/gemma-2-2b-it-Q4_K_M.gguf",
        size_bytes: 1_836_000_000,
        sha256: "",
    },
    ModelEntry {
        id: "mistral-7b-instruct-v0.3-q4km",
        name: "Mistral 7B Instruct v0.3",
        file_name: "Mistral-7B-Instruct-v0.3-Q4_K_M.gguf",
        url: "https://huggingface.co/bartowski/Mistral-7B-Instruct-v0.3-GGUF/resolve/main/Mistral-7B-Instruct-v0.3-Q4_K_M.gguf",
        size_bytes: 4_692_000_000,
        sha256: "",
    },
];

pub fn find_model(id: &str) -> Option<&'static ModelEntry> {
    MODELS.iter().find(|m| m.id == id)
}
