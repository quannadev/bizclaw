# 🦞 BizClaw

**Self-hosted AI Agent Platform** — Chạy AI Agent của riêng bạn, kết nối mọi kênh, 100% private.

<p align="center">
  <a href="https://github.com/nguyenduchoai/bizclaw/actions"><img src="https://img.shields.io/github/actions/workflow/status/nguyenduchoai/bizclaw/ci?style=flat-square" alt="Build"></a>
  <a href="https://github.com/nguyenduchoai/bizclaw/releases"><img src="https://img.shields.io/github/v/release/nguyenduchoai/bizclaw?style=flat-square" alt="Version"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/License-MIT-blue.svg?style=flat-square" alt="License"></a>
</p>

---

## ✨ Features

- **🤖 18+ AI Providers**: OpenAI, Anthropic, Gemini, DeepSeek, Groq, Ollama, MiniMax...
- **💬 8 Channels**: Zalo, Telegram, Discord, Slack, WhatsApp, Email, Webhook
- **🔧 40+ Tools**: Browser, Database, Social media, File, Shell, Memory
- **🛡️ Security**: AES-256, Prompt injection detection, API Key Vault
- **⚡ Rust-powered**: Fast, lightweight, self-hosted

---

## 🚀 Quick Start

```bash
# Clone & build
git clone https://github.com/nguyenduchoai/bizclaw.git
cd bizclaw
cargo build --release

# Chạy
./target/release/bizclaw-desktop
```

### Docker
```bash
docker run -d --name bizclaw -p 3000:3000 -p 8080:8080 -v bizclaw-data:/data \
  nguyenduchoai/bizclaw:latest
```

---

## 📋 Channels & Providers

| Channels | Providers |
|----------|-----------|
| Zalo, Telegram, Discord, Slack | OpenAI (GPT-4o, o1) |
| WhatsApp, Email, Webhook | Anthropic (Claude 3.5, 4) |
| Facebook, Instagram, TikTok | Google Gemini, DeepSeek, Groq |

---

## ⚙️ Configuration

```toml
[ai]
providers = ["openai", "anthropic", "gemini"]

[channels.telegram]
enabled = true
bot_token = "YOUR_BOT_TOKEN"

[channels.zalo]
enabled = true
app_id = "YOUR_APP_ID"
```

---

## 📁 Project Structure

```
bizclaw/
├── crates/
│   ├── bizclaw-agent/      # AI Agent core
│   ├── bizclaw-gateway/    # HTTP API
│   ├── bizclaw-channels/   # Channel integrations
│   ├── bizclaw-providers/  # AI provider adapters
│   ├── bizclaw-tools/      # Tool registry
│   ├── bizclaw-memory/     # Vector memory
│   ├── bizclaw-security/   # Security layer
│   ├── bizclaw-evaluator/  # LLM-as-Judge
│   ├── bizclaw-redteam/    # Security testing
│   └── bizclaw-hai/        # Conversation flows
└── docs/                   # Documentation
```

---

## 🤝 Contributing

```bash
git checkout -b feature/my-feature
git commit -m "feat: add feature"
git push origin feature/my-feature
```

---

## 📄 License

MIT License

---

<p align="center">
  Made with ❤️ in Vietnam
</p>
