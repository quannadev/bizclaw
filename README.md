# 🦞 BizClaw - AI Agent Platform cho Doanh Nghiệp Việt

<p align="center">
  <img src="assets/bizclaw-logo.png" alt="BizClaw" width="200">
</p>

<p align="center">
  <strong>Nhân viên AI làm việc 24/7 cho doanh nghiệp của bạn</strong><br>
  Không cần biết lập trình • Giao diện tiếng Việt • Dữ liệu 100% thuộc về bạn
</p>

<p align="center">
  <a href="https://github.com/nguyenduchoai/bizclaw-cloud/actions"><img src="https://img.shields.io/github/actions/workflow/status/nguyenduchoai/bizclaw-cloud/ci?style=flat-square" alt="Build"></a>
  <a href="https://github.com/nguyenduchoai/bizclaw-cloud/releases"><img src="https://img.shields.io/github/v/release/nguyenduchoai/bizclaw-cloud?style=flat-square" alt="Version"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/License-MIT-blue.svg?style=flat-square" alt="License"></a>
  <a href="https://discord.gg/bizclaw"><img src="https://img.shields.io/badge/Discord-Join-7289DA?style=flat-square&logo=discord" alt="Discord"></a>
</p>

---

## 🎯 BizClaw Dành Cho Ai?

| Đối tượng | Lợi ích |
|-----------|---------|
| 🛒 **Shop online** | Trả lời tin nhắn tự động, tư vấn sản phẩm 24/7 |
| 🍜 **Chủ nhà hàng** | Đặt bàn tự động, gửi menu hàng ngày |
| 🏨 **Chủ khách sạn** | Xác nhận booking, chăm sóc khách |
| 💼 **Dịch vụ B2B** | Báo giá tự động, lịch hẹn tự động |
| 📱 **Marketer** | Đăng bài đa nền tảng, nuôi lead tự động |

---

## 🚀 Bắt Đầu Trong 5 Phút

### Cách 1: Desktop App (Khuyến nghị)

```bash
# macOS / Linux
curl -fsSL https://bizclaw.vn/install.sh | bash

# Windows (PowerShell)
irm https://bizclaw.vn/install.ps1 | iex

# Khởi động
./bizclaw-desktop
```

### Cách 2: Docker

```bash
docker run -d \
  --name bizclaw \
  -p 3000:3000 \
  -p 8080:8080 \
  -v bizclaw-data:/data \
  nguyenduchoai/bizclaw:latest
```

### Cách 3: Build từ Source

```bash
git clone https://github.com/nguyenduchoai/bizclaw.git
cd bizclaw
cargo build --release
./target/release/bizclaw-desktop
```

---

## ✨ Tính Năng Nổi Bật

### 🤖 AI Agent Thông Minh
- Hiểu ý khách, trả lời tự nhiên bằng tiếng Việt
- Học từ mỗi cuộc trò chuyện để phục vụ tốt hơn
- Đa ngôn ngữ: Việt, Anh, Trung...

### � 12+ Kênh Kết Nối

| Kênh | Type | Status |
|------|------|--------|
| � Zalo OA | Official Account | ✅ |
| 💬 Telegram | Bot API | ✅ |
| 👥 Facebook | Page / Messenger | ✅ |
| 📸 Instagram | DM / Comments | ✅ |
| 🎵 TikTok | Comments / DM | ✅ |
| � Shopee | Chat / Orders | ✅ |
| 💼 Slack | Team Chat | ✅ |
| 🎮 Discord | Server / DM | ✅ |
| 📧 Email | IMAP/SMTP | ✅ |
| 🌐 Webhook | Custom API | ✅ |
| � WhatsApp | Business | ✅ |
| ➕ Custom | Webhook | ✅ |

### 🔌 18+ AI Providers

| Provider | Models |
|----------|--------|
| OpenAI | GPT-4o, GPT-4o Mini |
| Anthropic | Claude Sonnet 4, 3.5 |
| Google Gemini | 2.5 Pro, 2.5 Flash |
| DeepSeek | Chat, Reasoner (R1) |
| Groq | Llama 3.3 70B |
| Ollama | Qwen3, Llama 3.2 |
| Llama.cpp | Local GGUF |
| Cohere | Command R+ |
| Perplexity | Sonar Pro |
| DashScope | Qwen Max |

### 🛠️ 40+ Tools

| Category | Tools |
|----------|-------|
| **Browser** | Browser automation, Stealth mode, Screenshot |
| **Social** | Auto post, Multi-platform scheduling |
| **Database** | SQL query, Semantic search, Schema |
| **AI** | Content generation, Image generation, Voice |
| **Utility** | File, Shell, HTTP, Calendar |

