# 📋 Nhật Ký Thay Đổi (Changelog)

## [1.1.6] — 2026-04-04 🚀

### Thêm mới (Cloud SaaS & Multi-Agent)
- **🤖 Open-Multi-Agent Orchestrator**: Kiến trúc DAG Layer thay thế hệ thống Agent Group cũ (`coordinator.rs`, `task_dag.rs`). Định tuyến task mượt mà hơn và hỗ trợ Human-in-the-loop tự nhiên.
- **☁️ Auto-Provision VPS qua SePay/Pay2S**: Bồi đắp thêm tính năng nhận dạng `cloud_tenant` trong vòng lặp Webhook. Khi mua plan VPS, Platform tự động `clone` và `boot` máy ảo trên vSphere mà không cần can thiệp tay.
- **📱 Android PicoLMEngine Safeguards**: Tích hợp Sliding Window Context và Memory Guards (tránh OOM) cho tác vụ suy luận Gemma 4 trên thiết bị. Cập nhật JNI binding `BizClawLLM`.
- **🛠️ Signed Release APK**: NDK build rules fix tĩnh cho Android Release Build (1.1.6).
- **🖥️ Landing Page PaaS**: Tích hợp module báo giá Multi-tier (Solo, Growth, Business, Enterprise) và SaaS SMEs SEO templates.

## [1.0.0] — 2026-03-10 🎉 Bản Chính Thức

### 🚀 Cột Mốc: Phiên Bản Ổn Định Đầu Tiên

BizClaw chính thức **production-ready** — nền tảng AI Agent edge hoàn chỉnh nhất
với 18 LLM providers, 10 kênh giao tiếp, 13 công cụ, điều phối multi-agent, cùng
các tính năng độc quyền (Android Agent, Brain Engine, Xiaozhi Voice, Desktop App).

### Thêm mới
- **🔌 3 Provider mới**: Cohere (Command R+), Perplexity (Sonar Pro), DashScope/Qwen (Qwen-Max)
  - Tổng: **18 providers** — vượt GoClaw (13) và ZeroClaw (12)
  - Alias: `qwen`→dashscope, `alibaba`→dashscope, `pplx`→perplexity, `co`→cohere
- **🧠 Extended Thinking**: Chế độ suy luận sâu cho task phức tạp
  - Anthropic: `thinking.type=enabled` với `budget_tokens` tuỳ chỉnh
  - OpenAI-compatible: `reasoning_effort` (low/medium/high)
  - Config: `extended_thinking = true`, `thinking_budget_tokens = 10000`
- **🧊 Prompt Caching Metrics**: Theo dõi realtime hiệu suất cache của Anthropic
  - `cache_creation_input_tokens`, `cache_read_input_tokens` trong Usage
  - Tự động ghi log: "🧊 Prompt cache: X read, Y created (saved ~Z%)"
  - Tracking `thinking_tokens` cho chế độ extended thinking
- **📊 Orchestration Types** (định nghĩa đầy đủ, v1.1 sẽ kết nối vào runtime):
  - `AgentLink` — permission links với chiều Outbound/Inbound/Bidirectional
  - `Delegation` — uỷ quyền task đồng bộ/bất đồng bộ giữa agents
  - `AgentTeam` — vai trò Lead/Member với task board và mailbox
  - `Handoff` — chuyển quyền điều khiển hội thoại giữa agents
  - `EvaluateConfig` — vòng lặp phản hồi generator-evaluator
  - `QualityGate` — cổng chất lượng Command/Agent với block_on_failure
  - `LaneConfig` — làn thực thi Main/Subagent/Delegate/Cron

### Cải thiện
- **Parse Usage helper** — DRY refactoring loại bỏ 3 khối code phân tích usage trùng lặp
- **Prompt caching** đã hỗ trợ sẵn cho Anthropic (cache_control trên system blocks)
- **Toàn bộ 403+ tests pass** trên 19 workspace crates

### Bảng Provider

| # | Provider | Models |
|---|----------|--------|
| 1 | OpenAI | GPT-4o, GPT-4o Mini |
| 2 | OpenRouter | GPT-4o, Claude Sonnet 4, Step 3.5 Flash |
| 3 | Anthropic | Claude Sonnet 4, 3.5 Haiku, 3.5 Sonnet |
| 4 | DeepSeek | Chat, Reasoner (R1) |
| 5 | Gemini | 2.5 Pro, 2.5 Flash |
| 6 | Groq | Llama 3.3 70B, 3.1 8B, Mixtral |
| 7 | Ollama | Jan-nano, Qwen3, Llama 3.2, Gemma 3, Phi-4, DeepSeek R1 |
| 8 | llama.cpp | Local model |
| 9 | CLIProxy | Custom model |
| 10 | vLLM | Default model |
| 11 | Together AI | Llama 3.3 70B |
| 12 | Mistral | Large, Small |
| 13 | MiniMax | Text 01 |
| 14 | xAI/Grok | Grok 3, Grok 3 Mini |
| 15 | ModelArk | Seed 2.0, Doubao, DeepSeek V3, GLM 4.7 |
| 16 | **Cohere** (MỚI) | Command R+, Command R |
| 17 | **Perplexity** (MỚI) | Sonar Pro, Sonar |
| 18 | **DashScope** (MỚI) | Qwen Max, Plus, Turbo |

