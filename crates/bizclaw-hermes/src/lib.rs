//! # BizClaw Hermes Agent
//! 
//! Hermes Agent integration cho BizClaw:
//! - Tool-calling optimized (Hermes 2 Pro)
//! - Skills integration (Khazix)
//! - SME-friendly workflows

pub mod agent;
pub mod tools;
pub mod models;
pub mod chat;
pub mod hermes_skills;

pub use agent::{HermesAgent, AgentConfig, AgentResponse};
pub use tools::{Tool, ToolCall, ToolResult, HermesTools};
pub use models::{HermesModel, ModelConfig};
pub use chat::{ChatSession, Message, MessageRole};
pub use hermes_skills::HermesAgentWithSkills;
