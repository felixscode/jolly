<p align="center">
  <img src="static/jolly_normal.svg" width="120" alt="Jolly" />
</p>

<h1 align="center">Jolly</h1>

<p align="center">
  <strong>Local-first spell checker powered by on-device LLMs.</strong><br />
  Copy text &rarr; hit Enter &rarr; corrected text lands back in your clipboard.
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Rust-2021-b7410e?logo=rust&logoColor=white" alt="Rust" />
  <img src="https://img.shields.io/badge/TypeScript-5.9-3178c6?logo=typescript&logoColor=white" alt="TypeScript" />
  <img src="https://img.shields.io/badge/Svelte-5-ff3e00?logo=svelte&logoColor=white" alt="Svelte" />
  <img src="https://img.shields.io/badge/Tailwind-4-06b6d4?logo=tailwindcss&logoColor=white" alt="Tailwind" />
  <img src="https://img.shields.io/badge/Tauri-2-24c8d8?logo=tauri&logoColor=white" alt="Tauri" />
</p>

<p align="center">
  <img src="https://img.shields.io/badge/CI%2FCD-planned-lightgrey" alt="CI/CD planned" />
  <img src="https://img.shields.io/badge/coverage-planned-lightgrey" alt="Coverage planned" />
  <img src="https://img.shields.io/github/license/felixscode/jolly" alt="License" />
</p>

---

## How it works

1. Copy misspelled text to your clipboard
2. Press **Enter** in Jolly
3. A local LLM corrects spelling & grammar
4. The fixed text is pasted back — nothing leaves your machine

Supports English and German. Falls back to the OpenRouter API if local inference isn't available.

## Benchmarks

Tested across 40 cases (short, medium, long, email) in English and German.
Inference on CPU — times will be significantly faster with CUDA or Metal.

```
Accuracy (exact match)
                                    EN       DE     Overall
Qwen 2.5 3B Instruct   ████████░░  86%  ███░░░░░░░  52%  ███████░░░  72%
Qwen 2.5 1.5B Instruct ████████░░  90%  █░░░░░░░░░  14%  █████░░░░░  58%
```

| Model | Params | Size | Accuracy | Similarity | Avg Latency |
|-------|--------|------|----------|------------|-------------|
| **Qwen 2.5 3B** | 3B | 2.0 GB | **72%** | **99%** | 7.2s |
| Qwen 2.5 1.5B | 1.5B | 1.0 GB | 58% | 79% | 4.6s |

<details>
<summary>Breakdown by category (Qwen 2.5 3B)</summary>

| Category | Accuracy |
|----------|----------|
| Short | 100% |
| Medium | 85% |
| Long | 43% |
| Email | 20% |

</details>

> **Note:** Phi 3.5 Mini, Gemma 2 2B, and Mistral 7B are available in-app but not yet benchmarked.
> Run benchmarks yourself: `cargo run --bin benchmark` from `src-tauri/`.

## Available models

| Model | Size | Quantization |
|-------|------|--------------|
| Qwen 2.5 1.5B Instruct | 1.0 GB | Q4_K_M |
| Qwen 2.5 3B Instruct | 2.0 GB | Q4_K_M |
| Phi 3.5 Mini Instruct | 2.6 GB | Q4_K_M |
| Gemma 2 2B IT | 1.8 GB | Q4_K_M |
| Mistral 7B Instruct v0.3 | 4.7 GB | Q4_K_M |

Models are downloaded on demand from Hugging Face and cached locally.

## Build

### Prerequisites

- [Node.js](https://nodejs.org/) >= 18
- [Rust](https://rustup.rs/) (2021 edition)
- [Tauri CLI](https://v2.tauri.app/start/prerequisites/) system dependencies

### Development

```sh
npm install
npm run tauri dev
```

### Production

```sh
npm run tauri build
```

### CUDA

For NVIDIA GPU acceleration, enable the `cuda` feature flag:

```sh
npm run tauri build -- -- --features cuda
```

Or during development:

```sh
npm run tauri dev -- -- --features cuda
```

**Requirements:**
- NVIDIA GPU with compute capability >= 6.0
- [CUDA Toolkit](https://developer.nvidia.com/cuda-toolkit) installed and on `PATH`
- Compatible driver version for your CUDA toolkit

Jolly automatically detects GPU availability at runtime. If CUDA initialization fails (driver mismatch, out of memory, etc.), it silently falls back to CPU inference — no config needed.

### Metal (macOS)

```sh
npm run tauri build -- -- --features metal
```

## Tech stack

| Layer | Tech |
|-------|------|
| Frontend | [SvelteKit](https://github.com/sveltejs/kit), Tailwind CSS v4, TypeScript |
| Backend | [Tauri 2](https://github.com/tauri-apps/tauri), Rust |
| Inference | [mistral.rs](https://github.com/EricLBuehler/mistral.rs), GGUF models |

## License

Free and open source. No account, no subscription.
