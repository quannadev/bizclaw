# ⚡ BizClaw — Trợ lý AI Cho Doanh Nghiệp Nhỏ

<p align="center">
  <img src="docs/images/hero-banner.png" alt="BizClaw — AI Agent Platform cho SME" width="800">
</p>

<p align="center">
  <strong>"Nhân viên AI" làm việc 24/7 cho doanh nghiệp của bạn</strong><br>
  Không cần biết lập trình • Giao diện tiếng Việt • Dữ liệu 100% thuộc về bạn
</p>

[![Rust](https://img.shields.io/badge/Rust-100%25-orange?logo=rust)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Tests](https://img.shields.io/badge/tests-743%20passing-brightgreen)]()
[![Version](https://img.shields.io/badge/version-v1.1.7-purple)]()
[![Website](https://img.shields.io/badge/🌐_Website-bizclaw.vn-blue)](https://bizclaw.vn)
[![Facebook](https://img.shields.io/badge/📘_Fanpage-bizclaw.vn-1877F2?logo=facebook)](https://www.facebook.com/bizclaw.vn)

---

## 🎯 BizClaw Dành Cho Ai?

| Đối tượng | Lợi ích |
|-----------|---------|
| 🏪 **Chủ shop online** | Trả lời tin nhắn tự động, tư vấn sản phẩm 24/7 |
| 🍜 **Chủ nhà hàng/quán cafe** | Đặt bàn tự động, gửi menu hàng ngày |
| ✈️ **Chủ khách sạn/nhà nghỉ** | Xác nhận booking, chăm sóc khách |
| 💼 **Dịch vụ B2B** | Báo giá tự động, lịch hẹn tự động |
| 📱 **Marketer** | Đăng bài đa nền tảng, nuôi lead tự động |

---

## 🚀 Bắt Đầu Trong 5 Phút

### 1️⃣ Cài đặt

```bash
# Từ source
git clone https://github.com/nguyenduchoai/bizclaw.git
cd bizclaw && cargo build --release
./target/release/bizclaw-desktop

# Docker
docker-compose -f docker-compose.standalone.yml up -d

# Remote access
./target/release/bizclaw serve --tunnel
```

| Nền tảng | Binary | Size |
|----------|--------|------|
| 🍎 **macOS** | `bizclaw-desktop` | ~13MB |
| 🪟 **Windows** | `bizclaw-desktop.exe` | ~12MB |
| 🐧 **Linux** | `bizclaw-desktop` | ~12MB |

### 2️⃣ Kết nối kênh

```
Settings → Kênh → Zalo OA → Quét QR
Settings → Kênh → Telegram → Nhập Bot Token
```

### 3️⃣ Tạo AI Agent đầu tiên

```
🤖 My Team → + Thêm Agent → Chọn vai trò → Đặt tên → Tạo
```

Nhắn tin cho Zalo OA / Telegram Bot → AI tự động trả lời!

---

## 📚 Tính Năng Hệ Thống

| Hạng mục | Chi tiết |
|----------|----------|
| **🔌 18 AI Providers** | OpenAI, Anthropic, Gemini, DeepSeek, Groq, OpenRouter, MiniMax, xAI (Grok), Mistral, BytePlus ModelArk, Cohere, Perplexity, DashScope, Together, Ollama, llama.cpp, vLLM, và bất kỳ API tương thích OpenAI |
| **💬 9+ Channels** | Zalo (Personal + OA + Bot), Telegram, Discord, Slack, Email (IMAP/SMTP), WhatsApp, Webhook, Web Chat |
| **🛠️ 40+ Tools** | Browser automation (stealth), Social posting, Database semantic, Voice transcription, Shell exec, File operations, HTTP client, Gmail, Calendar, Facebook tools, CRM, Computer Use |
| **🔗 MCP Ecosystem** | Model Context Protocol — kết nối 1000+ tools từ [MCP Hub](https://github.com/modelcontextprotocol/servers) |
| **📚 Knowledge RAG** | Hybrid search (FTS5 + Vector), multi-model embedding, knowledge graph, folder watcher, nudge system |
| **🤖 Multi-Agent** | Tạo đội ngũ agent — Sequential, Fan-Out, Conditional, Loop, A2A protocol |
| **🖐️ Autonomous Hands** | Agent background 24/7 — Research, Analytics, Content, Monitor, Sync, Outreach |
| **🔄 10+ Workflows** | Content Pipeline, Expert Consensus, Research Pipeline, Code Review, AI Slides, và nhiều hơn |
| **🖥️ 36 Dashboard Pages** | Web UI (Vietnamese & English), dark/light mode tại `http://localhost:3000` |
| **🔐 Bảo mật** | AES-256 vault, Prompt injection scanner (6 patterns), Audit trail, Rate limiting, Command allowlist |
| **📊 Monitoring** | Prometheus metrics (`/metrics`), analytics dashboard |

---

## 💼 Giải Pháp Theo Ngành

### 🛒 Bán Lẻ Online

**Vấn đề:** Mệt mỏi vì phải trả lời tin nhắn hỏi size/giá suốt ngày

**Giải pháp BizClaw:**
- 🤖 AI tư vấn sản phẩm tự động
- 📦 AI theo dõi đơn hàng
- 💬 AI chăm sóc khách hàng 24/7
- 📊 Workflow "Lead Qualification" sàng lọc khách hàng

**Setup:** 10 phút

---

### 🍜 F&B (Nhà Hàng, Cafe)

**Vấn đề:** Lúc ca đông khách gọi đặt bàn không kịp

**Giải pháp BizClaw:**
- 📅 Đặt bàn tự động qua Zalo
- 🍽️ AI gửi menu hàng ngày cho khách quen
- ✅ AI xác nhận đơn hàng tự động
- ⭐ AI phản hồi đánh giá Google

**Setup:** 15 phút

---

### ✈️ Du Lịch & Khách Sạn

**Vấn đề:** Booking từ nhiều kênh (Booking.com, Agoda) khó quản lý

**Giải pháp BizClaw:**
- 🔄 Đồng bộ booking tự động
- 💌 Tin nhắn chào mừng tự động
- ⏰ Nhắc nhở checkout
- 📧 Yêu cầu đánh giá sau khi ở

**Setup:** 20 phút

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

```toml
[[agents]]
id = "researcher"
model = "gemini/gemini-2.0-flash"
tools = ["web_search", "web_fetch", "browser"]

[[agents]]
id = "writer"
model = "anthropic/claude-sonnet-4-20250514"
tools = ["file_write", "social_post"]

[[agents]]
id = "support"
model = "openai/gpt-4o"
tools = ["zalo", "telegram", "email"]
```

Orchestration modes: **Sequential** (chain) · **Fan-Out** (parallel) · **Conditional** (routing) · **Loop** (iterative)

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

## 🏗️ Kiến Trúc

```
┌─────────────────────────────────────────────────────┐
│                    BizClaw Platform                  │
├─────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌───────────┐  │
│  │   Gateway   │  │  Scheduler  │  │   Hands   │  │
│  │   (Axum)    │  │   (Cron)    │  │(Background)│  │
│  └─────────────┘  └─────────────┘  └───────────┘  │
├─────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌───────────┐  │
│  │  Channels   │  │   Agents    │  │ Workflows │  │
│  │  (9+ kênh)  │  │  (Multi)    │  │  (10+)    │  │
│  └─────────────┘  └─────────────┘  └───────────┘  │
├─────────────────────────────────────────────────────┤
│  ┌───────────────────────────────────────────────┐  │
│  │            AI Providers (18)                  │  │
│  │  OpenAI · Anthropic · Gemini · DeepSeek      │  │
│  │  Groq · MiniMax · xAI · Mistral · ...       │  │
│  └───────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────┘
```

### Source Layout

```
bizclaw/
├── src/              # Rust core — gateway, agents, channels, providers
├── crates/           # 30 internal crates — tools, RAG, vault, workflows
├── dashboard/        # Web UI (Preact, 36 pages)
├── migrations/       # SQLite schema migrations
├── deploy/           # Docker & VPS deployment configs
├── docs/             # Architecture, API reference, guides
├── android/          # Android interaction client (62 Kotlin files)
└── training/         # BizClaw Academy materials
```

---

## 🔐 Bảo mật

| Tính năng | Mô tả |
|-----------|--------|
| **AES-256 Vault** | Mã hoá API keys at rest |
| **Prompt Injection Scanner** | 6 detection patterns (EN/VI/CN) |
| **Audit Trail** | Log mọi action |
| **Rate Limiting** | Per-IP protection |
| **Command Allowlist** | Sandboxed execution |

---

## 📖 Hướng Dẫn Chi Tiết

### Cách upload tài liệu cho AI học

1. Vào **📚 Knowledge**
2. Nhấn **"+ Thêm tài liệu"**
3. Chọn file (PDF, DOCX, TXT)
4. Đợi AI xử lý
5. AI sẽ trả lời từ nội dung tài liệu

### Cách thiết lập AI trả lời tự động

1. Vào **🤖 My Team** → Chọn Agent
2. Nhấn **"Cấu hình"**
3. Bật **"Trả lời tự động"**
4. Đặt giờ làm việc
5. Nhấn **"Lưu"**

---

## 📋 Checklist Bắt Đầu

- [ ] Tải và cài đặt BizClaw
- [ ] Kết nối Zalo OA hoặc Telegram
- [ ] Tạo Agent đầu tiên
- [ ] Upload tài liệu vào Knowledge
- [ ] Bật trả lời tự động
- [ ] Kiểm tra bằng cách gửi tin nhắn

---

## ❓ FAQ

### Cần bao nhiêu tiền để chạy BizClaw?

| Phương pháp | Chi phí |
|-------------|---------|
| Ollama (local) | **Miễn phí** - Không cần API key |
| DeepSeek | **~$0.1/ngày** - Rẻ hơn 10x so với GPT |
| GPT-4 | **~$2-5/ngày** - Tùy usage |

### Dữ liệu của tôi có an toàn không?

- ✅ **100% dữ liệu** lưu trên thiết bị của bạn
- ✅ **Không telemetry** - Không gửi data về server
- ✅ **Mã hóa AES-256** - API keys được mã hóa
- ✅ **Không cần server** - Chạy local

### Cần biết lập trình không?

**Không!** BizClaw được thiết kế cho người không biết lập trình:
- Giao diện tiếng Việt
- Kéo thả workflow
- Cài đặt bằng click chuột

---

## 🧪 SME Lean Model — Giảm 75% Nhân Sự Vận Hành

> BizClaw được thiết kế theo mô hình **SME Lean** — tối ưu AI-first, giảm nhân sự thủ công.

| Trước | Sau với BizClaw |
|-------|----------------|
| 8-12 nhân viên | 2-3 nhân viên |
| 10 posts/tuần | 50+ posts/tuần |
| Phản hồi 2-4 giờ | Phản hồi 5-15 phút |
| Content handmade | Content AI-generated |
| Manual scheduling | Auto-scheduling |

---

## 📖 Tài Liệu

| Tài liệu | Link |
|-----------|------|
| 📚 Hướng dẫn SME | [docs/sme-quickstart.md](docs/sme-quickstart.md) |
| 🏗️ Kiến trúc hệ thống | [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) |
| 🔧 API Reference | [docs/api/endpoints.md](docs/api/endpoints.md) |
| 📋 Changelog | [CHANGELOG.md](CHANGELOG.md) |
| 🙏 Ghi nhận đóng góp | [CREDITS.md](CREDITS.md) |
| ⚖️ Third-party notices | [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) |

---

## 🙏 Ghi Nhận & Credits

BizClaw được xây dựng nhờ nhiều dự án mã nguồn mở tuyệt vời:

### Tích hợp trực tiếp

| Dự án | Tác giả | Sử dụng | License |
|-------|---------|---------|---------|
| [zca-js](https://github.com/RFS-ADRENO/zca-js) | RFS-ADRENO | Giao thức Zalo API cá nhân — port từ JS sang Rust | MIT |
| [llama.cpp](https://github.com/ggerganov/llama.cpp) | Georgi Gerganov | Engine suy luận LLM trên thiết bị qua FFI | MIT |

### Nguồn cảm hứng

| Dự án | Bài học |
|-------|--------|
| **SkyClaw** | Model Router — auto-select tier model tối ưu |
| **GoClaw** | Phát hiện vòng lặp tool — nhận biết khi agent gọi lặp lại |
| **Paperclip** | Lớp điều phối agent — team agent phân cấp, ngân sách token |
| **OpenRAG** | Tìm kiếm lai FTS5 + vector, dis_max scoring, nudge system |
| **Docling** | Chunking tài liệu thông minh theo heading |
| **Claudia** (Eric Blue) | Privacy-first, proactive memory |
| **Memspan** | Context di động — Brain workspace |
| **Datrics Text2SQL** | NL-to-SQL pipeline, schema indexing |
| **OpenClaw-RL** | Interaction Signal Logger — học từ hội thoại |

### Hệ sinh thái Rust

[tokio](https://tokio.rs/) · [axum](https://github.com/tokio-rs/axum) · [reqwest](https://github.com/seanmonstar/reqwest) · [serde](https://serde.rs/) · [rusqlite](https://github.com/rusqlite/rusqlite) · [tokio-tungstenite](https://github.com/snapview/tokio-tungstenite) · [tracing](https://github.com/tokio-rs/tracing) · [lettre](https://github.com/lettre/lettre)

Chi tiết đầy đủ: [CREDITS.md](CREDITS.md) · [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)

---

## 📝 License

[MIT License](LICENSE) — Sử dụng tự do cho mục đích thương mại và phi thương mại.

---

## 📞 Liên Hệ

| Kênh | Thông tin |
|------|----------|
| 🌐 **Website** | [bizclaw.vn](https://bizclaw.vn) |
| 📧 **Email** | support@bizclaw.vn |
| 💬 **Zalo OA** | BizClaw |
| 📘 **Fanpage** | [facebook.com/bizclaw.vn](https://www.facebook.com/bizclaw.vn) |

**Giờ hỗ trợ:** Thứ 2 - Thứ 6, 8:00 - 17:00 (GMT+7)

---

<p align="center">
  <strong>Built with ❤️ for SME Việt Nam</strong><br>
  <a href="https://bizclaw.vn">bizclaw.vn</a> · <a href="https://facebook.com/bizclaw.vn">Facebook</a> · <a href="mailto:support@bizclaw.vn">Support</a>
</p>
