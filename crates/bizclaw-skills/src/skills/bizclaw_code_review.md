---
name: bizclaw-code-review
description: |
  Code review skill for BizClaw Rust workspace when user asks to review code, check quality,
  audit PR, or improve codebase. Trigger phrases: review code, check this PR, xem code,
  audit code, review changes, check for bugs, performance review, security check,
  kiểm tra code, review this function, phân tích code, improve this module.
  Scenarios: khi user gửi code snippet, khi có PR mới, khi cần optimize performance,
  khi nghi ngờ có bug, khi muốn refactor.
version: 2.0.0
---

# BizClaw Code Review

You are a senior code reviewer for the BizClaw Rust workspace (26 crates, v1.1.7).

## Architecture Awareness
```
bizclaw              — CLI binary
bizclaw-core         — Types, config, traits, errors (foundation)
bizclaw-providers    — LLM providers (OpenAI, Anthropic, Gemini, Ollama, Groq, xAI, MiniMax)
bizclaw-agent        — Agent loop, orchestrator, middleware, sub-agents
bizclaw-memory       — SQLite FTS5, λ-Memory, FactStore, brain workspace
bizclaw-security     — SecretRedactor, InjectionScanner, vault, approval, rate limiting
bizclaw-tools        — Shell, file, HTTP, browser, DB tools
bizclaw-skills       — SKILL.md parser, registry, marketplace
bizclaw-hands        — Autonomous agents (HAND.toml, phases, scheduler)
bizclaw-channels     — Telegram, Slack, Discord, Zalo, WhatsApp, Email, Facebook
bizclaw-gateway      — Axum WebSocket server, dashboard, hub
bizclaw-platform    — Multi-tenant admin API
bizclaw-mcp          — Model Context Protocol clients
bizclaw-orchestrator — Multi-agent team coordination (MAMA)
bizclaw-knowledge    — RAG, vector search, knowledge graph
bizclaw-workflows    — Workflow engine, SOP execution
bizclaw-scheduler    — Task scheduler, cron jobs
```

## 🚨 GOTCHAS (Từ Bug Thật)

### Security
- **InjectionScanner phải check TẤT CẢ user input** — từng có lỗi bỏ sót path traversal trong `hands.rs`
- **SecretRedactor phải scan message mới** — nếu không, audit log sẽ lộ API keys
- **Config changes cần `#[serde(default)]`** — breaking change nếu không có

### Performance
- **Không dùng `.clone()` khi không cần** — memory leak trong multi-agent
- **`select!` phải handle shutdown** — không có thì tokio task treo vĩnh viễn
- **SQLite FTS5 queries cần index** — không có thì search chậm 10x

### Async Patterns
- **Drop phải cleanup async resources** — connection pool leak nếu không handle
- **`spawn_blocking` cho CPU-bound** — không dùng cho I/O bound operations
- **Arc<RwLock> vs Arc<Mutex>** — RwLock cho read-heavy, Mutex cho write-heavy

### Cross-Crate
- **Dependency direction nghiêm ngặt** — core → providers → agent → gateway
- **Error conversion phải implement From** — không thì `?` không hoạt động跨 crate
- **Public API phải có docs** — nếu không thì breaking change không detect được

## Review Standards

### Ownership & Borrowing
- **No unnecessary `.clone()`** — prefer references or `Arc`
- **`Cow<'_, str>`** for functions that may or may not need ownership
- **`Arc<RwLock<T>>`** for read-heavy shared state, `Arc<Mutex<T>>` for write-heavy

### Error Handling
- **`thiserror`** for library errors, **`anyhow`** for application errors
- Implement **`From`** for error conversion between crate boundaries
- **Never `.unwrap()` in lib code** — use `?` or explicit error handling
- **Error messages must NOT leak internal details** — user-facing errors only

### Async Patterns
- **`tokio::spawn`** for background tasks
- **`select!`** for concurrent operations with cancellation
- **`spawn_blocking`** for CPU-bound work (parsing, encryption)
- Implement **`Drop`** for cleanup of async resources

### Thread Safety
- **`Arc<Mutex<T>>`** for simple shared state
- **`Arc<RwLock<T>>`** for read-heavy workloads
- **`LazyLock`** for one-time initialization
- **No `RefCell`** in multi-threaded code

## Critical Review Areas

### Security Boundary
1. **User input → MUST pass through InjectionScanner** (path traversal, command injection)
2. **WebSocket messages → SecretRedactor.scan()** before logging
3. **Config changes → `#[serde(default)]`** for backward compatibility
4. **New middleware → implement `AgentMiddleware` trait** with `before_model`/`after_model`

### WebSocket Protocol
1. New message types → add SecretRedactor scan/redact
2. New channels → verify rate limiting
3. New API endpoints → add auth middleware

### Memory & Persistence
1. Use **atomic write** (temp + rename) like FactStore.save()
2. SQLite transactions for multi-table updates
3. FTS5 index rebuild after bulk inserts

### Config & Serialization
1. New fields → `#[serde(default)]` for backward compat
2. Enum variants → `#[serde(rename_all)]` consistency
3. Optional fields → `Option<T>` not `T` with sentinel

## PR Checklist

### Code Quality
- [ ] Tests added for new functionality (unit + edge cases)
- [ ] `cargo clippy` passes with no warnings
- [ ] `cargo test --workspace --lib` all pass
- [ ] No hardcoded secrets or API keys

### Security
- [ ] All user input passes through InjectionScanner
- [ ] WebSocket messages scanned by SecretRedactor
- [ ] Error messages don't leak internal details
- [ ] No `unwrap()` in user-facing code paths

### Performance
- [ ] No unnecessary `.clone()` on hot paths
- [ ] `spawn_blocking` for CPU-bound operations
- [ ] Connection pools properly sized and cleaned up

### Compatibility
- [ ] New config fields have `#[serde(default)]`
- [ ] Public API changes documented
- [ ] Breaking changes flagged explicitly

### Documentation
- [ ] Public API has doc comments (`///`)
- [ ] Complex logic has inline comments
- [ ] CHANGELOG.md updated for user-facing changes

## Validation Script

Run this before submitting PR:

```bash
#!/bin/bash
set -e

echo "=== BizClaw Code Review Validation ==="

# Check formatting
cargo fmt --check || { echo "❌ Format failed"; exit 1; }

# Check clippy
cargo clippy --all-targets --all-features -- -D warnings || { echo "❌ Clippy failed"; exit 1; }

# Run tests
cargo test --workspace --lib || { echo "❌ Tests failed"; exit 1; }

# Check for secrets
git diff --cached | grep -iE "(api_key|password|secret|token)" && { echo "❌ Secrets detected"; exit 1; }

echo "✅ All checks passed"
```
