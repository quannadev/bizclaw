# BizClaw Architecture

This document provides an in-depth overview of BizClaw's architecture, design decisions, and component interactions.

## High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              BizClaw Gateway                                 │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐    ┌─────────────┐ │
│  │   Agent     │    │   Memory    │    │   Hands     │    │   Brain     │ │
│  │  Runtime    │    │   System    │    │   (Tools)   │    │  (LLM)      │ │
│  └──────┬──────┘    └──────┬──────┘    └──────┬──────┘    └──────┬──────┘ │
│         │                   │                   │                   │        │
│         └───────────────────┴───────────────────┴───────────────────┘        │
│                                     │                                        │
│                                     ▼                                        │
│  ┌───────────────────────────────────────────────────────────────────────┐  │
│  │                        Orchestration Layer                            │  │
│  │  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐   │  │
│  │  │Pipeline │  │ Context │  │ Scheduler│  │ Handoff │  │Safety   │   │  │
│  │  └─────────┘  └─────────┘  └─────────┘  └─────────┘  └─────────┘   │  │
│  └───────────────────────────────────────────────────────────────────────┘  │
│                                     │                                        │
└─────────────────────────────────────┼────────────────────────────────────────┘
                                      │
         ┌────────────────────────────┼────────────────────────────┐
         │                            │                            │
         ▼                            ▼                            ▼
┌─────────────────┐       ┌─────────────────┐       ┌─────────────────┐
│    Channels     │       │     Tools       │       │   External      │
│                 │       │                 │       │   Services      │
│ • Telegram      │       │ • Browser       │       │                 │
│ • Discord       │       │ • Shell         │       │ • OpenAI        │
│ • Slack         │       │ • File System   │       │ • Anthropic     │
│ • Zalo          │       │ • Web Search    │       │ • Shopee API    │
│ • WhatsApp      │       │ • Database      │       │ • TikTok API    │
│ • ...           │       │ • Knowledge     │       │ • Facebook API  │
└─────────────────┘       │ • Social Media  │       └─────────────────┘
                          └─────────────────┘
```

## Core Components

### 1. Agent Runtime (`bizclaw-agent`)

The agent runtime is the heart of BizClaw, managing the lifecycle of AI agents.

#### Agent Types

| Type | Description | Lifetime |
|------|-------------|----------|
| **Main** | Primary agent for user interactions | Forever |
| **Named** | User-created agents with custom configs | Permanent |
| **Sub** | Spawned by LLM for complex tasks | Session |
| **Task** | One-shot agents for specific operations | One-shot |

#### Agent Pipeline

```
User Input
    │
    ▼
┌─────────┐
│ Context │ ← Memory, Config, Pre Parsed Commands
└────┬────┘
     │
     ▼
┌─────────┐
│ History │ ← Conversation History, Summaries
└────┬────┘
     │
     ▼
┌─────────┐
│ Prompt  │ ← System Prompt, Templates
└────┬────┘
     │
     ▼
┌─────────┐
│  Think  │ ← Model Reasoning (if enabled)
└────┬────┘
     │
     ▼
┌─────────┐
│   Act   │ ← Tool Calls, Responses
└────┬────┘
     │
     ▼
┌─────────┐
│ Observe │ ← Tool Results, External Data
└────┬────┘
     │
     ▼
┌─────────┐
│ Memory  │ ← Update Context, Store Facts
└────┬────┘
     │
     ▼
┌─────────┐
│Summarize│ ← Compact History (if needed)
└────┬────┘
     │
     ▼
Response
```

### 2. Brain - LLM Integration (`bizclaw-brain`)

The brain component handles LLM inference and model management.

#### Supported Models

| Provider | Models |
|----------|--------|
| OpenAI | GPT-4, GPT-4-Turbo, GPT-3.5-Turbo |
| Anthropic | Claude 3 Opus, Claude 3 Sonnet, Claude 3 Haiku |
| Google | Gemini Pro, Gemini Ultra |
| Local | Llama.cpp GGUF models |
| OpenRouter | 100+ models via unified API |

#### Model Features

- **Streaming**: Real-time token streaming
- **Function Calling**: Structured tool execution
- **Vision**: Image understanding (Claude, GPT-4V)
- **Caching**: Prompt caching for cost optimization
- **Failover**: Automatic fallback on provider errors

### 3. Memory System (`bizclaw-memory`)

BizClaw implements a three-layer memory architecture for optimal performance:

```
┌─────────────────────────────────────────────────────────────────┐
│                        Memory Architecture                       │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  Layer 1: Hot KV (redb)                                          │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │ Fast key-value store for session data                      │ │
│  │ • Sub-millisecond reads                                    │ │
│  │ • ACID transactions                                        │ │
│  │ • Automatic expiration                                     │ │
│  │ Key patterns: session:*, memory:*, cache:*                 │ │
│  └─────────────────────────────────────────────────────────────┘ │
│                              │                                    │
│                              ▼                                    │
│  Layer 2: Full-Text Search (tantivy)                            │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │ BM25 ranking for precise text retrieval                   │ │
│  │ • Fuzzy matching                                           │ │
│  │ • Phrase queries                                           │ │
│  │ • Boolean operators                                        │ │
│  │ Use: Search conversation history, documents               │ │
│  └─────────────────────────────────────────────────────────────┘ │
│                              │                                    │
│                              ▼                                    │
│  Layer 3: Vector Search (HNSW)                                   │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │ Semantic similarity for AI-powered retrieval               │ │
│  │ • Approximate nearest neighbor                             │ │
│  │ • Cosine/Euclidean metrics                                 │ │
│  │ • Incremental indexing                                     │ │
│  │ Use: Find related content, recommendations               │ │
│  └─────────────────────────────────────────────────────────────┘ │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### 4. Hands - Tool System (`bizclaw-hands`)

