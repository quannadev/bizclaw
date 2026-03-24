//! # BizClaw Core
//!
//! Core traits, types, and configuration for the BizClaw AI assistant platform.
//! Every subsystem is a trait — swap implementations with a config change.

pub mod circuit_breaker;
pub mod config;
pub mod error;
pub mod session_store;
pub mod traits;
pub mod types;
pub mod utils;

pub use circuit_breaker::CircuitBreaker;
pub use config::BizClawConfig;
pub use error::{BizClawError, Result};
pub use utils::safe_truncate;
