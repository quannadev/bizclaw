# ADR-0003: Tokio Async Runtime

**Status**: Accepted  
**Date**: 2024-02-01  
**Deciders**: BizClaw Team

## Context

BizClaw handles many concurrent operations:
- WebSocket connections (CDP browser automation)
- HTTP requests (API calls, webhooks)
- Database connections
- Background task scheduling
- Real-time messaging

We needed an async runtime that:
- Handles thousands of concurrent connections
- Provides reliable timer/interval functionality
- Integrates well with async ecosystem
- Has excellent debugging tools

## Decision

**Use Tokio as the async runtime with full features enabled.**

```rust
[dependencies]
tokio = { version = "1", features = ["full"] }
```

## Rationale

### Benefits

1. **Battle-Tested**: Powers production systems at scale (Discord, AWS, etc.)
2. **Feature-Rich**: Built-in channels, sync primitives, timers
3. **Runtime Agnostic**: Works with any async/await code
4. **Tooling**: Excellent debugging with `tokio-console`
5. **Performance**: Work-stealing scheduler optimized for diverse workloads

### Trade-offs

- Runtime overhead vs bare-metal async
- Larger binary size with `features = ["full"]`
- `#[tokio::main]` macro adds compile time

## Consequences

### Positive

- Single unified async model across all crates
- Reliable async I/O for networking
- Easy-to-use channels for inter-task communication
- Built-in tracing integration

### Negative

- Compile time increased by runtime features
- Must understand async concepts for contributors

## Usage Patterns

### Multi-threaded Runtime (Default)

```rust
#[tokio::main(flavor = "multi_thread")]
async fn main() {
    // Spawns worker threads automatically
}
```

### Single-threaded (Testing)

```rust
#[tokio::test]
async fn my_test() {
    // Faster for unit tests
}
```

### Background Tasks

```rust
tokio::spawn(async move {
    // Runs in background
});

tokio::task::spawn_blocking(|| {
    // CPU-bound work
}).await?;
```

## Alternatives Considered

| Runtime | Pros | Cons |
|---------|------|------|
| async-std | Similar API to std | Smaller ecosystem |
| smol | Lightweight | Less battle-tested |
| runtime-tokio | Rust official | Only async runtime |
