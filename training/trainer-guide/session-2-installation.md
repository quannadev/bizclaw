# 📖 Session 2: Hands-on Installation & Dashboard

> ⏰ **09:45 - 11:15** (1.5 giờ)  
> 🎯 **Mục tiêu**: Trainer tự cài + vận hành BizClaw, thành thạo Dashboard

---

## 🔧 Phần A: Cài Đặt (30 phút)

### A1: 3 Phương Pháp — Chọn theo tình huống

| Tình huống | Phương pháp | Command |
|-----------|------------|---------|
| Demo nhanh cho khách | Docker | `docker-compose up -d` |
| Triển khai cho khách (VPS) | One-click | `curl -sSL bizclaw.vn/install.sh \| sudo bash` |
| Dev/customize | Source build | `cargo build --release` |

### A2: Lab — Cài Đặt Trực Tiếp (Trainer làm)

```bash
# Bước 1: Clone
git clone https://github.com/nguyenduchoai/bizclaw-cloud.git
cd bizclaw

# Bước 2: Build (5-10 phút lần đầu)
cargo build --release

# Bước 3: Khởi động
./target/release/bizclaw serve --port 3579

# Bước 4: Mở browser
# → http://localhost:3579/dashboard
```

### A3: Cài Ollama (cho demo offline)

```bash
# Cài Ollama
curl -fsSL https://ollama.ai/install.sh | sh

# Pull model (chọn 1):
ollama pull qwen3         # 4.7GB — tốt cho tiếng Việt
ollama pull llama3.2      # 3.8GB — general purpose
ollama pull phi3           # 2.3GB — nhẹ nhất, demo nhanh

# BizClaw tự detect Ollama tại localhost:11434
```

### A4: Config Cơ Bản

```toml
# ~/.bizclaw/config.toml — Trainer setup

[provider]
type = "openai"          # Hoặc "ollama" cho offline
model = "gpt-4o-mini"    # Hoặc "qwen3" 
api_key = "sk-..."       # Bỏ trống nếu dùng Ollama

[security]
command_allowlist = ["ls", "cat", "grep", "find", "df", "date"]
```

---

## 🖥️ Phần B: Dashboard Walkthrough (30 phút)

### B1: 15 Trang — Trainer phải biết hết

**Tour nhanh (Trainer demo cho khách theo thứ tự này):**

```
1. 📊 Dashboard    → "Đây là tổng quan — thấy ngay server status"
2. 💬 Chat         → "Chat trực tiếp với AI — demo ngay"
3. 🖼️ Gallery      → "51 agent sẵn sàng — chọn theo nghiệp vụ"
4. 🤖 Agents       → "Quản lý nhiều agent — mỗi agent 1 vai trò"
5. 🔌 Providers    → "15 nhà cung cấp AI — chọn theo budget"
6. 📱 Channels     → "Kết nối Telegram, Zalo, Email..."
7. 📚 Knowledge    → "Upload tài liệu DN → Agent tra cứu được"
8. 🛠️ Tools        → "13 công cụ: file, shell, web search..."
9. ⏰ Scheduler    → "Hẹn giờ: agent tự chạy lúc 8h mỗi sáng"
10. 📊 Traces      → "Theo dõi chi phí — biết rõ tốn bao nhiêu"
```

**Các trang còn lại (biết nhưng ít demo):**
- ⚙️ Settings, 🔗 MCP, 🧠 Brain, 📄 Config, 📈 Activity

### B2: Chat Demo — Kịch Bản Hay Nhất

**Demo 1: Agent thông minh (dùng tools)**
```
Trainer gõ: "Hôm nay thứ mấy?"
Agent: [Dùng shell tool] → "Hôm nay là Thứ Sáu, 28/02/2026"

→ Chỉ cho khách: Agent HÀNH ĐỘNG, không chỉ nói
```

**Demo 2: Đa ngôn ngữ**
```
Trainer gõ: "Dịch sang tiếng Anh: BizClaw là nền tảng AI Agent"
Agent: "BizClaw is an AI Agent platform..."

→ Chỉ cho khách: Hiểu tiếng Việt hoàn hảo
```

**Demo 3: Memory**
```
Trainer gõ: "Tên tôi là Hoài, tôi làm ở Bizino"
Agent: "Xin chào Hoài! Đã ghi nhớ."
[Sau vài câu khác]
Trainer gõ: "Tôi làm ở đâu nhỉ?"
Agent: "Bạn Hoài làm việc tại Bizino."

→ Chỉ cho khách: Agent NHỚ — không như ChatGPT reset mỗi phiên
```

### B3: Gallery — Cài Agent 1 Click

```
Dashboard → Gallery → Chọn category phù hợp với khách:

Khách F&B?      → 📞 Customer Service → "Hỗ trợ khách hàng"
Khách Retail?   → 🛒 E-commerce → "Quản lý đơn hàng"
Khách Tech?     → 💻 IT → "Helpdesk Support"
Khách tổng hợp? → 💼 Management → "Báo cáo tổng hợp"

Click Install → Agent sẵn sàng trong 3 giây
```

---

## 🎯 Phần C: Lab Thực Hành (30 phút)

### Trainer tự làm:

- [ ] Cài BizClaw thành công
- [ ] Mở Dashboard — navigate 10 trang chính
- [ ] Chat: test 5 câu (tools, memory, tiếng Việt)
- [ ] Cài 2 agent templates từ Gallery
- [ ] Cấu hình provider (OpenAI hoặc Ollama)
- [ ] Xem LLM Traces — hiểu cost tracking

### Câu hỏi tự kiểm tra:

1. Binary bizclaw nằm ở đâu sau `cargo build --release`?
2. Dashboard chạy trên port mấy mặc định?
3. Ollama models được lưu ở đâu?
4. Làm sao biết agent đang dùng provider nào?

---

## ✅ Checkpoint Session 2

- [ ] Cài BizClaw thành công (verify: Dashboard accessible)
- [ ] Biết cài bằng 3 cách (Docker, one-click, source)
- [ ] Navigate Dashboard thành thạo (10 trang)
- [ ] Demo Chat với 3 kịch bản (tools, memory, tiếng Việt)
- [ ] Cài được agent template từ Gallery

---

*☕ Break 15 phút → Session 3*
