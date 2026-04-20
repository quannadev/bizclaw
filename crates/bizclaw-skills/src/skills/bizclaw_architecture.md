---
name: bizclaw-architecture
description: |
  Architecture expert for BizClaw platform design and system understanding. Trigger phrases:
  architecture, system design, crate structure, how does it work, component interaction,
  data flow, design pattern, module design, scalable architecture.
  Scenarios: khi cần hiểu kiến trúc, khi cần thiết kế feature mới,
  khi cần tối ưu performance, khi cần refactor.
version: 2.0.0
---

# BizClaw Architecture Expert

You are an architecture expert for the BizClaw AI platform — a Rust-native enterprise agent system.

## System Architecture
```
┌──────────────────────────────────────────────────────┐
│                   bizclaw (binary)                    │
├──────────────────────────────────────────────────────┤
│   bizclaw-gateway (Axum)                             │
│   ├── WebSocket (/ws) — streaming agent interaction  │
│   ├── Dashboard (/dashboard) — embedded Preact UI    │
│   ├── Hub (/hub) — ClawHub skill marketplace        │
│   └── REST API — health, config, metrics            │
├──────────────────────────────────────────────────────┤
│   bizclaw-platform (Admin API)                       │
│   ├── Multi-tenant management                       │
│   ├── JWT authentication                           │
│   └── Agent/team/workflow CRUD                     │
├──────────────────────────────────────────────────────┤
│   bizclaw-agent (Core)                              │
│   ├── Agent loop (process → tool call → respond)    │
│   ├── Middleware pipeline (5 built-in)              │
│   ├── Orchestrator (delegate, handoff, broadcast)   │
│   ├── Sub-agent executor (semaphore-controlled)      │
│   └── Context summarizer + /compact command         │
├──────────────────────────────────────────────────────┤
│   bizclaw-providers          bizclaw-tools          │
│   ├── OpenAI/Anthropic/etc   ├── Shell/File/HTTP    │
│   ├── Failover chain         ├── Database tools      │
│   ├── LLM tracing            └── MCP integration    │
│   └── Text tool call parser                          │
├──────────────────────────────────────────────────────┤
│   bizclaw-memory             bizclaw-security        │
│   ├── SQLite FTS5            ├── SecretRedactor     │
│   ├── λ-Memory (decay)       ├── InjectionScanner   │
│   ├── FactStore (DeerFlow)   ├── CommandAllowlist   │
│   ├── Brain workspace        ├── ApprovalSystem      │
│   └── Structured memory      └── Vault/SecretStore   │
├──────────────────────────────────────────────────────┤
│   bizclaw-hands              bizclaw-channels        │
│   ├── HAND.toml manifest     ├── Telegram/Slack      │
│   ├── Scheduler/Cron         ├── Discord/Zalo        │
│   ├── Phase executor         ├── WhatsApp/Email      │
│   └── Registry (7 built-in)  └── Facebook/Xiaozhi   │
├──────────────────────────────────────────────────────┤
│   bizclaw-core (Foundation)                          │
│   ├── Config (HotConfig mtime-based reload)          │
│   ├── Types (Message, ToolCall, Role)                │
│   ├── Traits (LlmProvider, MemoryBackend, Tool)      │
│   └── Error types (thiserror + anyhow)              │
└──────────────────────────────────────────────────────┘
```

## Dependency Graph

### Core Dependencies (Layered)
```
bizclaw-core       ← All crates depend on this
    ↓
bizclaw-providers  ← Depends on core
    ↓
bizclaw-agent      ← Depends on providers, memory
    ↓
bizclaw-gateway    ← Depends on agent, tools, channels
    ↓
bizclaw            ← Binary, depends on everything
```

### NO Circular Dependencies
- Core has NO dependencies on higher layers
- Providers know about core, not agents
- Agents know about providers, not gateway

## Key Design Patterns

### 1. Trait-Based Abstraction
```rust
// Define trait in core
pub trait LlmProvider: Send + Sync {
    async fn complete(&self, prompt: &str) -> Result<String, LlmError>;
    fn name(&self) -> &str;
}

// Implement in providers
impl LlmProvider for OpenAiProvider { ... }
impl LlmProvider for AnthropicProvider { ... }

// Use in agents
fn chat<P: LlmProvider>(provider: &P) -> Result<String> {
    provider.complete("Hello")
}
```

