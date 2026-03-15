# 🙏 Ghi Nhận Đóng Góp & Lời Cảm Ơn

BizClaw được xây dựng nhờ rất nhiều dự án mã nguồn mở tuyệt vời.
Chúng tôi ghi nhận và cảm ơn những dự án sau đã truyền cảm hứng, cung cấp kiến thức, hoặc được tích hợp trực tiếp vào BizClaw.

---

## 🔧 Tích Hợp Trực Tiếp

Các dự án có mã nguồn được port, chuyển đổi, hoặc sử dụng làm dependency.
License đầy đủ được ghi lại trong [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).

| Dự án | Tác giả | Sử dụng | License |
|-------|---------|---------|---------|
| [zca-js](https://github.com/RFS-ADRENO/zca-js) | RFS-ADRENO | Giao thức Zalo API cá nhân (xác thực, nhắn tin, mã hóa, bạn bè, nhóm) — port từ JS sang Rust | MIT |
| [llama.cpp](https://github.com/ggerganov/llama.cpp) | Georgi Gerganov | Engine suy luận LLM trên thiết bị qua FFI bindings + Android submodule | MIT |

---

## 💡 Nguồn Cảm Hứng

Các dự án ảnh hưởng đến thiết kế, kiến trúc, hoặc tính năng cụ thể của BizClaw.
Chúng tôi học hỏi từ cách tiếp cận của họ và tự triển khai phiên bản riêng bằng Rust.

### Agent & Điều Phối

| Dự án | Bài học |
|-------|--------|
| **SkyClaw** | Model Router — tự động chọn tier model tối ưu theo độ phức tạp của task |
| **GoClaw** | Phát hiện vòng lặp tool — nhận biết khi agent gọi cùng một tool với tham số tương tự |
| **Paperclip** | Lớp điều phối agent — team agent phân cấp, ngân sách token per-agent, cấu trúc tổ chức |

### Tri Thức & RAG

| Dự án | Bài học |
|-------|--------|
| **OpenRAG** | Kiến trúc tìm kiếm lai (FTS5 + vector), multi-model embedding với dis_max scoring, chunking tài liệu theo heading, hệ thống gợi ý (nudge) |
| **Docling** | Chiến lược chunking tài liệu thông minh tôn trọng ranh giới heading |
| **Claudia** (Eric Blue) | Thiết kế kiến trúc ưu tiên quyền riêng tư, khái niệm "proactive memory" cho nudges, triết lý agent thích nghi theo workflow |
| **Memspan** | Context di động, dựa trên file hoạt động xuyên suốt các công cụ — truyền cảm hứng cho thiết kế Brain workspace |

### Dữ Liệu & Công Cụ

| Dự án | Bài học |
|-------|--------|
| **Datrics Text2SQL** | Pipeline NL-to-SQL 6 bước, "Smart Example Matching" cho query, "Instant Documentation" cho schema indexing, prompt templates phân tích schema |
| **OpenClaw-RL** | "Mọi AI agent production đều thu thập training data... nhưng lại bỏ qua" — Interaction Signal Logger ghi nhận tín hiệu học tập từ hội thoại |

### Platform APIs

| Dịch vụ | Sử dụng |
|---------|---------|
| [Zalo Bot Platform](https://bot.zapps.me/docs/) | API chính thức Zalo OA cho nhắn tin doanh nghiệp |

---

## 🦀 Hệ Sinh Thái Rust

BizClaw được xây dựng với các crate Rust sau (trong số nhiều crate khác):

- **[tokio](https://tokio.rs/)** — Async runtime
- **[axum](https://github.com/tokio-rs/axum)** — Web framework
- **[reqwest](https://github.com/seanmonstar/reqwest)** — HTTP client
- **[serde](https://serde.rs/)** — Serialization / Deserialization
- **[rusqlite](https://github.com/rusqlite/rusqlite)** — SQLite bindings
- **[tokio-tungstenite](https://github.com/snapview/tokio-tungstenite)** — WebSocket
- **[tracing](https://github.com/tokio-rs/tracing)** — Structured logging
- **[lettre](https://github.com/lettre/lettre)** — Gửi email

Danh sách đầy đủ xem tại `Cargo.toml`.

---

## 🙏 Lời Cảm Ơn

Gửi đến tất cả các maintainer mã nguồn mở, nhà nghiên cứu, và cộng đồng chia sẻ thành quả của mình — BizClaw sẽ không tồn tại nếu thiếu các bạn.

Nếu bạn tin rằng dự án của mình nên được ghi nhận ở đây nhưng chưa có, vui lòng [mở issue](https://github.com/nguyenduchoai/bizclaw-cloud/issues) hoặc gửi pull request.

---

*Cập nhật lần cuối: 2026-03-15*
