# ⚡ BizClaw — AI Platform for SME Growth

<p align="center">
  <img src="docs/images/hero-banner.png" alt="BizClaw — AI Agent Platform" width="800">
</p>

<p align="center">
  <strong>AI Agent Platform — Two Deployment Options</strong><br>
  <a href="#-bizclaw-cloud-multi-tenant">BizClaw Cloud</a> (SaaS) • <a href="#-bizclaw-single-tenant">BizClaw</a> (Self-Hosted)
</p>

> **BizClaw** là nền tảng AI Agent được thiết kế cho SME Việt Nam. Kết nối đa kênh, tạo nội dung thông minh, và tự động hóa quy trình — tất cả trong một nền tảng duy nhất.

[![Rust](https://img.shields.io/badge/Rust-100%25-orange?logo=rust)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Tests](https://img.shields.io/badge/tests-743%20passing-brightgreen)]()
[![Version](https://img.shields.io/badge/version-v1.1.7-purple)]()
[![Website](https://img.shields.io/badge/🌐_Website-bizclaw.vn-blue)](https://bizclaw.vn)
[![Facebook](https://img.shields.io/badge/📘_Fanpage-bizclaw.vn-1877F2?logo=facebook)](https://www.facebook.com/bizclaw.vn)

---

## 🎯 BizClaw dành cho ai?

| Đối tượng | Lợi ích |
|-----------|---------|
| 🏪 **SME / Doanh nghiệp nhỏ** | "Nhân viên AI" trả lời 24/7, tư vấn tự động, tiết kiệm nhân sự |
| 🛒 **E-Commerce** | Quản lý đơn hàng, tạo content đa kênh (Shopee, TikTok, Zalo) |
| 📱 **Marketing** | Lên lịch đăng bài tự động, phân tích hiệu quả chiến dịch |
| 📞 **Sales / Support** | Chatbot thông minh, phản hồi nhanh chóng |

---

## 🏢 BizClaw Cloud (Multi-Tenant)

> **Giải pháp SaaS** — Không cần server, bắt đầu trong 5 phút.

### Tính năng chính

| Hạng mục | Chi tiết |
|-----------|----------|
| **🔌 18+ AI Providers** | OpenAI, Anthropic, Gemini, DeepSeek, Groq, MiniMax, xAI (Grok), Mistral, BytePlus ModelArk |
| **💬 9 Channels** | Telegram, Discord, Slack, Email, Webhook, WhatsApp, Zalo (Personal + OA), Web |
| **🛠️ 35+ Tools** | Browser automation, Social posting, Database, Voice transcription, Shell, File operations |
| **🔗 MCP Ecosystem** | Kết nối 1000+ tools từ MCP Hub |
| **📚 Knowledge RAG** | Hybrid search (FTS5 + Vector), multi-model embedding |
| **🤖 Multi-Agent** | Tạo đội ngũ agent với vai trò khác nhau |
| **🔄 Workflows** | 23 workflow templates — Sequential, FanOut, Conditional, Loop |
| **📊 Analytics** | Dashboard theo dõi hiệu suất, Prometheus metrics |
| **🔐 Bảo mật** | RBAC 4-tier, AES-256 encryption, Audit trail |

### Cài đặt nhanh

```bash
# Truy cập cloud platform
# → https://bizclaw.vn

# Hoặc CLI
npm install -g @bizclaw/cli
bizclaw login
bizclaw init
```

---

## 🏠 BizClaw (Single-Tenant)

> **Giải pháp Self-Hosted** — Dữ liệu 100% thuộc về bạn, chạy trên VPS/Local.

### Tính năng chính

| Hạng mục | Chi tiết |
|-----------|----------|
| **🔌 18+ AI Providers** | OpenAI, Anthropic, Gemini, DeepSeek, Groq, OpenRouter, Together, MiniMax, xAI (Grok), Mistral, BytePlus ModelArk, Cohere, Perplexity, DashScope |
| **💬 9 Channels** | CLI, Telegram, Discord, Slack, Email (IMAP/SMTP), Webhook, WhatsApp, Zalo |
| **🛠️ 35+ Tools** | Browser (Stealth), Social Post, DB Semantic, Voice Transcribe, Shell, File, HTTP, Plan, Zalo Tool |
| **🔗 MCP** | Model Context Protocol — kết nối MCP servers bên ngoài |
| **🖐️ Autonomous Hands** | Agent chạy background 24/7 — Research, Analytics, Content, Monitoring, Security |
| **📚 Knowledge RAG** | Hybrid search, Nudges, MCP server, Folder Watcher |
| **🔄 Workflows** | 23 workflow templates có sẵn |
| **🖥️ Web Dashboard** | 20+ trang UI (VI/EN), dark/light mode |
| **🔐 Vault** | Mã hoá API keys với AES-256-CBC |

### Cài đặt nhanh

```bash
# Cách 1: Desktop App (macOS / Windows / Linux)
git clone https://github.com/nguyenduchoai/bizclaw-cloud.git
cd bizclaw && cargo build --release
./target/release/bizclaw-desktop

# Cách 2: Docker
docker-compose -f docker-compose.standalone.yml up -d

# Cách 3: Remote Access
./target/release/bizclaw serve --tunnel
```

| Platform | Binary | Size |
|----------|--------|------|
| 🍎 **macOS** | `bizclaw-desktop` | ~13MB |
| 🪟 **Windows** | `bizclaw-desktop.exe` | ~12MB |
| 🐧 **Linux** | `bizclaw-desktop` | ~12MB |

---

## ✨ So sánh hai giải pháp

| Tính năng | BizClaw Cloud | BizClaw (Self-Hosted) |
|------------|--------------|------------------------|
| Deployment | SaaS (hosted) | VPS / Local |
| Dữ liệu | Encrypted in cloud | 100% on-premise |
| Setup | < 5 minutes | < 10 minutes |
| Maintenance | Managed by BizClaw | Self-managed |
| API Keys | Shared vault | Private vault |
| Channels | Full | Full |
| Workflows | Full | Full |
| Pricing | Subscription | One-time (self-hosted) |
| Support | 24/7 | Community + Enterprise |

---

## 🛠️ Tích hợp đa kênh

BizClaw kết nối tất cả các kênh quan trọng cho SME Việt:

```
  ┌─────────────────────────────────────────────────┐
  │                 BizClaw Platform                 │
  │                                                   │
  │  💬 Zalo OA    →  Tin nhắn, Quảng cáo, OA      │
  │  📱 TikTok    →  Video, Shop, Content         │
  │  🛒 Shopee    →  Quản lý đơn hàng           │
  │  💬 Telegram   →  Hỗ trợ khách hàng          │
  │  📧 Email     →  Email marketing              │
  │  🌐 Web       →  Chat widget                 │
  └─────────────────────────────────────────────────┘
```

---

## 🤖 AI Agents — Tự động hóa mọi quy trình

### Autonomous Hands (Background Agents)

Agent chạy 24/7, tự retry, tự báo cáo:

| Hand | Nhiệm vụ |
|------|-----------|
| 🔍 **Research Hand** | Thu thập thông tin, phân tích xu hướng |
| 📊 **Analytics Hand** | Thống kê, xử lý trends |
| ✍️ **Content Hand** | Sáng tạo nội dung, self-review |
| 🛡️ **Monitor Hand** | Giám sát hệ thống, cảnh báo |
| 🔄 **Sync Hand** | Đồng bộ dữ liệu đa kênh |
| 📣 **Outreach Hand** | Soạn tin, gửi đa kênh |

### Multi-Agent System

Tạo đội ngũ agent với vai trò khác nhau:

```
  ┌─────────────────── Orchestrator ───────────────────┐
  │                                                     │
  │  🧑‍💼 Agent "Research"  │ Gemini/flash    │ Web     │
  │  📊 Agent "Analyst"   │ DeepSeek/chat   │ Reports │
  │  ✍️ Agent "Writer"    │ Claude          │ Content │
  │  📞 Agent "Support"    │ GPT-4o          │ Chat    │
  │                                                     │
  └─────────────────────────────────────────────────────┘
```

---

## 🔄 Workflows — Tự động hóa quy trình

23 workflow templates có sẵn:

| Template | Use Case |
|---------|----------|
| **Content Pipeline** | Viết → Review → Publish |
| **Expert Consensus** | Thu thập ý kiến nhiều chuyên gia |
| **Research Pipeline** | Gather → Analyze → Report |
| **Code Review** | Submit → Review → Approve |
| **AI Slide Creator** | Topic → Slides → Present |

---

## 🔌 Mở rộng với MCP

> Model Context Protocol — kết nối tools không giới hạn.

```toml
[[mcp_servers]]
name = "github"
command = "npx"
args = ["-y", "@modelcontextprotocol/server-github"]

[[mcp_servers]]
name = "filesystem"
command = "npx"
args = ["-y", "@modelcontextprotocol/server-filesystem", "/data"]
```

**1000+ MCP tools** có sẵn tại [MCP Hub](https://github.com/modelcontextprotocol/servers)

---

## 🔐 Bảo mật & Compliance

| Tính năng | Mô tả |
|-----------|--------|
| **AES-256 Encryption** | Mã hoá API keys at rest |
| **RBAC 4-tier** | Admin → Manager → User → Viewer |
| **Prompt Injection Scanner** | 8 patterns, 80+ keywords (EN/VI/CN) |
| **SSRF Protection** | IPv4 + IPv6 validation |
| **Audit Trail** | Log mọi action |
| **Rate Limiting** | Per-IP protection |
| **Command Allowlist** | Sandboxed execution |

---

## 📊 Monitoring & Analytics

```bash
# Prometheus metrics endpoint
curl http://localhost:3000/metrics

# Output: OpenMetrics format
bizclaw_llm_requests_total{provider="openai"} 1234
bizclaw_channel_messages_total{channel="telegram"} 5678
bizclaw_agent_active_tasks 42
```

Tích hợp Grafana dashboards có sẵn.

---

## 🏗️ Architecture

```
┌─────────────────────────────────────────────────────┐
│                    BizClaw Platform                  │
├─────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌───────────┐  │
│  │   Gateway   │  │  Scheduler  │  │   Hands   │  │
│  │   (Axum)    │  │   (Cron)    │  │ (Background)│
│  └─────────────┘  └─────────────┘  └───────────┘  │
├─────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌───────────┐  │
│  │  Channels  │  │   Agents    │  │ Workflows │  │
│  │ (9 kênh)   │  │  (Multi)    │  │  (23+)    │  │
│  └─────────────┘  └─────────────┘  └───────────┘  │
├─────────────────────────────────────────────────────┤
│  ┌───────────────────────────────────────────────┐  │
│  │            AI Providers (18+)                 │  │
│  │  OpenAI • Anthropic • Gemini • DeepSeek     │  │
│  │  Groq • MiniMax • xAI • Mistral • ...      │  │
│  └───────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────┘
```

---

## 📖 Tài liệu

| Tài liệu | Mô tả |
|----------|--------|
| [📚 Docs](docs/) | Tài liệu chi tiết |
| [🏪 SME Quickstart](docs/sme-quickstart.md) | Hướng dẫn SME trong 5 phút |
| [🛠️ Architecture](docs/ARCHITECTURE.md) | Kiến trúc hệ thống |
| [🔧 API Reference](docs/api/) | API endpoints |
| [🤖 Agent Templates](gallery/) | 51 agent templates |

---

## 🧪 SME Lean Model — Giảm 75% nhân sự vận hành

> BizClaw được thiết kế theo mô hình **SME Lean** — tối ưu AI-first, giảm nhân sự thủ công.

| Trước | Sau với BizClaw |
|-------|----------------|
| 8-12 nhân viên | 2-3 nhân viên |
| 10 posts/tuần | 50+ posts/tuần |
| Phản hồi 2-4 giờ | Phản hồi 5-15 phút |
| Content handmade | Content AI-generated |
| Manual scheduling | Auto-scheduling |

---

## 📝 License

MIT License — Sử dụng tự do cho mục đích thương mại và phi thương mại.

---

## 🤝 Contributing

Pull requests are welcome! Vui lòng đọc [CONTRIBUTING.md](CONTRIBUTING.md) trước khi đóng góp.

---

<p align="center">
  <strong>Built with ❤️ for SME Việt Nam</strong><br>
  <a href="https://bizclaw.vn">bizclaw.vn</a> • <a href="https://facebook.com/bizclaw.vn">Facebook</a> • <a href="mailto:support@bizclaw.vn">Support</a>
</p>
