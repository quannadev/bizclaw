//! # BizClaw WebAuth
//!
//! Browser-based AI provider proxy for BizClaw.
//!
//! Uses web chat sessions (Gemini, Claude, ChatGPT, DeepSeek, Grok) to provide
//! LLM access without API keys. Exposes an OpenAI-compatible HTTP proxy at
//! `http://127.0.0.1:{PORT}/v1/chat/completions`.
//!
//! ## Architecture
//!
//! ```text
//! BizClaw Agent ──→ WebAuth Proxy (localhost HTTP)
//!                       │
//!                       ├─→ Gemini Web Provider (CDP + Batchexecute)
//!                       ├─→ Claude Web Provider (streaming API)
//!                       ├─→ ChatGPT Web Provider (conversation API)
//!                       ├─→ DeepSeek Web Provider (SSE chat)
//!                       └─→ Grok Web Provider (X.com API)
//!                               │
//!                               ▼
//!                       CDP WebSocket → Chrome/Chromium
//! ```
//!
//! ## Usage
//!
//! ```rust,no_run
//! use bizclaw_webauth::WebAuthPipeline;
//!
//! #[tokio::main]
//! async fn main() {
//!     let mut pipeline = WebAuthPipeline::new();
//!     let port = pipeline.start(0).await.unwrap();
//!     println!("WebAuth proxy on port {}", port);
//!     // Configure BizClaw provider: custom:http://127.0.0.1:{port}/v1
//! }
//! ```

pub mod cdp;
pub mod cookie_auth;
pub mod pipeline;
pub mod proxy;
pub mod providers;
pub mod types;

pub use pipeline::WebAuthPipeline;
pub use proxy::WebAuthProxy;
pub use types::*;
