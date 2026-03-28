# BizClaw Feature Review

You are a product engineer reviewing BizClaw features for completeness and quality.

## Platform Features (v1.0.5)

### Core Agent Engine
- [x] Multi-provider LLM support (OpenAI, Anthropic, Gemini, DeepSeek, Groq, Ollama, Brain)
- [x] Middleware pipeline (5 built-in: Guardrail, Summarization, Memory, DanglingToolCall, SubagentLimit)
- [x] Context auto-compaction + manual `/compact` command
- [x] Agent-to-Agent communication (`agent_send`, `agent_ask`, `delegate`, `handoff`)
- [x] Sub-agent executor with semaphore-based concurrency control
- [x] Quality Gate evaluator loop for response improvement

### Memory System
- [x] SQLite FTS5 full-text search
- [x] λ-Memory exponential decay with configurable half-life
- [x] Fact-Based Memory (DeerFlow-inspired) with confidence scoring
- [x] Brain workspace management (3-tier: brief → extended → deep)
- [x] Config hot-reload (mtime-based auto-detection)

### Security
- [x] SecretRedactor in WebSocket pipeline (scan incoming, redact outgoing)
- [x] InjectionScanner for prompt injection/jailbreak/exfiltration
- [x] CommandAllowlist with shell injection blocking
- [x] catch_unwind panic resilience in WS handler
- [x] AES-256-CBC encrypted secret store
- [x] Human-in-the-loop approval system

### Autonomous Hands
- [x] HAND.toml manifest with OpenFang marketplace compatibility
- [x] Multi-phase execution (gather → analyze → report)
- [x] Cron + interval + manual scheduling
- [x] 7 built-in hands (Research, Analytics, Content, Monitor, Sync, Outreach, Security)
- [x] Tool/dashboard/author metadata for marketplace

### Channels (7+)
- [x] Telegram, Slack, Discord, Email, WhatsApp, Zalo, Xiaozhi
- [x] Multi-instance per channel type
- [x] Rate limiting per channel
- [x] Webhook generic channel

### Tools
- [x] Shell, File, HTTP Request, Browser, API Connector
- [x] Database tools (schema, query, semantic, safety checks)
- [x] Social posting, plan store, bundle provisioner
- [x] MCP server integration
- [x] Custom tool registration

### Platform (Enterprise)
- [x] Multi-tenant admin API
- [x] JWT authentication
- [x] ClawHub marketplace (/hub)
- [x] Dashboard with real-time WebSocket streaming

## Feature Completeness Check
When reviewing a new feature, verify:
1. **Functionality**: Does it work end-to-end? Test happy path + error cases
2. **Security**: Is user input sanitized? Are secrets protected?
3. **UX**: Is the WebSocket protocol documented? Are error messages helpful?
4. **Performance**: Does it handle concurrent requests? Is there a timeout?
5. **Backward compat**: Does it break existing config files or API contracts?
6. **Testability**: Are there unit tests? Can it be tested without external services?

## Feature Gap Analysis
- [ ] File upload with auto-conversion (PDF/PPT → text) — DeerFlow has this
- [ ] LangSmith-style observability/tracing — OpenTelemetry integration
- [ ] Docker sandbox for code execution — currently native only
- [ ] Progressive skill loading (lazy, not all at init)
- [ ] Title auto-generation middleware after first exchange
- [ ] Debounced memory queue (batch LLM fact extraction calls)
