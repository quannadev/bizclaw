# 📱 Telegram Quick Start Guide

## 🎯 Mục tiêu
Test BizClaw với Telegram - đơn giản và nhanh nhất!

---

## 📋 Prerequisites

### 1. Bot Token từ @BotFather
1. Mở Telegram, tìm **@BotFather**
2. Gửi `/newbot`
3. Đặt tên bot (VD: "BizClaw Test Bot")
4. Đặt username bot (phải kết thúc bằng `bot`, VD: `bizclaw_test_bot`)
5. Copy **Bot Token**: `123456789:ABCdefGHIjklMNOpqrsTUVwxyz`

---

## 🔧 Cấu hình

### Cách 1: Qua Web UI
1. Mở http://localhost:3000
2. Settings → Channels → Telegram
3. Nhập Bot Token
4. Click **Connect**

### Cách 2: Qua Config File
```bash
# Tạo file config
cat > config.telegram.toml << 'EOF'
[channels.telegram]
enabled = true
bot_token = "YOUR_BOT_TOKEN_HERE"
EOF

# Chạy với config
./target/release/bizclaw-desktop --config config.telegram.toml
```

### Cách 3: Qua Environment
```bash
export TELEGRAM_BOT_TOKEN="YOUR_BOT_TOKEN_HERE"
./target/release/bizclaw-desktop
```

---

## 🚀 Khởi động

```bash
cd /Users/digits/Github/bizclaw

# Với config
./target/release/bizclaw-desktop --config config.telegram.toml

# Hoặc export env trước
export TELEGRAM_BOT_TOKEN="YOUR_BOT_TOKEN_HERE"
./target/release/bizclaw-desktop
```

---

## 🧪 Test ngay!

### 1. Mở Telegram
- Tìm bot của bạn (username bạn đặt lúc nãy)
- Click **Start** hoặc gửi tin nhắn

### 2. Test Commands cơ bản
Gửi cho bot:

| Command | Mô tả |
|---------|--------|
| `/start` | Bắt đầu |
| `/help` | Trợ giúp |
| `/status` | Kiểm tra trạng thái |

### 3. Test AI Chat
```
Gửi: "Xin chào, bạn là ai?"
```

**Expected**: Bot trả lời bằng tiếng Việt!

### 4. Test với Vietnamese
```
Gửi: "Giới thiệu về sản phẩm của bạn"
Gửi: "Tôi muốn đặt hàng"
Gửi: "Giá bao nhiêu?"
```

---

## 🔍 Verify hoạt động

### Check logs
```bash
# Terminal đang chạy sẽ hiển thị:
# [INFO] Received message from Telegram: "Xin chào"
# [INFO] Processing with AI...
# [INFO] Sending response to Telegram
```

### Check API
```bash
curl http://localhost:8080/api/channels/telegram/status
# {"connected": true, "bot_username": "your_bot", "messages_processed": 10}
```

---

## 🎯 Full SME Workflow Test

### Bước 1: Gửi order request
```
Gửi cho bot: "Tôi muốn đặt 2 áo thun size M, màu xanh"
```

### Bước 2: Verify response
Bot nên:
- ✅ Trả lời xác nhận
- ✅ Hỏi thông tin thêm nếu cần
- ✅ Lưu vào memory/database

### Bước 3: Tạo content
```
Gửi: "Tạo bài viết bán hàng từ đơn hàng này"
```

Bot sẽ tạo content marketing!

---

## 🐛 Troubleshooting

### Lỗi: "Bot not found"
- Kiểm tra Bot Token đúng chưa
- Bot chưa được activate bởi user nào

### Lỗi: "Connection timeout"
- Check internet connection
- Restart bot

### Lỗi: "API Error"
```bash
# Verify token
curl -s "https://api.telegram.org/botYOUR_TOKEN/getMe"

# Should return bot info if token is valid
```

---

## ✅ Success Checklist

| Test | Status |
|------|--------|
| Bot nhận tin nhắn | ⏳ |
| Bot trả lời đúng | ⏳ |
| Tiếng Việt hoạt động | ⏳ |
| Memory lưu trữ | ⏳ |
| Content generation | ⏳ |

---

## 🎉 Kết quả mong đợi

```
👤 User: Xin chào, bạn là ai?
🤖 BizClaw Bot: Xin chào! Tôi là trợ lý AI của bạn. Tôi có thể giúp bạn...
```

---

**Anh test và báo kết quả nhé! 🚀**