### 2. Middleware Pipeline
```rust
// Each middleware has priority and transforms data
pub struct MiddlewareChain {
    middlewares: Vec<Box<dyn AgentMiddleware>>,
}

impl AgentMiddleware for SummarizationMiddleware {
    fn priority(&self) -> i32 { 30 }
    async fn before_model(&self, ctx: &mut AgentContext) -> Result<(), Error> {
        // Auto-summarize if context too long
        if ctx.messages.len() > 50 {
            ctx.messages = summarize(&ctx.messages).await?;
        }
        Ok(())
    }
}
```

### 3. Atomic Write (Persistence)
```rust
pub fn save_atomic<T: Serialize>(path: &Path, data: &T) -> Result<()> {
    let temp = path.with_extension("tmp");
    let json = serde_json::to_string_pretty(data)?;
    std::fs::write(&temp, json)?;
    temp.rename(path)?; // Atomic on POSIX
    Ok(())
}
```

### 4. Builder Pattern (Configuration)
```rust
let agent = Agent::builder()
    .name("my-agent")
    .model("gpt-4")
    .provider(OpenAiProvider::new())
    .memory(FactStore::new())
    .tools(vec![shell_tool(), file_tool()])
    .middleware(SummarizationMiddleware::new(50))
    .build()
    .await?;
```

## Data Flow

### WebSocket Message Flow
```
1. Client connects with JWT token
2. Token validated, session created
3. Message received → SecretRedactor.scan() [detect secrets]
4. → InjectionScanner.check() [detect injection]
5. → Agent.process() [core logic]
6. → Tools executed if needed
7. → SecretRedactor.redact() [remove leaked secrets]
8. → Response sent to client
```

### Memory Flow
```
User Message
    ↓
Extract Facts (AI judgment)
    ↓
λ-Memory Decay (exponential)
    ↓
FactStore (persistent, searchable)
    ↓
Recall on Context (query similar facts)
    ↓
Inject into Prompt
```

## Performance Considerations

### Connection Pooling
```rust
// SQLite: Limited connections (only 1 writer)
let pool = SqlitePoolOptions::new()
    .max_connections(5)
    .acquire_timeout(Duration::from_secs(3));

// HTTP: Reuse connections
let client = reqwest::Client::builder()
    .pool_max_idle_per_host(10)
    .build()?;
```

### Caching
```rust
// HotConfig: mtime-based cache invalidation
static CONFIG: LazyLock<HotConfig> = LazyLock::new(|| {
    HotConfig::new("config.toml")
});

// LLM responses: Cache repeated prompts
let cache = LruCache::new(1000);
```

### Async Concurrency
```rust
// Spawn blocking for CPU-bound
let result = tokio::task::spawn_blocking(|| {
    cpu_intensive_work()
}).await?;

// Semaphore for bounded concurrency
let sem = Arc::new(Semaphore::new(5));
let _permit = sem.acquire().await?;
```

## Extension Points

### Add New LLM Provider
1. Create `bizclaw-provider-xxx` crate
2. Implement `LlmProvider` trait
3. Add to `providers.rs` match
4. Register in `BrainConfig`

### Add New Channel
1. Implement `Channel` trait
2. Add to `bizclaw-channels/src/`
3. Register in channel registry
4. Add WebSocket message handlers

### Add New Tool
1. Implement `Tool` trait
2. Add to `bizclaw-tools/src/`
3. Register in tool registry
4. Document in SKILL.md

## Gotchas

### 1. Mutex vs RwLock
```rust
// Read-heavy: RwLock
let guard = state.read().await; // Multiple readers OK

// Write-heavy or simple: Mutex
let guard = state.lock().await; // One writer
```

### 2. Clone Costs
```rust
// ❌ Bad: Clone in hot path
for msg in messages.clone() { ... }

// ✅ Good: Reference when possible
for msg in &messages { ... }
```

### 3. Error Propagation
```rust
// Use ? for clean error propagation
async fn fetch_data() -> Result<Data, AppError> {
    let config = load_config()?; // Converts to AppError
    let data = query_db(&config).await?;
    Ok(data)
}
```
