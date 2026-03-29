//! Approval Gates — human-in-the-loop for sensitive tool actions.
//!
//! Enterprise requirement: certain tools (email, http_request, shell)
//! can be configured to require explicit approval before execution.
//!
//! # How it works:
//! 1. Agent wants to call a tool (e.g., `shell` with `rm` command)
//! 2. ApprovalGate checks if tool requires approval
//! 3. If yes → action queued as "pending", user notified
//! 4. User approves/denies via dashboard or chat command
//! 5. Agent receives result and continues
//!
//! # Config:
//! ```toml
//! [autonomy]
//! approval_required_tools = ["shell", "http_request", "email"]
//! auto_approve_timeout_secs = 300  # auto-deny after 5 min
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, warn};

/// Unique ID for a pending approval.
pub type ApprovalId = String;

/// Status of an approval request.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ApprovalStatus {
    Pending,
    Approved,
    Denied,
    Expired,
}

impl std::fmt::Display for ApprovalStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Approved => write!(f, "approved"),
            Self::Denied => write!(f, "denied"),
            Self::Expired => write!(f, "expired"),
        }
    }
}

/// A pending action awaiting approval.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingAction {
    pub id: ApprovalId,
    pub tool_name: String,
    pub arguments_summary: String,
    pub session_id: String,
    pub status: ApprovalStatus,
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Who made the decision (if any).
    pub decided_by: Option<String>,
    /// When the decision was made.
    pub decided_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Configuration for approval gates.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalConfig {
    /// Tools that require approval before execution.
    #[serde(default)]
    pub approval_required_tools: Vec<String>,
    /// Auto-deny timeout in seconds (0 = never auto-deny).
    #[serde(default = "default_timeout")]
    pub auto_approve_timeout_secs: u64,
}

fn default_timeout() -> u64 {
    300 // 5 minutes
}

impl Default for ApprovalConfig {
    fn default() -> Self {
        Self {
            approval_required_tools: vec![],
            auto_approve_timeout_secs: default_timeout(),
        }
    }
}

/// Thread-safe approval gate manager.
#[derive(Clone)]
pub struct ApprovalGate {
    config: ApprovalConfig,
    pending: Arc<Mutex<HashMap<ApprovalId, PendingAction>>>,
}

