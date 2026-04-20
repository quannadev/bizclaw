# 🧪 E2E Test Guide - SME Workflow

## 🎯 Mục tiêu
Test BizClaw như một **SME thật sự** với workflow:
1. Thu thập tin nhắn từ **Zalo**
2. Tổng hợp thông tin từ **các kênh**
3. Viết **bài content** tự động
4. **Đăng bài** lên các nền tảng

---

## 📋 Prerequisites

### 1. Cài đặt BizClaw
```bash
# Build từ source
git clone https://github.com/nguyenduchoai/bizclaw.git
cd bizclaw
cargo build --release

# Hoặc download binary
# macOS: ./target/release/bizclaw-desktop
# Linux: ./target/release/bizclaw-local
```

### 2. Chuẩn bị API Keys
```bash
# Tạo file .env
cat > .env << EOF
# AI Providers (chọn ít nhất 1)
OPENAI_API_KEY=sk-xxx
ANTHROPIC_API_KEY=sk-ant-xxx
GEMINI_API_KEY=xxx

# Zalo (Zalo OA)
ZALO_APP_ID=your_app_id
ZALO_APP_SECRET=your_app_secret

# Social Media (chọn theo nhu cầu)
FACEBOOK_PAGE_ACCESS_TOKEN=xxx
INSTAGRAM_ACCESS_TOKEN=xxx
TIKTOK_ACCESS_TOKEN=xxx

# Database
DATABASE_URL=sqlite:bizclaw.db
EOF
```

### 3. Tạo Zalo OA (Nếu chưa có)
1. Vào https://developers.zalo.me/
2. Tạo ứng dụng Zalo Official Account
3. Lấy App ID và App Secret
4. Enable các tính năng: Messages, Timeline API

---

## 🚀 Bước 1: Khởi động BizClaw

### Desktop Mode (Recommended cho test)
```bash
# macOS/Linux
./target/release/bizclaw-desktop

# Hoặc với config
./target/release/bizclaw-desktop --config ./config.quickstart.toml
```

### Server Mode (Cho production)
```bash
./target/release/bizclaw serve --tunnel
```

### Kiểm tra server đang chạy
```bash
curl http://localhost:8080/health
# Response: {"status":"ok","version":"1.1.7"}
```

Mở browser: http://localhost:3000

---

## 🔗 Bước 2: Kết nối Zalo Channel

### Qua Web UI
1. Mở http://localhost:3000
2. Login/Register
3. Vào **Settings → Channels**
4. Click **Zalo OA**
5. Nhập App ID và App Secret
6. Click **Connect**

### Qua API
```bash
curl -X POST http://localhost:8080/api/channels/zalo/connect \
  -H "Content-Type: application/json" \
  -d '{
    "app_id": "your_app_id",
    "app_secret": "your_app_secret"
  }'
```

### Verify kết nối
```bash
curl http://localhost:8080/api/channels/zalo/status
# Response: {"connected": true, "channel": "zalo"}
```

---

## 🤖 Bước 3: Tạo Agent cho SME

### Agent 1: Thu thập tin nhắn (Collector)
```bash
curl -X POST http://localhost:8080/api/agents \
  -H "Content-Type: application/json" \
  -d '{
    "name": "collector-agent",
    "role": "collector",
    "description": "Thu thập và phân loại tin nhắn từ Zalo",
    "system_prompt": "Bạn là trợ lý thu thập tin nhắn. Nhiệm vụ của bạn:
      1. Đọc tin nhắn từ Zalo
      2. Phân loại: câu hỏi, phản hồi, khiếu nại, đơn hàng
      3. Trích xuất thông tin quan trọng
      4. Lưu vào memory để tổng hợp",
    "tools": ["zalo_read_messages", "memory_store"]
  }'
```

### Agent 2: Tổng hợp nội dung (Synthesizer)
```bash
curl -X POST http://localhost:8080/api/agents \
  -H "Content-Type: application/json" \
  -d '{
    "name": "synthesizer-agent", 
    "role": "synthesizer",
    "description": "Tổng hợp thông tin thành bài viết",
    "system_prompt": "Bạn là chuyên gia viết content. Nhiệm vụ:
      1. Đọc dữ liệu từ memory
      2. Tổng hợp thành nội dung hấp dẫn
      3. Viết theo phong cách: thân thiện, chuyên nghiệp, có emoji
      4. Tạo nhiều versions cho các nền tảng khác nhau",
    "tools": ["memory_search", "content_generator"]
  }'
```

