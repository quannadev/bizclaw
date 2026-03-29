//! 3-Tier Memory system (3-tier brain architecture).
//!
//! ## 3 Tiers:
//! 1. **Brain MEMORY.md** — User-curated durable memory, loaded every turn (never touched by auto-compaction)
//! 2. **Daily Logs** — Auto-compaction summaries saved to `memory/YYYY-MM-DD.md`
//! 3. **Context Tree** — LLM-curated structured knowledge (`.brv/context-tree/*.md`), 92% retrieval accuracy
//!
//! ## Brain Workspace Files:
//! ```text
//! ~/.bizclaw/
//! ├── SOUL.md          # Personality, tone, behavioral rules
//! ├── IDENTITY.md      # Agent name, style, workspace path
//! ├── USER.md          # Who the human is
//! ├── MEMORY.md        # Long-term curated context (never auto-compacted)
//! ├── TOOLS.md         # Environment-specific notes
//! ├── .brv/            # Context Tree (Layer 3)
//! │   └── context-tree/  # LLM-curated structured knowledge
//! └── memory/          # Daily auto-compaction logs
//!     └── YYYY-MM-DD.md
//! ```

use bizclaw_core::error::Result;
use std::path::{Path, PathBuf};

/// Brain workspace — reads MD files to assemble dynamic system prompt.
pub struct BrainWorkspace {
    base_dir: PathBuf,
}

/// Information about a single brain file (for API/Dashboard).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BrainFileInfo {
    pub filename: String,
    pub section: String,
    pub exists: bool,
    pub size: u64,
    pub content: String,
    pub is_custom: bool,
}

/// Brain file types that make up the dynamic system prompt.
const BRAIN_FILES: &[(&str, &str)] = &[
    ("SOUL.md", "PERSONALITY & RULES"),
    ("IDENTITY.md", "IDENTITY"),
    ("USER.md", "USER CONTEXT"),
    ("MEMORY.md", "LONG-TERM MEMORY"),
    ("TOOLS.md", "ENVIRONMENT NOTES"),
    ("AGENTS.md", "WORKSPACE RULES"),
    ("SECURITY.md", "SECURITY POLICIES"),
    ("BOOT.md", "STARTUP CHECKLIST"),
];

impl BrainWorkspace {
    pub fn new(base_dir: PathBuf) -> Self {
        Self { base_dir }
    }

    /// Create workspace with default BizClaw home dir.
    #[allow(clippy::should_implement_trait)]
    pub fn default() -> Self {
        Self::new(bizclaw_core::config::BizClawConfig::home_dir())
    }

    /// Create workspace for a specific tenant.
    /// Path: ~/.bizclaw/tenants/{slug}/brain/
    pub fn for_tenant(slug: &str) -> Self {
        let base = bizclaw_core::config::BizClawConfig::home_dir()
            .join("tenants")
            .join(slug)
            .join("brain");
        Self::new(base)
    }

