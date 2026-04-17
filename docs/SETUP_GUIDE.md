# BizClaw - Hướng Dẫn Cài Đặt & Sử Dụng

**Phiên bản**: 1.1.7  
**Cập nhật**: 2026-04-17

---

## 📋 Mục lục

1. [Yêu cầu hệ thống](#1-yêu-cầu-hệ-thống)
2. [Cài đặt nhanh](#2-cài-đặt-nhanh)
3. [Cấu hình](#3-cấu-hình)
4. [Chạy ứng dụng](#4-chạy-ứng-dụng)
5. [API Endpoints](#5-api-endpoints)
6. [Use Cases chính](#6-use-cases-chính)

---

## 1. Yêu cầu hệ thống

### Hardware
- **CPU**: 2 cores minimum (4 cores recommended)
- **RAM**: 4GB minimum (8GB recommended)
- **Disk**: 500MB free space

### Software
- **Rust**: 1.75+ (install via [rustup](https://rustup.rs/))
- **Git**: Latest version
- **Database**: SQLite (bundled)

### API Keys (Optional)
- MiniMax API Key (for AI features)
- OpenAI API Key (alternative)
- Claude API Key (alternative)

---

## 2. Cài đặt nhanh

### Clone repository
```bash
git clone https://github.com/nguyenduchoai/bizclaw.git
cd bizclaw
```

### Build
```bash
cargo build --release
```

### Chạy
```bash
./target/release/bizclaw serve
```

Truy cập: http://localhost:3001

---

## 3. Cấu hình

### Environment Variables (Optional)
```bash
# MiniMax API (khuyến nghị)
export MINIMAX_API_KEY=your_api_key_here

# OpenAI (backup)
export OPENAI_API_KEY=your_api_key_here

# Claude (backup)  
export ANTHROPIC_API_KEY=your_api_key_here
```

### Config file (~/.bizclaw/)
BizClaw tự động tạo cấu hình tại `~/.bizclaw/`

Các file quan trọng:
- `agents.json` - Agent definitions
- `gateway.db` - SQLite database
- `orchestration.db` - Orchestration state

---

## 4. Chạy ứng dụng

### Development mode
```bash
cargo run -- serve -p 3001
```

### Production mode
```bash
./target/release/bizclaw serve -p 3001 --prod
```

### Docker
```bash
docker build -t bizclaw:latest .
docker run -p 3001:3001 \
  -e MINIMAX_API_KEY=your_key \
  bizclaw:latest
```

---

## 5. API Endpoints

### Health Check
```bash
curl http://localhost:3001/health
```

### Agents
```bash
# List agents
curl http://localhost:3001/api/v1/agents

# Chat với agent
curl -X POST http://localhost:3001/api/v1/agents/mama/chat \
  -H "Content-Type: application/json" \
  -d '{"message":"xin chào"}'
```

### CRM
```bash
# List contacts
curl http://localhost:3001/api/v1/crm/contacts

# Dashboard
curl http://localhost:3001/api/v1/crm/dashboard

# Conversations
curl http://localhost:3001/api/v1/crm/conversations
```

### Skills
```bash
# List all skills
curl http://localhost:3001/api/v1/skills

# Search OpenHub skills
curl "http://localhost:3001/api/v1/skills/openhub/search?q=seo"

# Quick install skill
curl "http://localhost:3001/api/v1/skills/openhub/quick?q=accounting"
```

---

## 6. Use Cases chính

### 6.1 Chat với AI Agent
```bash
# Gửi message cho MAMA (router)
curl -X POST http://localhost:3001/api/v1/agents/mama/chat \
  -H "Content-Type: application/json" \
  -d '{"message":"tôi cần tư vấn sản phẩm"}'
```

### 6.2 Tìm và cài skill mới
```bash
# Tìm skill SEO
curl "http://localhost:3001/api/v1/skills/openhub/quick?q=seo"

# Chat với skill mới
curl -X POST "http://localhost:3001/api/v1/agents/hunter_seo_expert/chat" \
  -H "Content-Type: application/json" \
  -d '{"message":"tối ưu bài viết cho từ khóa AI"}'
```

### 6.3 Quản lý CRM
```bash
# Tạo contact
curl -X POST http://localhost:3001/api/v1/crm/contacts \
  -H "Content-Type: application/json" \
  -d '{"name":"Nguyễn Văn A","channel":"zalo","channel_id":"zalo_123","phone":"0909123456"}'

# Cập nhật pipeline
curl -X PUT http://localhost:3001/api/v1/crm/contacts/{id}/pipeline \
  -H "Content-Type: application/json" \
  -d '{"status":"contacted"}'
```

### 6.4 Webhooks
```bash
# Zalo webhook
curl -X POST http://localhost:3001/webhooks/zalo \
  -H "Content-Type: application/json" \
  -d '{"event":"user_message","data":{...}}'
```

---

## 🆘 Troubleshooting

### Lỗi "API Key not configured"
```bash
export MINIMAX_API_KEY=your_key
./target/release/bizclaw serve
```

### Lỗi port đã sử dụng
```bash
./target/release/bizclaw serve -p 3002
```

### Xem logs
```bash
# Debug mode
RUST_LOG=debug ./target/release/bizclaw serve
```

---

## 📚 Tài liệu thêm

- [README.md](README.md) - Tổng quan
- [ARCHITECTURE.md](ARCHITECTURE.md) - Kiến trúc hệ thống
- [API Documentation](api/endpoints.md)
- [Project Completion Report](PROJECT_COMPLETION_REPORT.md)
- [UI/UX Audit Report](UI_UX_AUDIT_REPORT.md)

---

## 🔗 Links

- **GitHub**: https://github.com/nguyenduchoai/bizclaw
- **Documentation**: https://bizclaw.vn/docs
- **Support**: support@bizclaw.vn

---

*© 2024-2026 BizClaw. All rights reserved.*
