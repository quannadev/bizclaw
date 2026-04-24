//! Permission Hooks - Inspired by OpenHarness security patterns
//!
//! Features:
//! - Path-level permissions (sensitive files)
//! - PreTool/PostTool approval dialogs
//! - Command validation
//! - Dangerous command detection

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Permission mode
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub enum PermissionMode {
    /// Auto-deny dangerous commands
    #[default]
    Strict,
    /// Ask before execution
    Interactive,
    /// Auto-approve safe commands
    Permissive,
}

/// Tool call approval request
#[derive(Debug, Clone)]
pub struct ApprovalRequest {
    pub tool_name: String,
    pub params: serde_json::Value,
    pub reason: String,
    pub risk_level: RiskLevel,
    pub session_id: String,
}

/// Risk assessment
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum RiskLevel {
    Safe,
    Medium,
    High,
    Critical,
}

impl PermissionHook {
    pub fn new(mode: PermissionMode) -> Self {
        Self {
            mode,
            dangerous_patterns: DangerousPatterns::default(),
            approval_queue: std::sync::Arc::new(tokio::sync::Mutex::new(Vec::new())),
        }
    }

    pub fn check_tool(&self, tool_name: &str, params: &serde_json::Value) -> CheckResult {
        // Check dangerous patterns
        let risk = self.dangerous_patterns.check(tool_name, params);
        
        match risk {
            RiskLevel::Critical => CheckResult {
                approved: false,
                risk_level: risk,
                message: Some("Critical risk tool call blocked".to_string()),
            },
            RiskLevel::High => {
                match self.mode {
                    PermissionMode::Strict | PermissionMode::Interactive => CheckResult {
                        approved: false,
                        risk_level: risk,
                        message: Some("High-risk tool requires approval".to_string()),
                    },
                    PermissionMode::Permissive => CheckResult {
                        approved: true,
                        risk_level: risk,
                        message: Some("Approved with warning".to_string()),
                    }
                }
            }
            _ => CheckResult {
                approved: true,
                risk_level: risk,
                message: None,
            }
        }
    }
}

/// Permission check result
pub struct CheckResult {
    pub approved: bool,
    pub risk_level: RiskLevel,
    pub message: Option<String>,
}

pub struct DangerousPatterns {
    patterns: Vec<Pattern>,
}

impl Default for DangerousPatterns {
    fn default() -> Self {
        Self::new()
    }
}

impl DangerousPatterns {
    pub fn new() -> Self {
        let patterns = vec![
            // File operations
            Pattern::new("rm.*-rf.*", RiskLevel::Critical, "Recursive delete detected"),
            Pattern::new("chmod.*777", RiskLevel::High, "World-writable permission"),
            Pattern::new("curl.*\\|.*sh", RiskLevel::Critical, "Pipe to shell detected"),
            // Network operations
            Pattern::new("curl.*(wget|curl).*", RiskLevel::Medium, "Network download detected"),
            // Environment manipulation
            Pattern::new("export.*PATH", RiskLevel::Medium, "PATH modification"),
            // Database
            Pattern::new("DROP.*TABLE", RiskLevel::Critical, "Dangerous SQL operation"),
            Pattern::new("DELETE.*WHERE.*true", RiskLevel::High, "DELETE without filter"),
            // Process
            Pattern::new("kill.*-9", RiskLevel::High, "Force kill detected"),
            Pattern::new("pkill|fkill", RiskLevel::Medium, "Process termination"),
            // File access
            Pattern::new("/etc/passwd", RiskLevel::Medium, "System file access"),
            Pattern::new("\\.ssh/|\\.aws/", RiskLevel::High, "Sensitive directory access"),
            // Eval/exec
            Pattern::new("eval\\(|exec\\(", RiskLevel::Critical, "Dynamic code execution"),
        ];
        Self { patterns }
    }
}

struct Pattern {
    regex: regex::Regex,
    level: RiskLevel,
    description: String,
}

impl Pattern {
    fn new(pattern: &str, level: RiskLevel, description: &str) -> Self {
        Self {
            regex: regex::Regex::new(pattern).unwrap_or_default(),
            level,
            description: description.to_string(),
        }
    }
}

/// Permission hook trait
pub trait PermissionHook: Send + Sync {
    fn check(&self, tool: &str, params: &serde_json::Value) -> CheckResult;
}

impl DangerousPatterns {
    fn check(&self, tool: &str, params: &serde_json::Value) -> RiskLevel {
        let mut max_level = RiskLevel::Safe;
        
        for pattern in &self.patterns {
            if pattern.regex.is_match(tool) {
                max_level = max_level.higher(&pattern.level);
            }
            
            let params_str = serde_json::to_string(params).unwrap_or_default();
            if pattern.regex.is_match(&params_str) {
                max_level = RiskLevel::Critical;
            }
        }
        
        max_level
    }
}

impl RiskLevel {
    fn higher(&self, other: &RiskLevel) -> RiskLevel {
        match (self, other) {
            (RiskLevel::Critical, _) | (_, RiskLevel::Critical) => RiskLevel::Critical,
            (RiskLevel::High, _) | (_, RiskLevel::High) => RiskLevel::High,
            (RiskLevel::Medium, _) | (_, RiskLevel::Medium) => RiskLevel::Medium,
            _ => RiskLevel::Safe,
        }
    }
}

pub struct PermissionHook {
    pub tool: String,
    pub level: RiskLevel,
}

pub struct PermissionGuard {
    hooks: Vec<Box<dyn PermissionHook>,
}

impl PermissionGuard {
    pub fn new() -> Self {
        Self { hooks: Vec::new() }
    }

    pub fn add_hook(&mut self, hook: impl PermissionHook + 'static) {
        self.hooks.push(Box::new(hook));
    }

    pub fn check_all(&self, tool: &str, params: &serde_json::Value) -> CheckResult {
        let mut result = CheckResult { approved: true, risk_level: RiskLevel::Safe, message: None };
        
        for hook in &self.hooks {
            let r = hook.check(tool, params);
            if !r.approved {
                return r;
            }
            if matches!(r.risk_level, RiskLevel::High | RiskLevel::Critical) {
                result = r;
            }
        }
        result
    }
}
