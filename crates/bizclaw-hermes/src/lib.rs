//! # BizClaw Hermes Agent
//! 
//! Hermes Agent integration cho BizClaw - Dựa trên NousResearch Hermes 2/3 models:
//! - Tool-calling optimized (Hermes 2 Pro)
//! - Function calling native (Hermes 3)
//! - Vietnamese language support
//! - SME-friendly workflows

pub mod agent;
pub mod tools;
pub mod models;
pub mod chat;

pub use agent::{HermesAgent, AgentConfig, AgentResponse};
pub use tools::{Tool, ToolCall, ToolResult, HermesTools};
pub use models::{HermesModel, ModelConfig};
pub use chat::{ChatSession, Message, MessageRole};