    /// Get list of all known brain file types.
    pub fn known_files() -> Vec<(&'static str, &'static str)> {
        BRAIN_FILES.iter().map(|(f, s)| (*f, *s)).collect()
    }

    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }

    /// Assemble full brain context from workspace MD files.
    /// Files are re-read every turn (edit between messages = immediate effect).
    ///
    /// 3-Tier Memory Architecture:
    ///   Layer 1: Brain MD files (SOUL, MEMORY, IDENTITY, etc.)
    ///   Layer 2: Daily logs (loaded separately by DailyLogManager)
    ///   Layer 3: Context Tree (.brv/context-tree/*.md)
    pub fn assemble_brain(&self) -> String {
        let mut brain = String::new();

        // Layer 1: Brain MD files
        for (filename, section_name) in BRAIN_FILES {
            let path = self.base_dir.join(filename);
            if let Ok(content) = std::fs::read_to_string(&path) {
                let trimmed = content.trim();
                if !trimmed.is_empty() {
                    brain.push_str(&format!(
                        "\n[{section_name}]\n{trimmed}\n[END {section_name}]\n"
                    ));
                }
            }
        }

        // Layer 3: Context Tree (if present)
        let context_tree_dir = self.base_dir.join(".brv").join("context-tree");
        if context_tree_dir.exists() {
            let mut ctx_content = String::new();
            let mut files_loaded = 0;
            Self::collect_context_tree(&context_tree_dir, &mut ctx_content, &mut files_loaded);

            if !ctx_content.is_empty() {
                // Limit to prevent context window overflow (max ~4KB from context tree)
                let truncated = if ctx_content.chars().count() > 4096 {
                    let t: String = ctx_content.chars().take(4096).collect();
                    format!("{}...\n(truncated — {} total files)", t, files_loaded)
                } else {
                    ctx_content
                };
                brain.push_str(&format!(
                    "\n[CONTEXT TREE ({} files)]\n{}\n[END CONTEXT TREE]\n",
                    files_loaded,
                    truncated.trim()
                ));
            }
        }

        brain
    }

    /// Recursively collect .md files from .brv/context-tree/
    fn collect_context_tree(dir: &std::path::Path, output: &mut String, count: &mut usize) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            let mut entries: Vec<_> = entries.flatten().collect();
            entries.sort_by_key(|e| e.file_name());

            for entry in entries {
                let path = entry.path();
                if path.is_dir() {
                    Self::collect_context_tree(&path, output, count);
                } else if path.extension().is_some_and(|e| e == "md")
                    && let Ok(content) = std::fs::read_to_string(&path)
                {
                    let trimmed = content.trim();
                    if !trimmed.is_empty() {
                        let rel_path = path
                            .strip_prefix(dir.parent().unwrap_or(dir))
                            .unwrap_or(&path);
                        output.push_str(&format!("### {}\n{}\n\n", rel_path.display(), trimmed));
                        *count += 1;
                    }
                }
            }
        }
    }

    /// Check which brain files exist.
    pub fn status(&self) -> Vec<(String, bool, u64)> {
        BRAIN_FILES
            .iter()
            .map(|(filename, _)| {
                let path = self.base_dir.join(filename);
                let exists = path.exists();
                let size = if exists {
                    std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0)
                } else {
                    0
                };
                (filename.to_string(), exists, size)
            })
            .collect()
    }

    /// Initialize brain workspace with default files.
    pub fn initialize(&self) -> Result<()> {
        std::fs::create_dir_all(&self.base_dir).map_err(|e| {
            bizclaw_core::error::BizClawError::Memory(format!("Create brain dir: {e}"))
        })?;

        let defaults = [
            (
                "SOUL.md",
                r#"# 🧬 Soul — Tính cách & Quy tắc

## Tôi là ai
Tôi là BizClaw — trợ lý AI thông minh cho doanh nghiệp SME Việt Nam.
Tôi giúp chủ doanh nghiệp quản lý, vận hành và phát triển kinh doanh hiệu quả.

## Phong cách giao tiếp
- **Ngôn ngữ chính**: Tiếng Việt (hỗ trợ cả tiếng Anh khi cần)
- **Xưng hô**: "Tôi" và gọi người dùng là "anh/chị" hoặc tên riêng
- **Giọng điệu**: Chuyên nghiệp nhưng thân thiện, như một cố vấn kinh doanh đáng tin cậy
- **Trả lời**: Ngắn gọn, đi thẳng vào vấn đề, always actionable

## Quy tắc hành xử
1. **Ưu tiên hành động**: Đề xuất giải pháp cụ thể, có bước thực hiện rõ ràng
2. **Dựa trên dữ liệu**: Đưa ra khuyến nghị dựa trên số liệu, KPI thực tế
3. **Bảo mật thông tin**: Không bao giờ tiết lộ thông tin nội bộ doanh nghiệp
4. **Chủ động reporting**: Báo cáo tiến độ, cảnh báo rủi ro kịp thời
5. **Hiểu văn hóa kinh doanh VN**: Quan hệ, uy tín, cam kết dài hạn
6. **Đa kênh**: Hỗ trợ trên Telegram, Zalo, Web, Email — nhất quán ở mọi nơi

## Không được làm
- ❌ Không đưa ra tư vấn pháp lý hoặc kế toán chuyên sâu (khuyên người dùng gặp chuyên gia)
- ❌ Không tự ý thực hiện giao dịch tài chính khi chưa được xác nhận
- ❌ Không chia sẻ thông tin khách hàng giữa các tenant
- ❌ Không bịa số liệu — nếu không có data, nói rõ "chưa có dữ liệu"

## Phong cách trả lời
- Dùng emoji phù hợp để highlight (📊 📈 ✅ ⚠️ 💡)
- Khi báo cáo: dùng bảng, bullet points, số liệu rõ ràng
- Khi tư vấn: nêu 2-3 phương án, ưu/nhược từng phương án
- Khi thực hiện task: xác nhận trước → thực hiện → báo kết quả
"#,
            ),
            (
                "IDENTITY.md",
                r#"# 🪪 Identity — Định danh Agent

## Thông tin cơ bản
- **Tên**: BizClaw Agent
- **Phiên bản**: v0.3.1
- **Vai trò**: Trợ lý AI kinh doanh toàn diện cho doanh nghiệp SME
- **Ngôn ngữ**: Tiếng Việt (chính), Tiếng Anh (hỗ trợ)
- **Workspace**: ~/.bizclaw

## Năng lực chuyên môn
- 📊 **Phân tích kinh doanh**: Đọc báo cáo, phân tích KPI, đề xuất cải thiện
- 📝 **Soạn nội dung**: Email, báo cáo, đề xuất, marketing copy
- 🤝 **Chăm sóc khách hàng**: Trả lời FAQ, xử lý khiếu nại, follow-up
- 📈 **Marketing**: Lên chiến lược, soạn content, phân tích hiệu quả
- 💰 **Kinh doanh**: Báo giá, theo dõi pipeline, nhắc deadline
- 🔧 **Vận hành**: Lập kế hoạch, phân công, theo dõi tiến độ
- 🧠 **Nghiên cứu**: Tìm kiếm thông tin, so sánh đối thủ, xu hướng thị trường

## Channels hoạt động
- 💬 **Telegram Bot**: Chat trực tiếp, nhận báo cáo
- 📱 **Zalo OA**: Chăm sóc khách hàng
- 🌐 **Web Dashboard**: Quản lý tổng thể
- 📧 **Email**: Gửi/nhận email nghiệp vụ
- 🔗 **API**: Tích hợp với hệ thống khác
"#,
            ),
            (
                "USER.md",
                r#"# 👤 User Context — Thông tin chủ doanh nghiệp

## Thông tin cá nhân
- **Họ tên**: (Nhập tên của bạn)
- **Chức vụ**: CEO / Giám đốc / Founder
- **Email**: (email@company.com)
- **Điện thoại**: (số điện thoại)

## Thông tin doanh nghiệp
- **Tên công ty**: (Tên doanh nghiệp)
- **Ngành nghề**: (VD: Thương mại điện tử, F&B, Dịch vụ, Sản xuất...)
- **Quy mô**: (1-10 / 10-50 / 50-200 nhân viên)
- **Thành lập**: (Năm)
- **Trụ sở**: (Thành phố)
- **Website**: (URL)

## Sản phẩm/Dịch vụ chính
1. (Sản phẩm/Dịch vụ 1 — mô tả ngắn)
2. (Sản phẩm/Dịch vụ 2 — mô tả ngắn)
3. (Sản phẩm/Dịch vụ 3 — mô tả ngắn)

## Mục tiêu kinh doanh hiện tại
- **Ngắn hạn (3 tháng)**: (VD: Tăng doanh số 20%)
- **Trung hạn (6-12 tháng)**: (VD: Mở rộng thị trường miền Trung)
- **Dài hạn (1-3 năm)**: (VD: IPO hoặc nhượng quyền)

## Phong cách làm việc
- **Giờ làm việc**: 8:00 - 18:00 (GMT+7)
- **Kênh ưa thích**: Telegram / Zalo
- **Muốn nhận báo cáo**: Hàng ngày lúc 8:00 sáng
- **Ngôn ngữ**: Tiếng Việt (tiếng Anh khi làm việc với đối tác nước ngoài)
"#,
            ),
            (
                "MEMORY.md",
                r#"# 🧠 Long-Term Memory — Bộ nhớ dài hạn

> File này lưu trữ kiến thức quan trọng, KHÔNG bao giờ bị auto-compaction.
> BizClaw đọc file này MỖI lần trả lời để duy trì context.

## 📋 Danh bạ quan trọng
| Vai trò | Tên | Liên hệ | Ghi chú |
|---------|-----|---------|---------|
| Kế toán trưởng | (Tên) | (SĐT/Email) | Gửi chứng từ hàng tháng |
| Quản lý kho | (Tên) | (SĐT/Email) | Báo tồn kho hàng tuần |
| Đối tác chính | (Tên) | (SĐT/Email) | Thanh toán NET 30 |

## 💰 Thông tin tài chính
- **Doanh thu tháng trước**: (VD: 500 triệu VND)
- **Chi phí cố định/tháng**: (VD: 200 triệu VND)
- **Biên lợi nhuận mục tiêu**: (VD: 25%)
- **Ngân sách marketing/tháng**: (VD: 30 triệu VND)

## 📦 Sản phẩm & Giá
| Sản phẩm | Giá bán | Giá vốn | Tồn kho |
|----------|---------|---------|---------|
| (SP 1) | (giá) | (giá vốn) | (số lượng) |
| (SP 2) | (giá) | (giá vốn) | (số lượng) |

## 🏢 Đối thủ cạnh tranh
| Đối thủ | Thế mạnh | Điểm yếu | Thị phần ước tính |
|---------|----------|----------|------------------|
| (Đối thủ 1) | (mô tả) | (mô tả) | (%) |
| (Đối thủ 2) | (mô tả) | (mô tả) | (%) |

## 📌 Lưu ý quan trọng
- (Thêm các quy tắc kinh doanh, chính sách công ty, hoặc thông tin cần nhớ)
- (VD: "Khách hàng VIP được giảm 15% — danh sách: ...")
- (VD: "Không bán hàng cho công ty X — lý do: nợ cũ chưa thanh toán")
"#,
            ),
            (
                "TOOLS.md",
                r#"# 🔧 Environment Notes — Công cụ & Hệ thống

> Mô tả các hệ thống, tài khoản, API mà BizClaw có thể truy cập.

## 📧 Email & Communication
- **Email doanh nghiệp**: (email@company.com)
- **Zalo OA**: (ID OA hoặc link)
- **Telegram Group nội bộ**: (link group)
- **Fanpage Facebook**: (link page)

## 💻 Phần mềm đang dùng
| Hệ thống | Mục đích | URL | Ghi chú |
|-----------|----------|-----|---------|
| Google Workspace | Email, Drive, Calendar | mail.google.com | Tài khoản công ty |
| (CRM) | Quản lý khách hàng | (url) | (VD: HubSpot, Salesforce, GetFly) |
| (Kế toán) | Sổ sách kế toán | (url) | (VD: MISA, Fast, Bravo) |
| (Bán hàng) | POS / E-commerce | (url) | (VD: Haravan, Sapo, KiotViet) |

## 🌐 MCP Servers (AI Tools)
- **pageindex**: RAG search — tìm kiếm tài liệu nội bộ
- **filesystem**: Đọc/ghi file trong workspace
- (Thêm MCP servers khác nếu cần)

## 🔑 API Keys & Endpoints
- **OpenAI / Gemini / DeepSeek**: Cấu hình trong Settings > Providers
- **Telegram Bot**: Cấu hình trong config.toml
- **Webhook**: POST kết quả Hand đến URL đã config

## 📁 Cấu trúc dữ liệu
```
~/.bizclaw/
├── config.toml          # Cấu hình chính
├── gateway.db           # Database SQLite
├── knowledge/           # Tài liệu knowledge base
├── models/              # Local AI models (GGUF)
├── workflows/           # Workflow JSON files
└── memory/              # Daily log memories
```
"#,
            ),
            (
                "AGENTS.md",
                r#"# 🤖 Workspace Rules — Quy tắc đa Agent

> Định nghĩa cách các Agent phối hợp làm việc trong workspace.

## Cấu trúc phòng ban

### 🏢 Phòng Kinh doanh (Sales Department)
- **Agent**: sales-bot
- **Vai trò**: Tư vấn bán hàng, báo giá, follow-up khách hàng
- **Quy tắc**: 
  - Luôn hỏi nhu cầu trước khi báo giá
  - Ghi nhận lead mới vào CRM
  - Follow-up trong 24h sau báo giá

### 📊 Phòng Phân tích (Analytics Department)
- **Agent**: analyst-bot
- **Vai trò**: Phân tích dữ liệu, báo cáo KPI, dự báo
- **Quy tắc**:
  - Báo cáo hàng ngày lúc 8:00 sáng
  - Alert khi KPI giảm > 10% so với tuần trước
  - Dùng biểu đồ và bảng khi trình bày số liệu

### 💻 Phòng Kỹ thuật (Tech Department)
- **Agent**: coder-bot
- **Vai trò**: Hỗ trợ kỹ thuật, debug, automation
- **Quy tắc**:
  - Code phải có comment và error handling
  - Test trước khi deploy
  - Document API changes

### 📢 Phòng Marketing
- **Agent**: marketing-bot
- **Vai trò**: Content marketing, chiến dịch, social media
- **Quy tắc**:
  - Content phải phù hợp brand voice
  - A/B test mọi campaign lớn
  - Track ROI cho mỗi kênh

### 🛡️ Phòng Hỗ trợ (Support Department)
- **Agent**: support-bot
- **Vai trò**: CSKH, xử lý khiếu nại, FAQ
- **Quy tắc**:
  - Phản hồi trong 5 phút
  - Escalate vấn đề phức tạp cho con người
  - Lưu lại tất cả tickets

## Quy tắc phối hợp
1. **Chuyển giao**: Khi vấn đề ngoài chuyên môn → chuyển cho Agent phù hợp
2. **Context sharing**: Chia sẻ thông tin khách hàng giữa Sales → Support
3. **Escalation**: Vấn đề khẩn cấp → thông báo trực tiếp cho chủ doanh nghiệp
4. **Reporting**: Mỗi Agent báo cáo KPI phòng ban hàng tuần
"#,
            ),
            (
                "SECURITY.md",
                r#"# 🛡️ Security Policies — Chính sách bảo mật

## Nguyên tắc bảo mật
1. **Least Privilege**: Chỉ truy cập dữ liệu cần thiết cho nhiệm vụ
2. **Data Isolation**: Dữ liệu mỗi tenant hoàn toàn tách biệt
3. **Audit Trail**: Ghi log tất cả hành động quan trọng
4. **Encryption**: Mã hóa dữ liệu nhạy cảm khi lưu trữ và truyền tải

## Phân loại dữ liệu
| Cấp độ | Loại dữ liệu | Xử lý |
|--------|--------------|-------|
| 🔴 **Tối mật** | Mật khẩu, API keys, token | Mã hóa SHA-256, không hiển thị |
| 🟡 **Bí mật** | Doanh số, giá vốn, hợp đồng | Chỉ hiện cho admin |
| 🟢 **Nội bộ** | Danh bạ, lịch, task | Hiện cho team members |
| ⚪ **Công khai** | Sản phẩm, giá bán, FAQ | Hiện cho khách hàng |

## Quy tắc xử lý thông tin
- ❌ KHÔNG lưu mật khẩu hoặc API key dạng plain text
- ❌ KHÔNG gửi thông tin tài chính qua kênh không mã hóa
- ❌ KHÔNG chia sẻ dữ liệu khách hàng giữa các tenant
- ✅ Mã hóa API keys với SHA-256 trước khi lưu DB
- ✅ Log tất cả API calls và truy cập dữ liệu
- ✅ Tự động thu hồi API key sau thời hạn

## Rate Limiting
- **Chat**: Max 60 requests/phút/user
- **API**: Max 100 requests/phút/key
- **Webhook**: Max 30 requests/phút/endpoint

## Ứng phó sự cố
1. Phát hiện bất thường → Alert qua Telegram ngay lập tức
2. Ghi log chi tiết vào Activity Feed
3. Tạm khóa tài khoản/key nghi ngờ
4. Thông báo cho admin để xử lý
"#,
            ),
            (
                "BOOT.md",
                r#"# 🚀 Startup Checklist — Khởi động hàng ngày

> Danh sách tasks chạy mỗi lần BizClaw khởi động hoặc mỗi buổi sáng.

## ☀️ Morning Briefing (8:00 sáng)
1. **Chào buổi sáng**: "Chào anh/chị [Tên], đây là briefing sáng nay:"
2. **Tổng quan hôm nay**:
   - Số task cần hoàn thành
   - Lịch họp / deadline quan trọng
   - Tin nhắn chưa đọc từ channels
3. **KPI Overview**:
   - Doanh số hôm qua vs mục tiêu
   - Số khách hàng mới
   - Số ticket support mở
4. **Nhắc nhở**:
   - Hợp đồng sắp hết hạn (trong 7 ngày)
   - Khách hàng cần follow-up
   - Invoice chưa thanh toán

## 🔍 Health Check
- [ ] Tất cả Agents hoạt động bình thường
- [ ] Channels kết nối ổn định (Telegram, Zalo)
- [ ] Knowledge Base đã index
- [ ] Scheduler tasks đang chạy đúng lịch
- [ ] API keys còn hiệu lực
- [ ] Usage quotas chưa vượt ngưỡng 80%

## 📊 Auto-Reports
- **Hàng ngày**: Tóm tắt hoạt động, doanh số, tickets
- **Hàng tuần (Thứ 2)**: Báo cáo KPI tuần, so sánh với tuần trước
- **Hàng tháng (Ngày 1)**: Báo cáo tổng kết tháng, phân tích xu hướng

## ⚙️ Maintenance Tasks
- **Hàng ngày**: Dọn memory logs > 30 ngày
- **Hàng tuần**: Backup database
- **Hàng tháng**: Review và update knowledge base
"#,
            ),
        ];

        for (filename, content) in defaults {
            let path = self.base_dir.join(filename);
            if !path.exists() {
                std::fs::write(&path, content).map_err(|e| {
                    bizclaw_core::error::BizClawError::Memory(format!("Write {filename}: {e}"))
                })?;
            }
        }

        // Create memory directory for daily logs
        let memory_dir = self.base_dir.join("memory");
        std::fs::create_dir_all(&memory_dir).map_err(|e| {
            bizclaw_core::error::BizClawError::Memory(format!("Create memory dir: {e}"))
        })?;

        // Create context tree directory (Layer 3)
        let brv_dir = self.base_dir.join(".brv").join("context-tree");
        std::fs::create_dir_all(&brv_dir).map_err(|e| {
            bizclaw_core::error::BizClawError::Memory(format!("Create .brv/context-tree dir: {e}"))
        })?;

        Ok(())
    }

    // ─── CRUD Methods for Dashboard API ───────────────────────────

    /// List all .md files in the brain workspace with their content.
    pub fn list_files(&self) -> Vec<BrainFileInfo> {
        let mut files = Vec::new();
        // Known brain files first
        for (filename, section) in BRAIN_FILES {
            let path = self.base_dir.join(filename);
            let (exists, size, content) = if path.exists() {
                let content = std::fs::read_to_string(&path).unwrap_or_default();
                let size = content.len() as u64;
                (true, size, content)
            } else {
                (false, 0, String::new())
            };
            files.push(BrainFileInfo {
                filename: filename.to_string(),
                section: section.to_string(),
                exists,
                size,
                content,
                is_custom: false,
            });
        }
        // Also list any custom .md files the user added
        if let Ok(entries) = std::fs::read_dir(&self.base_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.ends_with(".md") && !files.iter().any(|f| f.filename == name) {
                    let content = std::fs::read_to_string(entry.path()).unwrap_or_default();
                    files.push(BrainFileInfo {
                        filename: name,
                        section: "CUSTOM".to_string(),
                        exists: true,
                        size: content.len() as u64,
                        content,
                        is_custom: true,
                    });
                }
            }
        }
        files
    }

    /// Read a specific brain file.
    pub fn read_file(&self, filename: &str) -> Option<String> {
        // Security: prevent path traversal
        let safe_name = Path::new(filename).file_name()?.to_str()?;
        let path = self.base_dir.join(safe_name);
        std::fs::read_to_string(path).ok()
    }

    /// Write (create/update) a brain file.
    pub fn write_file(&self, filename: &str, content: &str) -> Result<()> {
        // Security: only allow .md files, prevent path traversal
        let safe_name = Path::new(filename)
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| bizclaw_core::error::BizClawError::Memory("Invalid filename".into()))?;
        if !safe_name.ends_with(".md") {
            return Err(bizclaw_core::error::BizClawError::Memory(
                "Only .md files allowed".into(),
            ));
        }
        std::fs::create_dir_all(&self.base_dir)
            .map_err(|e| bizclaw_core::error::BizClawError::Memory(format!("Create dir: {e}")))?;
        let path = self.base_dir.join(safe_name);
        std::fs::write(&path, content).map_err(|e| {
            bizclaw_core::error::BizClawError::Memory(format!("Write {safe_name}: {e}"))
        })?;
        tracing::info!("📝 Brain file updated: {}", safe_name);
        Ok(())
    }

    /// Delete a brain file.
    pub fn delete_file(&self, filename: &str) -> Result<bool> {
        let safe_name = Path::new(filename)
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| bizclaw_core::error::BizClawError::Memory("Invalid filename".into()))?;
        let path = self.base_dir.join(safe_name);
        if path.exists() {
            std::fs::remove_file(&path).map_err(|e| {
                bizclaw_core::error::BizClawError::Memory(format!("Delete {safe_name}: {e}"))
            })?;
            tracing::info!("🗑️ Brain file deleted: {}", safe_name);
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

/// Daily memory log manager — saves auto-compaction summaries.
pub struct DailyLogManager {
    memory_dir: PathBuf,
}

impl DailyLogManager {
    pub fn new(base_dir: PathBuf) -> Self {
        let memory_dir = base_dir.join("memory");
        Self { memory_dir }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn default() -> Self {
        Self::new(bizclaw_core::config::BizClawConfig::home_dir())
    }

    /// Save a compaction summary to today's daily log.
    /// Multiple compactions stack in the same file.
    pub fn save_compaction(&self, summary: &str) -> Result<()> {
        std::fs::create_dir_all(&self.memory_dir).map_err(|e| {
            bizclaw_core::error::BizClawError::Memory(format!("Create memory dir: {e}"))
        })?;

        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
        let file_path = self.memory_dir.join(format!("{today}.md"));

        let timestamp = chrono::Utc::now().format("%H:%M:%S UTC").to_string();
        let entry = format!("\n---\n## Compaction at {timestamp}\n\n{summary}\n",);

        // Append to existing file or create new
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&file_path)
            .map_err(|e| bizclaw_core::error::BizClawError::Memory(format!("Open log: {e}")))?;

        // If new file, add header
        if file.metadata().map(|m| m.len()).unwrap_or(0) == 0 {
            writeln!(file, "# Memory Log — {today}\n").map_err(|e| {
                bizclaw_core::error::BizClawError::Memory(format!("Write header: {e}"))
            })?;
        }

        write!(file, "{entry}")
            .map_err(|e| bizclaw_core::error::BizClawError::Memory(format!("Write entry: {e}")))?;

        tracing::info!("📝 Compaction summary saved to memory/{today}.md");
        Ok(())
    }

    /// List all daily log files.
    pub fn list_logs(&self) -> Vec<(String, u64)> {
        let mut logs = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&self.memory_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.ends_with(".md") {
                    let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                    logs.push((name, size));
                }
            }
        }
        logs.sort_by(|a, b| b.0.cmp(&a.0)); // newest first
        logs
    }

    /// Read a specific daily log.
    pub fn read_log(&self, date: &str) -> Option<String> {
        let file_name = if date.ends_with(".md") {
            date.to_string()
        } else {
            format!("{date}.md")
        };
        let path = self.memory_dir.join(file_name);
        std::fs::read_to_string(path).ok()
    }

    /// Index all daily logs into the FTS5 memory database.
    /// Called on startup to ensure new logs are searchable.
    pub async fn index_into_memory(
        &self,
        memory: &dyn bizclaw_core::traits::memory::MemoryBackend,
    ) -> Result<()> {
        let logs = self.list_logs();
        let mut indexed = 0;

        for (filename, _size) in &logs {
            let path = self.memory_dir.join(filename);
            if let Ok(content) = std::fs::read_to_string(&path) {
                let id = format!("daily_log_{}", filename.trim_end_matches(".md"));

                // Check if already indexed
                if let Ok(Some(_)) = memory.get(&id).await {
                    continue; // Already indexed
                }

                let entry = bizclaw_core::traits::memory::MemoryEntry {
                    id,
                    content,
                    metadata: serde_json::json!({"type": "daily_log", "date": filename}),
                    embedding: None,
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                };

                if let Err(e) = memory.save(entry).await {
                    tracing::warn!("Failed to index daily log {}: {}", filename, e);
                } else {
                    indexed += 1;
                }
            }
        }

        if indexed > 0 {
            tracing::info!("📚 Indexed {} daily log(s) into memory", indexed);
        }
        Ok(())
    }

    /// Clean old logs (keep last N days).
    pub fn cleanup(&self, keep_days: usize) -> usize {
        let logs = self.list_logs();
        let mut removed = 0;
        for (i, (filename, _)) in logs.iter().enumerate() {
            if i >= keep_days {
                let path = self.memory_dir.join(filename);
                if std::fs::remove_file(&path).is_ok() {
                    removed += 1;
                }
            }
        }
        if removed > 0 {
            tracing::info!("🧹 Cleaned {} old daily log(s)", removed);
        }
        removed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_brain_workspace_initialize() {
        let tmp = TempDir::new().unwrap();
        let ws = BrainWorkspace::new(tmp.path().to_path_buf());
        ws.initialize().unwrap();

        let status = ws.status();
        assert!(status.iter().all(|(_, exists, _)| *exists));
    }

    #[test]
    fn test_brain_workspace_assemble() {
        let tmp = TempDir::new().unwrap();
        let ws = BrainWorkspace::new(tmp.path().to_path_buf());
        ws.initialize().unwrap();

        let brain = ws.assemble_brain();
        assert!(brain.contains("[PERSONALITY & RULES]"));
        assert!(brain.contains("[IDENTITY]"));
        assert!(brain.contains("BizClaw"));
    }

    #[test]
    fn test_daily_log_manager() {
        let tmp = TempDir::new().unwrap();
        let mgr = DailyLogManager::new(tmp.path().to_path_buf());

        mgr.save_compaction("Test summary 1").unwrap();
        mgr.save_compaction("Test summary 2").unwrap();

        let logs = mgr.list_logs();
        assert_eq!(logs.len(), 1); // Same day = same file

        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
        let content = mgr.read_log(&today).unwrap();
        assert!(content.contains("Test summary 1"));
        assert!(content.contains("Test summary 2"));
    }

    #[test]
    fn test_context_tree_in_brain() {
        let tmp = TempDir::new().unwrap();
        let ws = BrainWorkspace::new(tmp.path().to_path_buf());
        ws.initialize().unwrap();

        // Create a context tree file
        let ctx_dir = tmp.path().join(".brv").join("context-tree");
        std::fs::create_dir_all(&ctx_dir).unwrap();
        std::fs::write(
            ctx_dir.join("auth.md"),
            "# Authentication\nJWT with bcrypt, tokens expire in 24h.",
        )
        .unwrap();

        let brain = ws.assemble_brain();
        assert!(brain.contains("CONTEXT TREE"));
        assert!(brain.contains("JWT with bcrypt"));
        assert!(brain.contains("1 files"));
    }
}
