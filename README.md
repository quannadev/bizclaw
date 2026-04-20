# 🦞 BizClaw - Self-Hosted AI Agent Platform

<p align="center">
  <strong>Your own AI assistant. Any channel. Any provider. 100% yours.</strong><br>
  <sub>Single-tenant • Rust-powered • Privacy-first</sub>
</p>

<p align="center">
  <a href="https://github.com/nguyenduchoai/bizclaw/actions"><img src="https://img.shields.io/github/actions/workflow/status/nguyenduchoai/bizclaw/ci?style=flat-square" alt="Build"></a>
  <a href="https://github.com/nguyenduchoai/bizclaw/releases"><img src="https://img.shields.io/github/v/release/nguyenduchoai/bizclaw?style=flat-square" alt="Version"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/License-MIT-blue.svg?style=flat-square" alt="License"></a>
  <a href="https://discord.gg/bizclaw"><img src="https://img.shields.io/badge/Discord-Join-7289DA?style=flat-square&logo=discord" alt="Discord"></a>
</p>

---

## 🎯 BizClaw là gì?

BizClaw là **nền tảng AI Agent tự-host** (self-hosted) cho phép bạn:

- 🤖 Chạy AI Agent của riêng bạn trên server/VPS
- 💬 Kết nối nhiều kênh: Zalo, Telegram, Discord, Slack...
- 🔒 Dữ liệu 100% thuộc về bạn - không có cloud
- ⚡ Fast, lightweight, viết bằng Rust

### So với Cloud AI Assistants

| | BizClaw (Self-hosted) | ChatGPT/Claude Cloud |
|--|----------------------|----------------------|
| **Dữ liệu** | 100% private | Server-side |
| **Chi phí** | Một lần server | Trả theo usage |
| **Customization** | Full control | Limited |
| **Privacy** | Không có data离开 server | Data được lưu trên cloud |
| **Setup** | Cần server | Ngay lập tức |

---

## 🚀 Bắt Đầu

### Yêu cầu
- Rust 1.85+
- macOS / Linux / Windows
- Server/VPS (hoặc chạy local)

### Cài đặt nhanh

```bash
# 1. Clone repo
git clone https://github.com/nguyenduchoai/bizclaw.git
cd bizclaw

# 2. Build
cargo build --release

# 3. Chạy
./target/release/bizclaw-desktop
```

### Docker

```bash
docker run -d \
  --name bizclaw \
  -p 3000:3000 \
  -p 8080:8080 \
  -v bizclaw-data:/data \
  nguyenduchoai/bizclaw:latest
```

---

## ✅ Features Thực Sự Hoạt Động

### Channels (Kết nối kênh)

| Kênh | Status | Ghi chú |
|------|--------|----------|
| **Zalo OA** | ✅ Có code | Official Account API |
| **Zalo Personal** | ✅ Có code | Personal Zalo |
| **Telegram** | ✅ Có code | Bot API |
| **Discord** | ✅ Có code | Bot Gateway |
| **Slack** | ✅ Có code | Socket Mode |
| **WhatsApp** | ✅ Có code | Business API |
| **Email** | ✅ Có code | IMAP/SMTP |
| **Webhook** | ✅ Có code | Custom HTTP |

### AI Providers (18+ Models)

| Provider | Status |
|----------|--------|
| OpenAI (GPT-4o, o1) | ✅ |
| Anthropic (Claude 3.5, 4) | ✅ |
| Google Gemini (2.0, 2.5) | ✅ |
| DeepSeek (V3, R1) | ✅ |
| Groq (Llama, Mixtral) | ✅ |
| Ollama (local models) | ✅ |
| Llama.cpp (GGUF local) | ✅ |
| Cohere, Perplexity, Mistral, xAI, MiniMax | ✅ |

### Tools (40+)

| Category | Tools |
|----------|-------|
| **Browser** | browser, screenshot, stealth mode |
| **Database** | SQL query, semantic search, schema |
| **Social** | Facebook, Instagram, TikTok posting |
| **File** | read, write, edit, glob, grep |
| **Shell** | Command execution |
| **Web** | search, fetch, HTTP requests |
| **Memory** | Vector store, semantic search |
| **AI** | Content generation, image generation |

### Security

