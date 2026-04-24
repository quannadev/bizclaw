//! CubeSandbox Integration for BizClaw
//!
//! Provides safe code execution for AI agents using CubeSandbox or E2B-compatible API.
//! 
//! Features:
//! - Blazing fast cold start (<60ms)
//! - VM-level isolation (safer than Docker)
//! - Memory efficient (<5MB per instance)
//! - E2B SDK compatible (easy migration)

use serde::{Deserialize, Serialize};
use std::time::Duration;
use thiserror::Error;

pub mod executor;
pub mod config;

pub use executor::*;
pub use config::*;

/// Programming languages supported by sandbox
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    Python,
    Javascript,
    Typescript,
    Rust,
    Go,
    Java,
    Bash,
    #[default]
    Unknown,
}

impl Language {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "python" | "py" | "python3" => Language::Python,
            "javascript" | "js" | "nodejs" => Language::Javascript,
            "typescript" | "ts" => Language::Typescript,
            "rust" | "rs" => Language::Rust,
            "go" | "golang" => Language::Go,
            "java" => Language::Java,
            "bash" | "shell" | "sh" => Language::Bash,
            _ => Language::Unknown,
        }
    }
    
    pub fn as_str(&self) -> &'static str {
        match self {
            Language::Python => "python",
            Language::Javascript => "javascript",
            Language::Typescript => "typescript",
            Language::Rust => "rust",
            Language::Go => "go",
            Language::Java => "java",
            Language::Bash => "bash",
            Language::Unknown => "unknown",
        }
    }
}

/// Execution result from sandbox
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub id: String,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub runtime_ms: u64,
    pub memory_mb: Option<f32>,
    pub network_enabled: bool,
    pub success: bool,
}

/// Sandbox instance info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxInstance {
    pub id: String,
    pub created_at: i64,
    pub language: Language,
    pub hostname: String,
    pub network_id: Option<String>,
}

/// Sandbox errors
#[derive(Error, Debug)]
pub enum SandboxError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    
    #[error("Timeout after {0:?}")]
    Timeout(Duration),
    
    #[error("Execution failed: {0}")]
    ExecutionFailed(String),
    
    #[error("Sandbox limit exceeded: {0}")]
    LimitExceeded(String),
    
    #[error("Invalid request: {0}")]
    InvalidRequest(String),
    
    #[error("Sandbox not available: {0}")]
    NotAvailable(String),
}

/// Fix result with auto-retry loop
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixResult {
    pub original_code: String,
    pub fixed_code: String,
    pub attempts: u32,
    pub final_result: Option<ExecutionResult>,
    pub error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_parsing() {
        assert_eq!(Language::from_str("python"), Language::Python);
        assert_eq!(Language::from_str("js"), Language::Javascript);
        assert_eq!(Language::from_str("rust"), Language::Rust);
    }

    #[test]
    fn test_execution_result_success() {
        let result = ExecutionResult {
            id: "test-123".to_string(),
            stdout: "Hello, World!".to_string(),
            stderr: String::new(),
            exit_code: 0,
            runtime_ms: 45,
            memory_mb: Some(2.5),
            network_enabled: false,
            success: true,
        };
        assert!(result.success);
        assert_eq!(result.stdout, "Hello, World!");
    }
}
