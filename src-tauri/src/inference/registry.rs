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
        id: "grmr-2b-instruct-q4km",
        name: "GRMR 2B Instruct",
        file_name: "GRMR-2B-Instruct-Q4_K_M.gguf",
        url: "https://huggingface.co/bartowski/GRMR-2B-Instruct-GGUF/resolve/main/GRMR-2B-Instruct-Q4_K_M.gguf",
        size_bytes: 1_708_582_560,
        sha256: "",
    },
    ModelEntry {
        id: "grmr-v3-g4b-q2k",
        name: "GRMR V3 G4B (Q2_K)",
        file_name: "GRMR-V3-G4B-Q2_K.gguf",
        url: "https://huggingface.co/qingy2024/GRMR-V3-G4B-GGUF/resolve/main/GRMR-V3-G4B-Q2_K.gguf",
        size_bytes: 1_730_000_000,
        sha256: "",
    },
    ModelEntry {
        id: "grmr-v3-g4b-q4km",
        name: "GRMR V3 G4B (Q4_K_M)",
        file_name: "GRMR-V3-G4B-Q4_K_M.gguf",
        url: "https://huggingface.co/qingy2024/GRMR-V3-G4B-GGUF/resolve/main/GRMR-V3-G4B-Q4_K_M.gguf",
        size_bytes: 2_489_892_960,
        sha256: "",
    },
    ModelEntry {
        id: "grmr-v3-g4b-q8_0",
        name: "GRMR V3 G4B (Q8_0)",
        file_name: "GRMR-V3-G4B-Q8_0.gguf",
        url: "https://huggingface.co/qingy2024/GRMR-V3-G4B-GGUF/resolve/main/GRMR-V3-G4B-Q8_0.gguf",
        size_bytes: 4_130_000_000,
        sha256: "",
    },
    ModelEntry {
        id: "qwen3-1.7b-q4km",
        name: "Qwen3 1.7B",
        file_name: "Qwen_Qwen3-1.7B-Q4_K_M.gguf",
        url: "https://huggingface.co/bartowski/Qwen_Qwen3-1.7B-GGUF/resolve/main/Qwen_Qwen3-1.7B-Q4_K_M.gguf",
        size_bytes: 1_282_439_584,
        sha256: "",
    },
    ModelEntry {
        id: "qwen3.5-4b-q4km",
        name: "Qwen3.5 4B",
        file_name: "Qwen_Qwen3.5-4B-Q4_K_M.gguf",
        url: "https://huggingface.co/bartowski/Qwen_Qwen3.5-4B-GGUF/resolve/main/Qwen_Qwen3.5-4B-Q4_K_M.gguf",
        size_bytes: 2_871_743_520,
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
