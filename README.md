<div align="center">


# Jolly

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

<img src="static/jolly_normal.svg" width="120" alt="Jolly" />

[Features](#features) | [Installation](#installation) | [Benchmarks](#benchmarks) | [Models](#available-models) | [Development](#development) | [Tech Stack](#tech-stack)

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

#### GPU Acceleration (optional)

**CUDA (NVIDIA):**
```sh
npx tauri build -- --features cuda
```
Requires CUDA Toolkit on `PATH` and GPU with compute capability >= 6.0.

**Metal (macOS):**
```sh
npx tauri build -- --features metal
```

Jolly detects GPU availability at runtime. If initialization fails, it silently falls back to CPU.

## Benchmarks

Tested across 8 cases (short, medium, email) in English and German.
Inference on CPU — times will be significantly faster with CUDA or Metal.

| Model | Params | Size | Accuracy | Similarity | Avg Latency |
|-------|--------|------|----------|------------|-------------|
| **Qwen 2.5 3B** | 3B | 2.0 GB | **72%** | **99%** | 7.2s |
| Qwen 2.5 1.5B | 1.5B | 1.0 GB | 58% | 79% | 4.6s |


> Run benchmarks yourself: `cargo run --bin benchmark` from `src-tauri/`.

## Available Models

| Model | Size | Quantization |
|-------|------|--------------|
| Qwen 2.5 1.5B Instruct | 1.0 GB | Q4_K_M |
| Qwen 2.5 3B Instruct | 2.0 GB | Q4_K_M |
| Phi 3.5 Mini Instruct | 2.6 GB | Q4_K_M |
| Gemma 2 2B IT | 1.8 GB | Q4_K_M |
| Mistral 7B Instruct v0.3 | 4.7 GB | Q4_K_M |

Models are downloaded on demand from Hugging Face and cached locally.

[!TIP] you can add any gguf model via the settings in app.  



## Acknowledgements

- [Tauri](https://github.com/tauri-apps/tauri) — desktop app framework
- [Rust](https://www.rust-lang.org/) — systems programming language
- [Svelte](https://github.com/sveltejs/svelte) — reactive UI framework
- [llama.cpp](https://github.com/ggerganov/llama.cpp) — local LLM inference engine
- [Harper](https://github.com/Automattic/harper) — grammar checker

## Screenshots

<div align="center">
<img src="static/jollyhistory.png" width="32%" alt="Jolly History" />
<img src="static/jollyhome.png" width="32%" alt="Jolly Home" />
<img src="static/jollysettings.png" width="32%" alt="Jolly Settings" />
</div>

## License

[GPL-3.0](LICENSE) — free and open source. No account, no subscription.