### Agent 3: Đăng bài (Publisher)
```bash
curl -X POST http://localhost:8080/api/agents \
  -H "Content-Type: application/json" \
  -d '{
    "name": "publisher-agent",
    "role": "publisher", 
    "description": "Đăng bài lên các kênh",
    "system_prompt": "Bạn là chuyên gia đăng bài. Nhiệm vụ:
      1. Nhận nội dung đã viết
      2. Format phù hợp cho từng nền tảng
      3. Đăng lên Zalo, Facebook, Instagram theo lịch
      4. Theo dõi engagement và báo cáo",
    "tools": ["zalo_post", "facebook_post", "instagram_post", "telegram_post"]
  }'
```

---

## 📝 Bước 4: Tạo Workflow Tự động

### Workflow: Daily Content Pipeline
```bash
curl -X POST http://localhost:8080/api/workflows \
  -H "Content-Type: application/json" \
  -d '{
    "name": "daily-content-pipeline",
    "description": "Thu thập → Tổng hợp → Đăng bài",
    "trigger": {
      "type": "cron",
      "schedule": "0 9 * * *"  # Chạy 9h sáng hàng ngày
    },
    "steps": [
      {
        "agent": "collector-agent",
        "action": "collect_messages",
        "params": {
          "channels": ["zalo", "telegram"],
          "time_range": "24h"
        }
      },
      {
        "agent": "synthesizer-agent", 
        "action": "generate_content",
        "params": {
          "style": "sme_friendly",
          "platforms": ["zalo", "facebook"]
        }
      },
      {
        "agent": "publisher-agent",
        "action": "publish",
        "params": {
          "channels": ["zalo", "facebook"],
          "schedule": "immediate"
        }
      }
    ]
  }'
```

---

## 🧪 Bước 5: E2E Test Scenarios

### Test Case 1: Thu thập tin nhắn Zalo
```bash
# 1. Gửi tin nhắn test đến Zalo OA
# (Từ điện thoại của bạn)

# 2. Kiểm tra BizClaw nhận được
curl http://localhost:8080/api/messages?channel=zalo&limit=10

# Expected: Danh sách tin nhắn gần đây
```

### Test Case 2: Tổng hợp thông tin
```bash
# Trigger synthesizer agent
curl -X POST http://localhost:8080/api/agents/synthesizer-agent/run \
  -H "Content-Type: application/json" \
  -d '{
    "action": "generate_content",
    "context": {
      "topic": "Khuyến mãi cuối tuần",
      "source": "zalo_conversations"
    }
  }'

# Expected: Content được tạo ra
```

### Test Case 3: Đăng bài lên Zalo
```bash
# Post lên Zalo OA
curl -X POST http://localhost:8080/api/channels/zalo/post \
  -H "Content-Type: application/json" \
  -d '{
    "content": "🎉 Chào mừng cuối tuần! Giảm 20% cho tất cả sản phẩm. Liên hệ ngay!",
    "type": "text"
  }'

# Verify: Kiểm tra Zalo OA page
```

### Test Case 4: Full Pipeline
```bash
# Chạy workflow
curl -X POST http://localhost:8080/api/workflows/daily-content-pipeline/run \
  -H "Content-Type: application/json"

# Verify:
# 1. Tin nhắn được thu thập
# 2. Content được tạo  
# 3. Bài được đăng lên các kênh
```

---

## 📊 Bước 6: Verify Kết quả

### Check Logs
```bash
# Xem logs
tail -f logs/bizclaw.log

# Hoặc qua API
curl http://localhost:8080/api/logs?level=info&limit=50
```

### Check Dashboard
```
http://localhost:3000/dashboard
```

Kiểm tra:
- ✅ Số tin nhắn đã xử lý
- ✅ Số bài đã đăng
- ✅ Engagement metrics

### Check Memory
```bash
curl http://localhost:8080/api/memory/stats
```

---

## 🎯 Success Criteria

| Criteria | Target | Actual | Status |
|----------|--------|--------|--------|
| Kết nối Zalo | ✅ | ? | ⏳ |
| Nhận tin nhắn | ✅ | ? | ⏳ |
| Tổng hợp content | ✅ | ? | ⏳ |
| Đăng bài tự động | ✅ | ? | ⏳ |
| Không crash | ✅ | ? | ⏳ |

---

## 🐛 Troubleshooting

### Zalo không kết nối được
```bash
# Check credentials
curl http://localhost:8080/api/channels/zalo/config

# Verify OAuth
# Zalo OA cần được approve bởi Zalo team trước khi dùng được API
```

### Agent không chạy
```bash
# Check agent status
curl http://localhost:8080/api/agents

# Check memory
curl http://localhost:8080/api/memory/health
```

### Content không được tạo
```bash
# Check AI provider
curl http://localhost:8080/api/providers/health
```

---

## 📞 Cần hỗ trợ?

1. Check logs: `tail -f logs/bizclaw.log`
2. Check API docs: http://localhost:8080/api/docs
3. Check GitHub issues: https://github.com/nguyenduchoai/bizclaw-cloud/issues
