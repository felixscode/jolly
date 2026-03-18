<div align="center">


# Jolly

[![Deploy Website](https://github.com/felixscode/jolly/actions/workflows/deploy-web.yml/badge.svg)](https://github.com/felixscode/jolly/actions/workflows/deploy-web.yml)
[![Release Desktop App](https://github.com/felixscode/jolly/actions/workflows/release-app.yml/badge.svg)](https://github.com/felixscode/jolly/actions/workflows/release-app.yml)
[![License](https://img.shields.io/github/license/felixscode/jolly)](https://github.com/felixscode/jolly)
[![Release](https://img.shields.io/github/v/release/felixscode/jolly)](https://github.com/felixscode/jolly/releases)

**Local-first spell checker powered by on-device LLMs.**
Copy Enter Paste! Jolly read from your clipboard and applies changes so you can paste it back.
Notthing leafs your Device

<img src="static/jolly_normal.svg" width="120" alt="Jolly" />

[Features](#features) | [Installation](#installation) | [Benchmarks](#benchmarks) | [Models](#available-models) | [Development](#development) | [Tech Stack](#tech-stack)

</div>
## Features

- **Privacy-first**: All inference runs locally — nothing leaves your machine
- **One-step correction**: Copy, press Enter, paste — corrected text is in your clipboard
- **Multiple models**: Choose from on-device LLMs, downloaded on demand
- **API fallback**: Falls back to OpenRouter API if local inference isn't available or too slow

> **Tip:** If Jolly feels slow on your machine, consider switching to [Harper](https://github.com/Automattic/harper) for instant grammar checking or [OpenRouter](https://openrouter.ai/) for fast cloud-based inference.

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
- [Rust](https://rustup.rs/) (2021 edition)
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

> **Note:** Phi 3.5 Mini, Gemma 2 2B, and Mistral 7B are available in-app but not yet benchmarked.
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

## Development

```sh
npm install
npx tauri dev
```

### Commands

| Command | Description |
|---------|-------------|
| `npm run dev` | Start dev server |
| `npm run build` | Production build (web) |
| `npx tauri dev` | Desktop dev mode |
| `npx tauri build` | Desktop production build |
| `npm run check` | Type checking |
| `npm run lint` | Format check |
| `npm run format` | Auto-format |


## Acknowledgements

- [Tauri](https://github.com/tauri-apps/tauri) — desktop app framework
- [Rust](https://www.rust-lang.org/) — systems programming language
- [Svelte](https://github.com/sveltejs/svelte) — reactive UI framework
- [mistral.rs](https://github.com/EricLBuehler/mistral.rs) — local LLM inference engine
- [Harper](https://github.com/Automattic/harper) — grammar checker

## License

[GPL-3.0](LICENSE) — free and open source. No account, no subscription.