| Feature | Status |
|---------|--------|
| AES-256 encryption | ✅ |
| API Key Vault | ✅ |
| Prompt injection detection | ✅ |
| SQL injection prevention | ✅ |
| Rate limiting | ✅ |
| Audit trail | ✅ |

---

## 🏗️ Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    BizClaw Architecture                   │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌─────────┐    ┌─────────┐    ┌─────────┐              │
│  │Channels │    │ Gateway │    │Dashboard│              │
│  │ Zalo/TG │───▶│ Actix   │◀───│  Web UI  │              │
│  │ FB/IG   │    │  Web    │    │ React   │              │
│  └────┬────┘    └────┬────┘    └─────────┘              │
│       │                │                               │
│       │         ┌──────┴──────┐                      │
│       │         │   Agent     │                      │
│       │         │ Orchestrator│                      │
│       │         └──────┬──────┘                      │
│       │                │                               │
│  ┌───┴───┐      ┌─────┴─────┐      ┌───────┐         │
│  │Memory │      │ Providers │      │ Tools │         │
│  │Vector │      │  18+ LLMs │      │ 40+  │         │
│  └───────┘      └───────────┘      └───────┘         │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

---

## 📖 Documentation

| Document | Description |
|----------|-------------|
| [Quick Start](docs/QUICK_START.md) | Bắt đầu nhanh |
| [Configuration](docs/CONFIG.md) | Cấu hình chi tiết |
| [Channels](docs/CHANNELS.md) | Kết nối kênh |
| [AI Providers](docs/PROVIDERS.md) | Cấu hình AI providers |
| [Tools](docs/TOOLS.md) | Danh sách tools |
| [Deployment](docs/DEPLOYMENT.md) | Deploy lên VPS |
| [API Reference](docs/API.md) | REST API |

---

## � Use Cases

### 1. Customer Support Bot
```yaml
# Kết nối Zalo + AI Agent
channels:
  zalo:
    enabled: true
    app_id: your_app_id
    
agent:
  system_prompt: "Bạn là nhân viên chăm sóc khách hàng..."
```

### 2. Social Media Manager
```yaml
# Auto post lên nhiều kênh
tools:
  social_post:
    channels: [facebook, instagram, telegram]
    schedule: "0 9 * * *"  # 9h daily
```

### 3. Internal Assistant
```yaml
# Chat với team qua Slack/Discord
channels:
  slack:
    enabled: true
    bot_token: xoxb-...
```

---

## ⚙️ Configuration

```toml
# config.toml
[app]
name = "BizClaw"
port = 8080

[ai]
# Provider đầu tiên available sẽ được dùng
providers = ["openai", "anthropic", "gemini"]

[channels.telegram]
enabled = true
bot_token = "YOUR_BOT_TOKEN"

[channels.zalo]
enabled = true
app_id = "YOUR_APP_ID"
app_secret = "YOUR_SECRET"

[memory]
vector_dimensions = 1536
```

---

## 🐛 Troubleshooting

### Zalo không kết nối?
- Kiểm tra app_id và app_secret đúng
- Verify OAuth callback URL
- Xem logs: `tail -f logs/bizclaw.log`

### AI không trả lời?
- Kiểm tra API key: `curl http://localhost:8080/api/providers`
- Thử provider khác trong config

### Server chạy chậm?
- Restart: `docker restart bizclaw`
- Check memory: `docker stats`
- Kiểm tra logs: `docker logs bizclaw`

---

## 🤝 Contributing

Contributions are welcome! Xem [CONTRIBUTING.md](CONTRIBUTING.md).

```bash
# Fork repo
# Tạo branch mới
git checkout -b feature/my-feature

# Commit
git commit -m "feat: add amazing feature"

# Push và tạo PR
git push origin feature/my-feature
```

---

## 📞 Liên hệ

| Kênh | Link |
|------|------|
| 🌐 Website | [bizclaw.vn](https://bizclaw.vn) |
| 💬 Discord | [Discord Community](https://discord.gg/bizclaw) |
| 🐦 Twitter | [@bizclawvn](https://twitter.com/bizclawvn) |
| 📧 Email | contact@bizclaw.vn |

---

## 📄 License

MIT License - Xem [LICENSE](LICENSE).

---

<p align="center">
  <sub>Made with ❤️ in Vietnam</sub><br>
  <sub>BizClaw - Self-hosted AI Agent Platform</sub>
</p>
