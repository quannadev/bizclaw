# 📖 Module 13: Cài Đặt & Cấu Hình BizClaw

> **Phase**: 🔧 TOOLSET  
> **Buổi**: 13/24  
> **Thời lượng**: 2 giờ  
> **Skills tham chiếu**: `devops`, `docker-expert`, `server-management`

---

## 🎯 Mục Tiêu Học Tập

- [ ] Cài đặt BizClaw từ source code
- [ ] Cấu hình config.toml cho production
- [ ] Chạy với Docker và systemd
- [ ] Setup Ollama cho local inference

---

## 📋 Nội Dung

### 1. Ba Cách Cài Đặt

#### 1.1 One-Click Install (VPS/Pi)
```bash
curl -sSL https://bizclaw.vn/install.sh | sudo bash
# → Auto-detect OS, install Rust + BizClaw + systemd service
```

#### 1.2 Docker
```bash
git clone https://github.com/nguyenduchoai/bizclaw-cloud
cd bizclaw && docker-compose up -d
# → Container: bizclaw + bizclaw-platform
```

#### 1.3 Build From Source
```bash
git clone https://github.com/nguyenduchoai/bizclaw-cloud.git
cd bizclaw
cargo build --release
# Binary: target/release/bizclaw (~12MB)
# Binary: target/release/bizclaw-platform (~7.7MB)
```

### 2. Cấu Hình config.toml

```toml
# ~/.bizclaw/config.toml

[general]
name = "BizClaw Agent"
language = "vi"

[provider]
type = "openai"
model = "gpt-4o-mini"
api_key = "sk-..."

[brain]
enabled = true
model_path = "/path/to/model.gguf"

[security]
command_allowlist = ["ls", "cat", "grep", "find", "df"]
sandbox_enabled = true

[[channels]]
type = "telegram"
bot_token = "123456:ABC..."
enabled = true

[[channels]]
type = "discord"
bot_token = "MTIz..."
enabled = true

[[mcp_servers]]
name = "pageindex"
command = "npx"
args = ["-y", "@pageindex/mcp"]
```

### 3. Chế Độ Triển Khai

| Mode | Binaries | Use Case |
|------|----------|----------|
| **Standalone** | `bizclaw` only | 1 bot, cá nhân, dev |
| **Platform** | `bizclaw` + `bizclaw-platform` | Multi-tenant, agency |

```bash
# Standalone mode
./target/release/bizclaw serve --port 3579

# Platform mode
./target/release/bizclaw-platform --port 3001 &
# → Admin at http://localhost:3001/admin/
# → Each tenant gets own port (10001, 10002, ...)
```

### 4. Setup Ollama (Local LLM)

```bash
# Install Ollama
curl -fsSL https://ollama.ai/install.sh | sh

# Pull models (shared across ALL tenants)
ollama pull llama3.2      # ~3.8GB, general purpose
ollama pull qwen3         # ~4.7GB, good for Vietnamese
ollama pull phi3           # ~2.3GB, lightweight

# BizClaw auto-detects Ollama at localhost:11434
```

### 5. Systemd Service (Production)

```ini
# /etc/systemd/system/bizclaw-platform.service
[Unit]
Description=BizClaw Platform
After=network.target

[Service]
Type=simple
User=root
ExecStart=/root/bizclaw/target/release/bizclaw-platform --port 3001
WorkingDirectory=/root/bizclaw
Restart=always
RestartSec=5
Environment=JWT_SECRET=your-secure-secret-here
Environment=BIZCLAW_CORS_ORIGINS=https://bizclaw.vn

[Install]
WantedBy=multi-user.target
```

```bash
sudo systemctl daemon-reload
sudo systemctl enable bizclaw-platform
sudo systemctl start bizclaw-platform
sudo systemctl status bizclaw-platform
```

### 6. Verification Checklist

- [ ] `bizclaw serve` starts without errors
- [ ] Dashboard accessible at http://localhost:3579
- [ ] WebSocket chat works
- [ ] At least 1 provider configured
- [ ] Ollama models pulled (optional)
- [ ] Systemd service auto-restarts on failure

---

## 📝 Lab

### Lab: Full Installation (45 phút)

1. Clone repo + build from source
2. Create config.toml with OpenAI key
3. Start standalone mode, verify dashboard
4. Install Ollama + pull qwen3
5. Switch provider to Ollama, test chat
6. (Optional) Setup systemd service

---

## ⏭️ Buổi Tiếp Theo

**Module 14: Dashboard & Agent Management**
