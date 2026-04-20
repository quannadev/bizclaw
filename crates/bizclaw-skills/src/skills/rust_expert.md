---
name: rust-expert
description: |
  Rust programming expert for BizClaw when writing Rust code, fixing compiler errors,
  or optimizing performance. Trigger phrases: write Rust, fix Rust error, this won't compile,
  cargo build failed, how to use tokio, async Rust, implement trait, lifetime error,
  borrow checker, Arc Mutex, write a test, benchmarking, optimization, idiomatic Rust.
  Scenarios: khi cần viết code Rust mới, khi có compiler error, khi cần optimize performance,
  khi muốn implement async pattern, khi cần viết test.
version: 2.0.0
---

# Rust Expert

You are an expert Rust programmer for the BizClaw codebase.

## Ownership & Borrowing

### Core Principles
- **Prefer borrowing (`&T`, `&mut T`)** over cloning when possible
- Use **`Cow<'_, str>`** for functions that may or may not need ownership
- **Avoid `.clone()` on hot paths** — creates memory pressure

### When to Clone
```rust
// ✅ Good: Clone when crossing thread boundary
let data = Arc::new(data);
let handle = tokio::spawn(move || process(data));

// ✅ Good: Clone for async callbacks that outlive scope
let config = config.clone();
async move { config.get() }.await;

// ❌ Bad: Clone in tight loop
for item in items {
    let owned = item.clone(); // Unnecessary allocation
}
```

### Arc vs Arc\<Mutex\> vs Arc\<RwLock\>

```rust
// Read-heavy: Arc<RwLock<T>>
let data = Arc::new(RwLock::new(data));
let read = data.read().await; // Multiple readers OK

// Write-heavy: Arc<Mutex<T>>
let data = Arc::new(Mutex::new(data));
let mut write = data.lock().await; // Exclusive access

// Single owner + async: Arc<T>
let data = Arc::new(Data::new());
tokio::spawn(move || data.method());
```

## Error Handling

### Library vs Application
```rust
// Library: use thiserror
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("missing required field: {field}")]
    MissingField { field: String },
    #[error("invalid value for {field}: {value}")]
    InvalidValue { field: String, value: String },
}

// Application: use anyhow
use anyhow::{Context, Result};

fn load_config() -> Result<Config> {
    let content = std::fs::read_to_string("config.toml")
        .context("Failed to read config file")?;
    // ...
}
```

### Never unwrap() in Library Code
```rust
// ❌ Bad
fn get_value(&self) -> &str {
    self.values.first().unwrap() // Panics if empty
}

// ✅ Good
fn get_value(&self) -> Option<&str> {
    self.values.first()
}

// ✅ Good with default
fn get_value(&self) -> &str {
    self.values.first().unwrap_or(&DEFAULT_VALUE)
}
```

### Cross-Crate Error Conversion
```rust
// In bizclaw-core/src/error.rs
#[derive(Error, Debug)]
pub enum CoreError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

// In bizclaw-gateway/src/error.rs
#[derive(Error, Debug)]
pub enum GatewayError {
    #[error("Core error: {0}")]
    Core(#[from] bizclaw_core::error::CoreError),
}
```

## Async Patterns

### Tokio Runtime
```rust
#[tokio::main]
async fn main() -> Result<()> {
    // Use Builder for custom runtime
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_name("bizclaw-worker")
        .build()?;

    runtime.block_on(async {
        run().await
    })
}
```

### Spawning Tasks
```rust
// ✅ Spawn for fire-and-forget background work
tokio::spawn(async move {
    if let Err(e) = process_background().await {
        tracing::error!("Background task failed: {}", e);
    }
});

// ✅ Spawn blocking for CPU-bound work
let handle = tokio::task::spawn_blocking(|| {
    cpu_intensive_computation()
}).await?;

// ✅ Select for cancellation
tokio::select! {
    result = process() => result?,
    _ = shutdown_signal => {
        tracing::info!("Shutting down gracefully");
        return Ok(());
    }
}
```

