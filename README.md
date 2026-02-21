# ⚡ BizClaw

> **Fast, modular AI assistant infrastructure — written in pure Rust.**

BizClaw is a trait-driven AI agent platform designed to run **anywhere** — from Raspberry Pi to cloud servers. It supports multiple LLM providers, communication channels, and tools through a unified, swappable architecture.

[![Rust](https://img.shields.io/badge/Rust-100%25-orange?logo=rust)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

---

## 🎯 Features

- **🧠 Local Brain Engine** — Run LLaMA-family models locally via GGUF format with mmap, quantization (Q4_0/Q8_0), and SIMD acceleration
- **🔌 Multi-Provider** — OpenAI, Anthropic Claude, Ollama, llama.cpp, OpenRouter, or any OpenAI-compatible server
- **💬 Multi-Channel** — CLI, Zalo (Personal + OA), Telegram, Discord, WhatsApp, Webhooks
- **🛠️ Tool Calling** — Shell execution, file operations, with extensible tool registry
- **🔒 Security** — Command allowlists, path restrictions, sandboxed execution, encrypted secrets
- **💾 Memory** — SQLite persistence, in-memory vector search (cosine similarity), no-op mode
- **🌐 HTTP Gateway** — Axum-based REST API with CORS and tracing middleware
- **📦 Modular** — 10 independent crates, swap any component via traits

---

## 🏗️ Architecture

```
┌──────────────────────────────────────────────────────┐
│                    bizclaw (CLI)                      │
│              ┌─────────────────────┐                  │
│              │   bizclaw-agent     │                  │
│              │  (orchestration)    │                  │
│              └──────┬──────────────┘                  │
│     ┌───────────────┼───────────────┐                 │
│     ▼               ▼               ▼                 │
│ ┌─────────┐  ┌──────────┐  ┌──────────────┐         │
│ │Providers│  │ Channels │  │   Tools      │         │
│ │─────────│  │──────────│  │──────────────│         │
│ │ OpenAI  │  │   CLI    │  │   Shell      │         │
│ │Anthropic│  │   Zalo   │  │   File       │         │
│ │ Ollama  │  │ Telegram │  │  (custom)    │         │
│ │LlamaCpp │  │ Discord  │  └──────────────┘         │
│ │  Brain  │  │ Webhook  │                            │
│ │ Custom  │  └──────────┘                            │
│ └─────────┘                                          │
│     ┌───────────────┬───────────────┐                │
│     ▼               ▼               ▼                │
│ ┌─────────┐  ┌──────────┐  ┌──────────────┐        │
│ │ Memory  │  │ Security │  │   Gateway    │        │
│ │─────────│  │──────────│  │──────────────│        │
│ │ SQLite  │  │Allowlist │  │  Axum HTTP   │        │
│ │ Vector  │  │ Sandbox  │  │  REST API    │        │
│ │  NoOp   │  │ Secrets  │  └──────────────┘        │
│ └─────────┘  └──────────┘                           │
│                    ▼                                  │
│           ┌──────────────┐                           │
│           │ bizclaw-brain│                           │
│           │──────────────│                           │
│           │ GGUF Parser  │                           │
│           │ BPE Tokenizer│                           │
│           │ Attention    │                           │
│           │ KV Cache     │                           │
│           │ Quantization │                           │
│           │ SIMD/Rayon   │                           │
│           └──────────────┘                           │
└──────────────────────────────────────────────────────┘
```

---

## 📦 Crate Map

| Crate | Description | Status |
|-------|-------------|--------|
| `bizclaw-core` | Traits, types, config, errors | ✅ Complete |
| `bizclaw-brain` | Local GGUF inference engine | ✅ Foundation |
| `bizclaw-providers` | OpenAI, Anthropic, Ollama, LlamaCpp, Custom | ✅ Complete |
| `bizclaw-channels` | CLI, Zalo, Telegram, Discord | 🟡 CLI done |
| `bizclaw-memory` | SQLite, Vector, NoOp backends | ✅ Complete |
| `bizclaw-tools` | Shell, File tools + registry | ✅ Complete |
| `bizclaw-security` | Allowlist, Sandbox, Secrets | ✅ Complete |
| `bizclaw-agent` | Agent loop, context, tool execution | ✅ Complete |
| `bizclaw-gateway` | Axum HTTP REST API | ✅ Complete |
| `bizclaw-runtime` | Native process adapter | ✅ Complete |

---

## 🚀 Quick Start

### Prerequisites

- **Rust** 1.85+ (edition 2024)
- **Git**

### Build

```bash
git clone https://github.com/nguyenduchoai/bizclaw.git
cd bizclaw
cargo build --release
```

### Run (CLI mode with OpenAI)

```bash
export OPENAI_API_KEY="sk-..."
./target/release/bizclaw chat
```

### Run (with Ollama local model)

```bash
# Start Ollama first
ollama serve &
ollama pull llama3.2

# Run BizClaw with Ollama
./target/release/bizclaw chat --provider ollama --model llama3.2
```

### Run (with Anthropic Claude)

```bash
export ANTHROPIC_API_KEY="sk-ant-..."
./target/release/bizclaw chat --provider anthropic --model claude-sonnet-4-20250514
```

---

## ⚙️ Configuration

BizClaw uses TOML configuration at `~/.bizclaw/config.toml`:

```toml
# Default provider
default_provider = "openai"
default_model = "gpt-4o-mini"
default_temperature = 0.7

# Identity
[identity]
name = "BizClaw"
persona = "A helpful AI assistant"
system_prompt = "You are BizClaw, a fast and capable AI assistant."

# Memory
[memory]
backend = "sqlite"  # "sqlite" | "none"
auto_save = true

# Gateway
[gateway]
enabled = false
host = "127.0.0.1"
port = 3000

# Security
[autonomy]
level = "supervised"  # "full" | "supervised" | "locked"
allowed_commands = ["ls", "cat", "echo", "pwd", "find", "grep"]
forbidden_paths = ["/etc", "/var", "~/.ssh"]
```

---

## 🧠 Brain Engine (Local Inference)

BizClaw includes a **pure Rust** local inference engine for running GGUF models:

| Component | Description |
|-----------|-------------|
| **GGUF v3 Parser** | Full metadata + tensor index parsing |
| **mmap Loader** | Zero-copy model loading (critical for Pi 512MB) |
| **BPE Tokenizer** | Byte-level encoding with iterative merges |
| **Tensor Ops** | RMSNorm, MatMul, Softmax, SiLU |
| **Quantization** | Q4_0, Q8_0 dequantization kernels |
| **Attention** | Scaled dot-product with multi-head support |
| **KV Cache** | Per-layer key-value cache for generation |
| **RoPE** | Rotary Position Embeddings |
| **Sampler** | Temperature, Top-K, Top-P, repeat penalty |
| **Thread Pool** | Rayon-based parallel matmul |

---

## 📡 Gateway API

When enabled, the HTTP gateway exposes:

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/health` | GET | Health check |
| `/api/v1/info` | GET | System info + uptime |
| `/api/v1/config` | GET | Sanitized config |
| `/api/v1/providers` | GET | Available providers |
| `/api/v1/channels` | GET | Available channels |

---

## 🔒 Security Model

| Feature | Description |
|---------|-------------|
| **Command Allowlist** | Only whitelisted commands can be executed |
| **Path Restrictions** | Forbidden paths (e.g., `~/.ssh`) are rejected |
| **Workspace Only** | Optionally restrict to current working directory |
| **Sandbox** | Timeout, output truncation, restricted env |
| **Secret Store** | JSON with Unix 0600 permissions |

---

## 🧪 Testing

```bash
# Run all tests
cargo test

# Run brain engine tests
cargo test -p bizclaw-brain

# Run memory tests
cargo test -p bizclaw-memory
```

**Current test coverage: 11 tests passing** (tensor math, vector search, RoPE, parallel matmul)

---

## 🗺️ Roadmap

- [x] **Phase 1** — Core infrastructure (traits, config, error handling)
- [x] **Phase 1** — All providers (OpenAI, Anthropic, Ollama, LlamaCpp, Custom)
- [x] **Phase 1** — CLI channel, memory backends, security, gateway
- [x] **Phase 2** — Brain engine foundation (GGUF, tokenizer, tensor, quant, attention)
- [ ] **Phase 2** — Brain forward pass (wire weights to inference)
- [ ] **Phase 3** — Zalo channel (WebSocket login + messaging)
- [ ] **Phase 4** — SIMD acceleration (NEON for ARM, AVX2 for x86)
- [ ] **Phase 5** — Gateway WebSocket, streaming responses
- [ ] **Phase 6** — Telegram, Discord channels

---

## 📁 Project Structure

```
bizclaw/
├── Cargo.toml                 # Workspace root
├── src/main.rs                # CLI binary
├── crates/
│   ├── bizclaw-core/          # Traits, types, config, errors
│   ├── bizclaw-brain/         # Local GGUF inference engine
│   │   ├── gguf.rs            # GGUF v3 parser
│   │   ├── mmap.rs            # Memory-mapped loader
│   │   ├── tokenizer.rs       # BPE tokenizer
│   │   ├── tensor.rs          # Math ops (RMSNorm, MatMul, etc.)
│   │   ├── quant.rs           # Quantization kernels
│   │   ├── attention.rs       # Scaled dot-product attention
│   │   ├── kv_cache.rs        # Key-value cache
│   │   ├── rope.rs            # Rotary position embeddings
│   │   ├── sampler.rs         # Token sampling
│   │   └── model.rs           # LLaMA model params
│   ├── bizclaw-providers/     # LLM provider impls
│   │   ├── openai.rs          # OpenAI / OpenRouter
│   │   ├── anthropic.rs       # Anthropic Claude
│   │   ├── ollama.rs          # Ollama (local/remote)
│   │   ├── llamacpp.rs        # llama.cpp server
│   │   └── custom.rs          # Any OpenAI-compatible
│   ├── bizclaw-channels/      # Communication channels
│   │   ├── cli.rs             # Interactive terminal
│   │   └── zalo/              # Zalo Personal + OA
│   ├── bizclaw-memory/        # Persistence backends
│   │   ├── sqlite.rs          # SQLite storage
│   │   ├── vector.rs          # In-memory vector search
│   │   └── noop.rs            # No-op (disabled)
│   ├── bizclaw-tools/         # Tool execution
│   ├── bizclaw-security/      # Security policies
│   ├── bizclaw-agent/         # Agent orchestration
│   ├── bizclaw-gateway/       # HTTP REST API
│   └── bizclaw-runtime/       # Process adapters
└── plans/                     # Project plans & specs
```

---

## 📊 Stats

- **Language:** 100% Rust
- **Crates:** 11 (10 library + 1 binary)
- **Lines of Code:** ~5,200
- **Tests:** 11 passing
- **Dependencies:** tokio, axum, reqwest, serde, rusqlite, rayon, memmap2, half

---

## 📄 License

MIT License — see [LICENSE](LICENSE) for details.

---

**BizClaw** — *Fast AI, everywhere.*