The hands component provides tools for the agent to interact with the world.

#### Tool Categories

| Category | Tools | Description |
|----------|-------|-------------|
| **Browser** | CDP-based automation | 50+ actions: navigate, click, fill, screenshot, etc. |
| **File System** | read, write, edit, glob, grep | File operations with virtual FS routing |
| **Shell** | exec, bash | Secure shell execution with approval workflow |
| **Web** | search, fetch, download | Web scraping and API calls |
| **Memory** | memory_search, memory_get | 3-tier memory access |
| **Media** | image, audio, video, tts | Content generation and analysis |
| **Social** | facebook, instagram, tiktok | Social media management |
| **E-Commerce** | shopee, tiktok_shop | Store integration |
| **Database** | query, schema, examples | SQL and semantic database queries |
| **Office** | pdf, docx, spreadsheet | Document processing |

#### Tool Safety

Tools implement a deny/confirm/allow security model:

```rust
// Default deny patterns (50+)
const DENY_PATTERNS: &[&str] = &[
    "sudo *",
    "rm -rf /",
    "rm -rf /*",
    ".ssh/*",
    ".env*",
    "*/etc/passwd",
    "chmod 777 *",
    "curl * | sh",
    "wget * | sh",
    // ... more patterns
];

pub enum SafetyAction {
    Deny,    // Block immediately
    Confirm, // Ask user for approval
    Allow,   // Execute without prompting
}
```

### 5. Channels (`bizclaw-channels`)

Channels provide the interface between BizClaw and various messaging platforms.

#### Supported Channels

| Channel | Protocol | Features |
|---------|----------|----------|
| Telegram | HTTP Long Poll | DM, Groups, Media, Voice |
| Discord | Gateway WebSocket | Slash Commands, Threads |
| Slack | Socket Mode | Events, Modal, Workflows |
| Zalo | Official/Personal API | OA, Mini App, ZNS |
| WhatsApp | Cloud API | Templates, Media |
| Facebook | Graph API | Pages, Messenger, Instagram |
| Email | SMTP/IMAP | Send/Receive emails |
| Webhook | HTTP POST | Custom integrations |
| WebSocket | WS | Real-time web clients |

#### Channel Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        Channel Layer                             │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌──────────┐    ┌──────────┐    ┌──────────┐    ┌──────────┐  │
│  │ Telegram │    │ Discord  │    │  Slack   │    │  Zalo    │  │
│  │ Adapter  │    │ Adapter  │    │ Adapter  │    │ Adapter  │  │
│  └────┬─────┘    └────┬─────┘    └────┬─────┘    └────┬─────┘  │
│       │               │               │               │        │
│       └───────────────┴───────────────┴───────────────┘        │
│                               │                                 │
│                               ▼                                 │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │              Channel Abstraction Layer                    │  │
│  │  • Message normalization                                  │  │
│  │  • DM policy (open/pairing/allowlist)                    │  │
│  │  • Rate limiting                                          │  │
│  │  • Retry logic                                            │  │
│  └──────────────────────────────────────────────────────────┘  │
│                               │                                 │
└───────────────────────────────┼─────────────────────────────────┘
                                │
                                ▼
                    ┌───────────────────────┐
                    │   Agent Runtime       │
                    └───────────────────────┘
```

### 6. Orchestration Layer

The orchestration layer coordinates all components for coherent agent behavior.

#### Components

| Component | Purpose |
|-----------|---------|
| **Pipeline** | Orchestrates the agent loop |
| **Context** | Maintains conversation context |
| **Scheduler** | Manages cron jobs and scheduled tasks |
| **Handoff** | Routes between agents |
| **Safety** | Security and approval workflows |

#### Handoff Flow

```
User ─────► Agent A ─────► [Complex Task]
                            │
                            ▼
                     ┌──────────────┐
                     │   Handoff    │
                     │   Manager    │
                     └──────┬───────┘
                            │
              ┌─────────────┼─────────────┐
              ▼             ▼             ▼
         ┌─────────┐  ┌─────────┐  ┌─────────┐
         │ Agent B │  │ Agent C │  │ Agent D │
         │ (Code)  │  │ (Web)   │  │ (Data)  │
         └────┬────┘  └────┬────┘  └────┬────┘
              │             │             │
              └─────────────┴─────────────┘
                            │
                            ▼
                     ┌──────────────┐
                     │   Results    │
                     │   Returned   │
                     └──────────────┘