Thêm: `brain` (GGUF on-device) và `custom:` endpoints

## [0.3.2] — 2026-03-08

### Thêm mới
- **🖥️ Desktop App**: Binary `bizclaw-desktop` — 1 file 13MB duy nhất
  - Tự mở browser đến dashboard khi khởi động
  - Tự chọn port ngẫu nhiên (hoặc `--port N`)
  - Dữ liệu lưu tại `~/.bizclaw/` (đa nền tảng)
  - Không cần cấu hình — chạy là dùng được ngay
  - CLI: `--port`, `--no-open`, `--help`
- **🔑 JWT SSO**: Gateway chấp nhận JWT token từ Platform để xác thực liền mạch
  - 3 phương thức: `Authorization: Bearer`, Cookie `bizclaw_token`, URL `?token=`
  - Dùng chung biến `JWT_SECRET` giữa Platform và Gateway
  - `JwtClaims` struct: sub, email, role, tenant_id, exp
  - Hàm `validate_jwt()` sử dụng HS256
  - Endpoint `verify-pairing` chấp nhận cả `{token:JWT}` và `{code:PIN}`
- **🔨 GitHub Actions CI/CD**: `.github/workflows/release-desktop.yml`
  - macOS Apple Silicon (.dmg) — ~20MB
  - macOS Intel (.dmg) — cross-compiled
  - Windows x64 (.zip) — ~15MB
  - Linux x64 (.deb) — ~26MB
  - Tự tạo GitHub Release khi push tag

### Thay đổi
- **Pairing code**: `require_pairing` mặc định `false` (JWT là xác thực chính)
- **Dashboard frontend**: Auth helpers hỗ trợ cả JWT và pairing code cũ
  - `getJwtToken()`: trích xuất từ URL `?token=`, cookie, hoặc sessionStorage
  - `authFetch()`: gửi Bearer token hoặc header X-Pairing-Code
  - WebSocket: truyền JWT qua query param `?token=`
- **README**: 5 phương thức triển khai (Desktop, Source, Docker, Cloud, PaaS)
  - Bảng link tải cho macOS/Windows/Linux
  - Tài liệu tính năng mới: Desktop App, Cloud Platform, JWT SSO
- **Tài liệu kiến trúc**: Cập nhật v0.3.2 với sơ đồ triển khai 3 chế độ

### Bảo mật
- Xác thực JWT sử dụng so sánh constant-time
- Bảo vệ chống brute-force cho nỗ lực đăng nhập
- Xoá token khi nhận phản hồi 401 (clear sessionStorage)

## [0.3.2-pre] — 2026-03-06

### Thêm mới
- **Dropdown Selects**: Chọn Provider/Model bằng `<select>` dropdown từ `/api/v1/providers`
  - SettingsPage: Dropdown Provider + Model tự động populate models theo provider đã chọn
  - AgentsPage: Dropdown Provider + Model trong form tạo/sửa agent
  - OrchestrationPage: Dropdown From/To Agent từ `/api/v1/agents`
  - Tất cả select có option "✏️ Nhập thủ công..." cho giá trị tuỳ chỉnh
- **Multi-instance Channels**: ChannelsPage hỗ trợ nhiều instance per channel type
  - Nút "➕ Thêm kênh" với dropdown chọn loại kênh
  - Đặt tên per-instance (VD: "Bot bán hàng", "Zalo cá nhân 2")
  - Hỗ trợ: Telegram, Zalo, Discord, Email, Webhook (multi:true)
  - Trường tên hiển thị trong form cấu hình kênh
- **🐕 Script Watchdog**: `scripts/watchdog.sh` — Tự động kill process treo
  - `--status`: Xem các process BizClaw đang chạy
  - `--kill-all`: Kill khẩn cấp tất cả
  - `--daemon`: Chạy nền giám sát (kiểm tra mỗi 60s)
  - `--dry-run`: Xem trước sẽ kill gì
  - Tuỳ chỉnh: biến môi trường `WATCHDOG_MAX_MINUTES=20`
- **Workflow `/watchdog`**: Slash command quản lý process nhanh

### Thay đổi
- **Shell timeout**: 30s → 900s (15 phút) — tránh ngắt command sớm
  - `crates/bizclaw-tools/src/shell.rs`: Tuỳ chỉnh qua tham số `timeout_secs` hoặc biến `BIZCLAW_SHELL_TIMEOUT_SECS`
  - `crates/bizclaw-runtime/src/lib.rs`: NativeRuntime + SandboxedRuntime đều nâng cấp
  - Hỗ trợ timeout per-call: tối đa 3600s (1 giờ)
- **Thông báo timeout cải thiện**: Hiển thị cả phút và giây
- **4 file sửa đổi**: app.js (+344 dòng), shell.rs, lib.rs (runtime), Cargo.lock

