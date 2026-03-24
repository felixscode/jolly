<div align="center">


# Jolly
<img src="static/jolly_normal.svg" width="120" alt="Jolly" />

[![Deploy Website](https://github.com/felixscode/jolly/actions/workflows/deploy-web.yml/badge.svg)](https://github.com/felixscode/jolly/actions/workflows/deploy-web.yml)
[![Release Desktop App](https://github.com/felixscode/jolly/actions/workflows/release-app.yml/badge.svg)](https://github.com/felixscode/jolly/actions/workflows/release-app.yml)
[![License](https://img.shields.io/github/license/felixscode/jolly)](https://github.com/felixscode/jolly)
[![Release](https://img.shields.io/github/v/release/felixscode/jolly)](https://github.com/felixscode/jolly/releases)

![Rust](https://img.shields.io/badge/Rust-2021-b7410e?logo=rust&logoColor=white)
![TypeScript](https://img.shields.io/badge/TypeScript-5.9-3178c6?logo=typescript&logoColor=white)
![Svelte](https://img.shields.io/badge/SvelteKit-5-ff3e00?logo=svelte&logoColor=white)
![Tailwind](https://img.shields.io/badge/Tailwind-4-06b6d4?logo=tailwindcss&logoColor=white)
![Tauri](https://img.shields.io/badge/Tauri-2-24c8d8?logo=tauri&logoColor=white)

**Local-first spell checker powered by on-device LLMs.**
Copy Enter Paste! Jolly read from your clipboard and applies changes so you can paste it back.
Nothing leaves your device.


[Features](#features) | [Installation](#installation) | [Models](#available-models) | [Benchmarks](#benchmarks) | [Screenshots](#screenshots)

</div>

## About

Spell checkers are annoying — squiggly lines and too much clicking. Pasting text into an AI with "fix spelling" works, but sending your mails and notes to LLM providers feels uneasy. Jolly does it locally and makes it fun.

Copy text, hit Enter, paste it back — corrected. Jolly reads your clipboard, passes it through a local LLM, and writes the result back. If your machine doesn't support local inference, you can use API keys or conventional grammar checking via [Harper](https://github.com/Automattic/harper). Everything runs on your device.

Built with SvelteKit, Tailwind, Tauri, and lama.cpp in Rust and TypeScript. Jolly started as a way to learn frontend development and explore the trade-offs between local LLM inference and conventional grammar checkers. On one a "simple" task

## Features

- **Privacy-first**: All inference runs locally — nothing leaves your machine
- **One-shot correction**: Copy, press Enter, paste — corrected text is in your clipboard
- **Multiple ways**: Choose from on-device LLMs, Openrouter via Api Call or Harper 

## Installation

### Pre-built Binaries (Recommended)

Download the latest release for your platform from [GitHub Releases](https://github.com/felixscode/jolly/releases):

| Platform | File | Notes |
|----------|------|-------|
| **macOS** | `Jolly_x.x.x_aarch64.dmg` | Apple Silicon (Intel via Rosetta) |
| **Windows** | `Jolly_x.x.x_x64-setup.exe` | NSIS installer |
| **Linux** | `Jolly_x.x.x_amd64.deb` | Debian/Ubuntu |
| **Linux** | `Jolly_x.x.x_amd64.AppImage` | Universal |

### Building from Source

**Prerequisites:**
- [Node.js](https://nodejs.org/) >= 18
- [Rust](https://rustup.rs/) 
- [Tauri CLI](https://v2.tauri.app/start/prerequisites/) system dependencies

```sh
git clone https://github.com/felixscode/jolly.git
cd jolly
npm install
npx tauri build
```

#### GPU Acceleration

Thanks to amazing lamacpp and vulcan Jolly detects GPU availability at runtime. If initialization fails, it silently falls back to CPU. It uses vulcan for win,linux and metal for mac.

## Available Models

The [GRMR](https://huggingface.co/qingy2024) models are specifically fine-tuned for grammar correction — they take text in and return corrected text with minimal over-editing. The general-purpose models (Gemma, Mistral) are instruction-following LLMs prompted to fix spelling and grammar.

| Model | Type | Size | Quantization |
|-------|------|------|--------------|
| GRMR V3 3B | Grammar-specialized | 2.0 GB | Q4_K_M |
| GRMR V3 4B | Grammar-specialized | 2.5 GB | Q4_K_M |
| Gemma 3 4B Instruct | General-purpose | 2.5 GB | Q4_K_M |
| Mistral 7B Instruct v0.3 | General-purpose | 4.7 GB | Q4_K_M |

## Benchmarks

Tested across 8 cases (short, medium, email) in English and German. Errors fixed measures how many spelling/grammar errors the model actually corrected out of 62 total. Inference on CPU — times will be significantly faster with a GPU.

| Model | Errors Fixed | Exact Match | Avg Latency |
|-------|:------------:|:-----------:|------------:|
| **OpenRouter gpt-4o-mini** | **62/62** | 6/8 | 1.5s |
| **Mistral 7B Instruct v0.3** | **58/62** | 4/8 | 8.0s |
| GRMR V3 4B | 54/62 | 3/8 | 4.2s |
| GRMR V3 3B | 46/62 | 2/8 | 3.5s |
| Gemma 3 4B Instruct | 44/62 | 1/8 | 4.6s |
| Harper | 36/62 | 2/8 | 0.2s |

**Which model should I use?**

- **Mistral 7B** is the best local model overall (58/62) and the only one that handles German well. It needs 4.7 GB of RAM and is the slowest local option.
- **GRMR V3 4B** is the recommended tradeoff — fast, small (2.5 GB), and fixes 87% of errors. Best choice for English-only use.
- **GRMR V3 3B** is similar but lighter (2.0 GB) at the cost of some accuracy.
- **Gemma 3 4B** is a general-purpose model that works but can't match the grammar-specialized GRMR models.
- **Harper** is instant (0.2s) but only supports English and misses more complex errors.
- **OpenRouter** gives the best results but requires an API key and sends text to a remote server.

> Run benchmarks yourself: `cargo run --release --bin benchmark` from `src-tauri/`.

Models are downloaded on demand from Hugging Face and cached locally.

> [!TIP]
> You can add any GGUF model via the settings in app.


## Acknowledgements

- [Tauri](https://github.com/tauri-apps/tauri) — desktop app framework
- [Rust](https://www.rust-lang.org/) — systems programming language
- [Svelte](https://github.com/sveltejs/svelte) — reactive UI framework
- [llama.cpp](https://github.com/ggerganov/llama.cpp) — local LLM inference engine
- [Harper](https://github.com/Automattic/harper) — grammar checker
- [GRMR](https://huggingface.co/qingy2024) — grammar fine-tuned models
- [Gemma](https://ai.google.dev/gemma) — Google's open models

## Screenshots

<div align="center">
<img src="static/jollyhistory.png" width="32%" alt="Jolly History" />
<img src="static/jollyhome.png" width="32%" alt="Jolly Home" />
<img src="static/jollysettings.png" width="32%" alt="Jolly Settings" />
</div>

## License

[GPL-3.0](LICENSE) — free and open source. No account, no subscription.