---

## 📊 So Sánh

| Feature | BizClaw | OpenClaw | GoClaw | RsClaw |
|---------|---------|----------|--------|---------|
| **Binary Size** | 16-22MB | ~300MB | ~25MB | ~15MB |
| **AI Providers** | 18+ | ~10 | ~12 | 15+ |
| **Channels** | 12+ | 8 | 7 | 13 |
| **VN Channels** | ✅ Zalo, Shopee, TikTok | ❌ | ❌ | ❌ |
| **Security** | AES-256, Vault, RBAC | ❌ | ✅ | ❌ |
| **MCP Protocol** | ✅ | ❌ | ❌ | ❌ |
| **Model Router** | ✅ 3 tiers | ❌ | ❌ | ❌ |

---

## 🏗️ Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      BizClaw Architecture                   │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐  │
│  │  Channels   │    │   Gateway   │    │  Dashboard  │  │
│  │  Zalo/TG/  │───▶│  Actix-web  │◀───│   Web UI    │  │
│  │  FB/IG/... │    │    REST     │    │   React     │  │
│  └─────────────┘    └──────┬──────┘    └─────────────┘  │
│                            │                             │
│                     ┌──────┴──────┐                     │
│                     │    Agent     │                     │
│                     │  Orchestrator│                     │
│                     └──────┬──────┘                     │
│                            │                             │
│         ┌───────────────────┼───────────────────┐          │
│         │                   │                   │          │
│  ┌─────┴─────┐     ┌─────┴─────┐     ┌─────┴─────┐    │
│  │  Providers │     │   Tools   │     │  Memory   │    │
│  │ 18+ LLMs  │     │   40+    │     │  Vector+  │    │
│  └───────────┘     └───────────┘     └───────────┘    │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

---

## 📖 Documentation

| Document | Description |
|----------|-------------|
| [Quick Start](docs/QUICK_START.md) | Bắt đầu nhanh |
| [API Reference](docs/API.md) | REST API documentation |
| [Channels Guide](docs/CHANNELS.md) | Kết nối kênh |
| [AI Providers](docs/PROVIDERS.md) | Cấu hình AI providers |
| [Deployment](docs/DEPLOYMENT.md) | Deploy lên server |
| [Examples](docs/EXAMPLES.md) | Code examples |

---

## 💰 Bảng Giá

| Plan | Giá | Channels | Messages | Agents |
|------|------|----------|----------|--------|
| **Free** | 0đ/tháng | 1 | 100/tháng | 1 |
| **Pro** | 499K/tháng | 5 | 5,000/tháng | 5 |
| **Enterprise** | Liên hệ | Unlimited | Unlimited | Unlimited |

[Đăng ký ngay](https://bizclaw.vn/pricing)

---

## 🏆 Case Studies

### 🛒 Shop Thời Trang - TP.HCM
> "Trước tôi phải thuê 2 nhân viên chăm sóc Zalo. Giờ BizClaw làm hết, tiết kiệm **15 triệu/tháng**."
> — Chị Lan, Chủ shop thời trang

### 🍜 Nhà Hàng - Đà Nẵng
> "Khách đặt bàn tự động lúc 11h đêm. Sáng hôm sau tôi chỉ cần xác nhận."
> — Anh Tuấn, Chủ nhà hàng

### 🏨 Homestay - Vũng Tàu
> "BizClaw trả lời nhanh, đúng thông tin, không cần tôi cầm điện thoại suốt ngày."
> — Chị Hương, Chủ homestay

---

## 🤝 Contributing

Chúng tôi welcomes contributions! Xem [CONTRIBUTING.md](CONTRIBUTING.md) để biết thêm chi tiết.

```bash
# Fork repo
# Tạo branch mới
git checkout -b feature/amazing-feature

# Commit changes
git commit -m "feat: add amazing feature"

# Push và tạo PR
git push origin feature/amazing-feature
```

---

## 📞 Liên Hệ

| Kênh | Link |
|------|------|
| 🌐 Website | [bizclaw.vn](https://bizclaw.vn) |
| � Discord | [Discord Community](https://discord.gg/bizclaw) |
| � Twitter | [@bizclawvn](https://twitter.com/bizclawvn) |
| 📧 Email | contact@bizclaw.vn |

---

## 📄 License

MIT License - Xem [LICENSE](LICENSE) để biết chi tiết.

---

<p align="center">
  <strong>Made with ❤️ in Vietnam</strong><br>
  <sub>BizClaw - Trợ lý AI 24/7 cho doanh nghiệp Việt</sub>
</p>