## [0.3.1] — 2026-03-06

### Sửa lỗi
- **NGHIÊM TRỌNG**: Lỗi Preact dual-instance — click navigation giờ hoạt động trên TẤT CẢ trang
  - Nguyên nhân: `hooks.mjs` import `options` từ module `preact.mjs` riêng, tạo ra 2 instance Preact
  - State setters (`useState`) đăng ký với instance B trong khi `render()` dùng instance A → thay đổi state không trigger re-render
  - Fix: Thay 3 file vendor riêng bằng `htm/preact/standalone.module.js` (1 file duy nhất, không import ngoài)
- **Dashboard data**: Uptime, Version, OS, Arch giờ hiển thị dữ liệu thật từ `/api/v1/info` (trước đó hiện "—")
- **Skills Market**: Load 10 skills từ API thay vì hiện "Total Skills: 0"
- **Trang Settings**: Không còn treo "Loading..." vô hạn (thêm timeout 8s + xử lý lỗi)
- **Theme Light/Dark**: Nút chuyển theme hoạt động ổn định (state cập nhật đúng với single Preact instance)

### Thay đổi
- Nâng version: 0.3.0 → 0.3.1
- Vendor bundle: 3 file (preact.mjs + hooks.mjs + htm.mjs) → 1 file (standalone.mjs, 13KB)
- DashboardPage: fetch `/api/v1/info` khi mount để lấy thông tin hệ thống (uptime_secs, version, platform)
- `dashboard.rs`: nhúng `standalone.mjs` vào static file registry

### Chi tiết kỹ thuật
- 20+ trang dashboard navigate đúng khi click sidebar
- WebSocket: trạng thái 🟢 Connected duy trì qua các lần chuyển trang
- Nút ngôn ngữ (VI/EN) và theme (Light/Dark) hoạt động trên mọi trang

## [0.3.0] — 2026-03-05

### Thêm mới
- **Workflow Rules Engine**: 6 loại trigger → 4 loại action, visual builder trong dashboard
- **Vector RAG**: Tìm kiếm lai (FTS5 keyword + Vector cosine similarity) cho knowledge base
- **Scheduler++**: Task cron, interval, one-time với thông báo qua Telegram/Email/Webhook
- **Script build APK Android**: `android/build-apk.sh` (debug/release/clean)
- **Tích hợp InjectionScanner**: Phát hiện prompt injection hoạt động trong pipeline agent
- **Bảo mật ShellTool**: Chặn metacharacter, phát hiện pattern nguy hiểm, env_clear, timeout
- **Bảo mật FileTool**: Kiểm tra path, phát hiện traversal, bảo vệ ghi file nhạy cảm
- **Bảo mật ExecuteCodeTool**: Scanner pattern code nguy hiểm (16 patterns)
- **AES-256-CBC**: Thay thế ECB bằng mã hoá CBC cho secrets (IV ngẫu nhiên mỗi lần mã hoá)

### Thay đổi
- Nâng version: 0.2.0 → 0.3.0
- Số tests: 144 → 342 tests pass
- Security headers: Runtime sandbox, HMAC-SHA256 key derivation
- Gateway: toàn bộ std::sync::Mutex .lock().unwrap() → .unwrap_or_else() chống crash khi poison
- Agent: SecurityPolicy giờ kiểm tra cả shell VÀ file tools (trước đây chỉ shell)
- README cập nhật tính năng Workflow Rules, Scheduler, Vector RAG

### Sửa lỗi
- **NGHIÊM TRỌNG**: Nạp config tenant — truyền flag CLI `--config` + fallback biến `BIZCLAW_CONFIG`
- **NGHIÊM TRỌNG**: Docker networking — tenants bind `0.0.0.0` để port forwarding hoạt động
- **NGHIÊM TRỌNG**: CORS allow-all trong production → giới hạn 5 domain whitelist
- **NGHIÊM TRỌNG**: JWT secret giờ lưu cố định qua biến môi trường (trước đây random mỗi lần restart)
- Xử lý lỗi SchedulerDb open()

### Bảo mật
- AES-256-ECB → AES-256-CBC (IV ngẫu nhiên, HMAC-SHA256 key derivation)
- ShellTool: bảo vệ đa tầng (tool-level + agent-level validation)
- FileTool: chặn path cấm, phát hiện path traversal, bảo vệ ghi
- ExecuteCodeTool: scanner pattern nguy hiểm
- InjectionScanner: chèn guardrail vào LLM context khi phát hiện prompt khả nghi
- Mutex poisoning: sửa 27 instance trên toàn gateway
- CORS: whitelist domain chỉ cho production
- JWT: secret ngẫu nhiên cố định

## [0.2.0] — 2026-03-01

### Thêm mới
- Phiên bản đầu tiên với 19 crates
- 16 LLM providers, 9 kênh giao tiếp, 13 công cụ
- Brain Engine (suy luận GGUF + SIMD)
- Knowledge RAG (FTS5)
- Nền tảng admin multi-tenant
- Web Dashboard (20+ trang)
- Lớp Android FFI