```

## Data Flow

### Request Lifecycle

```
1. Message Received (Channel)
   │
   ▼
2. Channel Adapter Normalizes Message
   │
   ▼
3. Security Check (DM Policy, Rate Limit)
   │
   ▼
4. Agent Selection (Routing)
   │
   ▼
5. Pre Parsed Commands Check (/help, /new, etc.)
   │
   ▼
6. Context Assembly (Memory + Config)
   │
   ▼
7. LLM Inference (Brain)
   │
   ▼
8. Tool Execution (Hands)
   │
   ▼
9. Response Formatted
   │
   ▼
10. Message Sent (Channel)
```

## Security Model

### Multi-Layer Security

```
┌─────────────────────────────────────────────────────────────────┐
│                        Security Layers                           │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  Layer 1: Network Security                                      │
│  • TLS encryption                                               │
│  • WebSocket over WSS                                           │
│  • Firewall rules                                               │
│                                                                  │
│  Layer 2: Authentication & Authorization                        │
│  • API key authentication                                       │
│  • OAuth for external services                                  │
│  • RBAC (Role-Based Access Control)                            │
│                                                                  │
│  Layer 3: Input Validation                                      │
│  • Prompt injection detection                                   │
│  • XSS prevention                                               │
│  • SQL injection prevention                                     │
│                                                                  │
│  Layer 4: Tool Safety                                           │
│  • Deny/confirm/allow patterns                                  │
│  • Sandbox execution                                            │
│  • Path isolation                                               │
│                                                                  │
│  Layer 5: Output Filtering                                      │
│  • Sensitive data redaction                                     │
│  • Content moderation                                           │
│  • Rate limiting                                                │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Secret Management

Secrets are stored securely using:

- **Environment Variables**: `${VAR}` substitution
- **Vault Integration**: HashiCorp Vault support
- **Cloud KMS**: AWS/GCP/Azure key management
- **Local Encrypted Store**: AES-256-GCM encryption

## Scalability

### Horizontal Scaling

```
                    ┌─────────────────┐
                    │   Load Balancer  │
                    └────────┬────────┘
                             │
         ┌───────────────────┼───────────────────┐
         │                   │                   │
         ▼                   ▼                   ▼
┌─────────────────┐ ┌─────────────────┐ ┌─────────────────┐
│   Gateway 1     │ │   Gateway 2     │ │   Gateway 3     │
│   (Primary)     │ │   (Replica)     │ │   (Replica)     │
└────────┬────────┘ └────────┬────────┘ └────────┬────────┘
         │                   │                   │
         └───────────────────┼───────────────────┘
                             │
                             ▼
                    ┌─────────────────┐
                    │   Shared Store   │
                    │  (Redis/Postgres)│
                    └─────────────────┘
```

### Resource Limits

| Resource | Default | Max |
|----------|---------|-----|
| Memory | 512MB | 4GB |
| CPU | 1 core | 8 cores |
| Concurrent Agents | 10 | 100 |
| Tool Execution Time | 30s | 300s |
| Session History | 100 messages | 1000 messages |

## Observability

### Metrics

Built-in metrics for monitoring:

- `bizclaw_requests_total` - Total requests
- `bizclaw_request_duration_seconds` - Request latency
- `bizclaw_agent_loops_total` - Agent loop counts
- `bizclaw_tool_executions_total` - Tool usage
- `bizclaw_llm_tokens_total` - Token consumption
- `bizclaw_channel_messages_total` - Messages per channel

### Tracing

Distributed tracing with OpenTelemetry:

```bash
# Enable tracing
bizclaw gateway --otel-endpoint http://localhost:4317

# View traces
open http://localhost:16686  # Jaeger UI
```

### Logging

Structured JSON logging:

```json
{
  "timestamp": "2025-01-25T10:30:00Z",
  "level": "info",
  "component": "agent",
  "agent_id": "main",
  "message": "Processing request",
  "trace_id": "abc123"
}
```

## Technology Stack

| Component | Technology |
|-----------|------------|
| Language | Rust 1.85+ |
| Async Runtime | Tokio |
| HTTP Server | Axum |
| WebSocket | tokio-tungstenite |
| Database | SQLite, PostgreSQL |
| Vector Search | Custom HNSW |
| Full-Text Search | Tantivy |
| LLM Inference | Multiple providers |
| Serialization | Serde |
| Logging | Tracing |
| Error Handling | Anyhow, Thiserror |

## Design Principles

1. **Performance First**: Rust ensures minimal latency and memory usage
2. **Security by Default**: Multiple security layers, least privilege
3. **Extensibility**: Plugin system for tools, channels, skills
4. **Resilience**: Failover, retries, circuit breakers
5. **Observability**: Built-in metrics, tracing, logging
6. **Developer Experience**: CLI tools, clear documentation

---

For more details, see:
- [Configuration Reference](CONFIG.md)
- [API Reference](API.md)
- [Deployment Guide](DEPLOYMENT.md)