impl ApprovalGate {
    /// Create a new approval gate with configuration.
    pub fn new(config: ApprovalConfig) -> Self {
        Self {
            config,
            pending: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Check if a tool requires approval.
    pub fn requires_approval(&self, tool_name: &str) -> bool {
        self.config
            .approval_required_tools
            .iter()
            .any(|t| t == tool_name)
    }

    /// Submit an action for approval. Returns the approval ID.
    pub async fn submit(&self, tool_name: &str, arguments: &str, session_id: &str) -> ApprovalId {
        let id = uuid::Uuid::new_v4().to_string();

        // Summarize arguments (truncate for safety — never expose full secrets)
        let summary = if arguments.len() > 200 {
            let truncated: String = arguments.chars().take(200).collect();
            format!("{}...", truncated)
        } else {
            arguments.to_string()
        };

        let action = PendingAction {
            id: id.clone(),
            tool_name: tool_name.to_string(),
            arguments_summary: summary,
            session_id: session_id.to_string(),
            status: ApprovalStatus::Pending,
            created_at: chrono::Utc::now(),
            decided_by: None,
            decided_at: None,
        };

        info!(
            "⏳ Approval required: [{}] {} → {}",
            id,
            tool_name,
            &action.arguments_summary[..action.arguments_summary.len().min(80)]
        );

        let mut pending = self.pending.lock().await;
        pending.insert(id.clone(), action);
        id
    }

    /// Approve a pending action.
    pub async fn approve(&self, id: &str, by: &str) -> Option<PendingAction> {
        let mut pending = self.pending.lock().await;
        if let Some(action) = pending.get_mut(id) {
            if action.status != ApprovalStatus::Pending {
                return None;
            }
            action.status = ApprovalStatus::Approved;
            action.decided_by = Some(by.to_string());
            action.decided_at = Some(chrono::Utc::now());
            info!("✅ Approved: [{}] {} by {}", id, action.tool_name, by);
            Some(action.clone())
        } else {
            None
        }
    }

    /// Deny a pending action.
    pub async fn deny(&self, id: &str, by: &str) -> Option<PendingAction> {
        let mut pending = self.pending.lock().await;
        if let Some(action) = pending.get_mut(id) {
            if action.status != ApprovalStatus::Pending {
                return None;
            }
            action.status = ApprovalStatus::Denied;
            action.decided_by = Some(by.to_string());
            action.decided_at = Some(chrono::Utc::now());
            warn!("❌ Denied: [{}] {} by {}", id, action.tool_name, by);
            Some(action.clone())
        } else {
            None
        }
    }

    /// Get status of a pending action.
    pub async fn status(&self, id: &str) -> Option<PendingAction> {
        let pending = self.pending.lock().await;
        pending.get(id).cloned()
    }

    /// List all pending actions (for dashboard/admin).
    pub async fn list_pending(&self) -> Vec<PendingAction> {
        let pending = self.pending.lock().await;
        pending
            .values()
            .filter(|a| a.status == ApprovalStatus::Pending)
            .cloned()
            .collect()
    }

    /// Expire old pending actions that exceeded timeout.
    pub async fn expire_old(&self) -> usize {
        if self.config.auto_approve_timeout_secs == 0 {
            return 0;
        }

        let mut pending = self.pending.lock().await;
        let now = chrono::Utc::now();
        let timeout = chrono::Duration::seconds(self.config.auto_approve_timeout_secs as i64);
        let mut expired_count = 0;

        for action in pending.values_mut() {
            if action.status == ApprovalStatus::Pending
                && now.signed_duration_since(action.created_at) > timeout
            {
                action.status = ApprovalStatus::Expired;
                action.decided_by = Some("system:timeout".to_string());
                action.decided_at = Some(now);
                warn!(
                    "⏰ Expired: [{}] {} ({}s timeout)",
                    action.id, action.tool_name, self.config.auto_approve_timeout_secs
                );
                expired_count += 1;
            }
        }

        expired_count
    }

    /// Clean up old completed/expired actions (keep last 100).
    pub async fn cleanup(&self) {
        let mut pending = self.pending.lock().await;
        if pending.len() > 100 {
            let mut entries: Vec<_> = pending.drain().collect();
            entries.sort_by_key(|(_, a)| a.created_at);
            let keep = entries.split_off(entries.len().saturating_sub(100));
            *pending = keep.into_iter().collect();
        }
    }
}

impl Default for ApprovalGate {
    fn default() -> Self {
        Self::new(ApprovalConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_approval_flow() {
        let config = ApprovalConfig {
            approval_required_tools: vec!["shell".into(), "http_request".into()],
            auto_approve_timeout_secs: 60,
        };
        let gate = ApprovalGate::new(config);

        // Check requires_approval
        assert!(gate.requires_approval("shell"));
        assert!(gate.requires_approval("http_request"));
        assert!(!gate.requires_approval("file"));
        assert!(!gate.requires_approval("web_search"));

        // Submit action
        let id = gate
            .submit("shell", r#"{"command":"ls -la"}"#, "session-1")
            .await;
        assert!(!id.is_empty());

        // Check pending
        let pending = gate.list_pending().await;
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].tool_name, "shell");

        // Approve
        let approved = gate.approve(&id, "admin@test.com").await;
        assert!(approved.is_some());
        assert_eq!(approved.unwrap().status, ApprovalStatus::Approved);

        // Verify no more pending
        assert!(gate.list_pending().await.is_empty());
    }

    #[tokio::test]
    async fn test_deny_flow() {
        let gate = ApprovalGate::new(ApprovalConfig {
            approval_required_tools: vec!["shell".into()],
            auto_approve_timeout_secs: 60,
        });

        let id = gate.submit("shell", r#"{"command":"rm -rf"}"#, "s1").await;
        let denied = gate.deny(&id, "security@test.com").await;
        assert!(denied.is_some());
        assert_eq!(denied.unwrap().status, ApprovalStatus::Denied);
    }

    #[tokio::test]
    async fn test_double_approve_rejected() {
        let gate = ApprovalGate::new(ApprovalConfig {
            approval_required_tools: vec!["shell".into()],
            auto_approve_timeout_secs: 60,
        });

        let id = gate.submit("shell", r#"{"command":"ls"}"#, "s1").await;
        gate.approve(&id, "admin").await;
        // Second approve should fail (already decided)
        let second = gate.approve(&id, "another_admin").await;
        assert!(second.is_none());
    }

    #[tokio::test]
    async fn test_argument_truncation() {
        let gate = ApprovalGate::default();
        let long_args = "x".repeat(500);
        let id = gate.submit("shell", &long_args, "s1").await;
        let action = gate.status(&id).await.unwrap();
        assert!(action.arguments_summary.len() <= 203); // 200 + "..."
    }
}