### Drop for Cleanup
```rust
pub struct DatabasePool {
    conn: Connection,
}

impl Drop for DatabasePool {
    fn drop(&mut self) {
        // Clean up resources
        self.conn.close();
    }
}
```

## Performance

### Benchmarking
```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn fibonacci(n: u64) -> u64 {
    match n {
        0 => 0,
        1 => 1,
        _ => fibonacci(n - 1) + fibonacci(n - 2),
    }
}

fn bench_fib(c: &mut Criterion) {
    c.bench_function("fib_20", |b| {
        b.iter(|| fibonacci(black_box(20)))
    });
}
```

### SIMD (when justified)
```rust
// Only use after profiling shows it's needed
#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

unsafe {
    _mm256_loadu_si256(data.as_ptr() as *const _);
}
```

### Stack vs Heap
```rust
// ✅ Prefer stack for small fixed-size data
fn process(data: &[u8; 64]) { }

// ✅ Use Box for heap allocation when needed
let data = Box::new(large_struct);

// ✅ Use Vec for dynamic collections
let mut items = Vec::with_capacity(100);
```

## Testing

### Unit Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_validation() {
        let config = Config::parse("valid=true");
        assert!(config.is_ok());
    }

    #[test]
    fn test_error_propagation() {
        let result = parse_config("invalid{");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ConfigError::Parse(_)));
    }
}
```

### Property-Based Testing
```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_config_roundtrip(s in "\\PC*") {
        let config = Config::parse(&s);
        if let Ok(c) = config {
            let serialized = c.to_string();
            let reparsed = Config::parse(&serialized);
            prop_assert!(reparsed.is_ok());
        }
    }
}
```

### Integration Tests
```rust
// tests/integration.rs
#[tokio::test]
async fn test_agent_workflow() {
    let agent = Agent::new().await.unwrap();
    let response = agent.process("hello").await.unwrap();
    assert!(!response.is_empty());
}
```

## Code Quality

### Formatting (cargo fmt)
```toml
# .rustfmt.toml
edition = "2021"
max_width = 100
tab_spaces = 4
newline_style = "Unix"
```

### Clippy Rules
```rust
# Allow specific lints
#[allow(clippy::unnecessary_wraps)]
fn optional_warning() -> Option<i32> { Some(1) }
```

### Documentation
```rust
/// Configuration for the agent.
///
/// # Examples
/// ```
/// use bizclaw_core::config::AgentConfig;
///
/// let config = AgentConfig::default();
/// ```
///
/// # Panics
/// Panics if required fields are missing.
pub struct AgentConfig {
    /// The name of the agent.
    pub name: String,
}
```

## Gotchas

### 1. LazyLock vs OnceLock
```rust
// LazyLock: initializes once, lazily
static CONFIG: LazyLock<Config> = LazyLock::new(|| Config::load());

// OnceLock: may fail to initialize
static CONFIG: OnceLock<Config> = OnceLock::new();

fn get_config() -> &'static Config {
    CONFIG.get_or_init(|| Config::load())
}
```

### 2. Mutex in async code
```rust
// ❌ Bad: blocking lock in async
let data = mutex.lock().unwrap(); // Blocks thread pool!

// ✅ Good: async lock
let data = mutex.lock().await;
```

### 3. Send + Sync
```rust
// Ensure your types are Send + Sync if needed
unsafe impl Send for MyType {}
unsafe impl Sync for MyType {}

// Common issue: Rc is not Send
let rc = Rc::new(Data); // Not Send!
// Use Arc instead
let arc = Arc::new(Data); // Is Send
```

### 4. tokio::select! bias
```rust
// select! runs first branch if ready, biased by order
tokio::select! {
    _ = future1 => {}, // This runs first if ready
    _ = future2 => {},
}
// Use biased(false) for random selection
tokio::select! {
    biased;
    _ = future1 => {},
    _ = future2 => {},
}
```

### 5. Pin and Box\<dyn Future\>
```rust
// Pin is needed for !Unpin types
let future = Box::pin(async { /* ... */ });
// futures are !Unpin by default
```
