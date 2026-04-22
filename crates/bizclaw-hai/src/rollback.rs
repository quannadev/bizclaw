//! Action rollback and undo functionality

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionSnapshot {
    pub id: String,
    pub action_type: ActionType,
    pub description: String,
    pub state_before: HashMap<String, serde_json::Value>,
    pub state_after: HashMap<String, serde_json::Value>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub can_undo: bool,
    pub rollback_data: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionType {
    Create,
    Update,
    Delete,
    Send,
    Approve,
    Reject,
    ExternalCall,
    FileOperation,
    DatabaseWrite,
    EmailSent,
    Payment,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackResult {
    pub success: bool,
    pub snapshot_id: String,
    pub restored_state: Option<HashMap<String, serde_json::Value>>,
    pub error: Option<String>,
    pub cascading_effects: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackPolicy {
    pub max_undo_depth: usize,
    pub allowed_types: Vec<ActionType>,
    pub time_limit_seconds: u64,
    pub require_confirmation: bool,
    pub auto_rollback_on_error: bool,
}

impl Default for RollbackPolicy {
    fn default() -> Self {
        Self {
            max_undo_depth: 10,
            allowed_types: vec![
                ActionType::Create,
                ActionType::Update,
                ActionType::Delete,
                ActionType::Send,
            ],
            time_limit_seconds: 3600,
            require_confirmation: true,
            auto_rollback_on_error: true,
        }
    }
}

pub struct RollbackManager {
    snapshots: Vec<ActionSnapshot>,
    policy: RollbackPolicy,
}

impl Default for RollbackManager {
    fn default() -> Self {
        Self::new()
    }
}

impl RollbackManager {
    pub fn new() -> Self {
        Self {
            snapshots: Vec::new(),
            policy: RollbackPolicy::default(),
        }
    }

    pub fn with_policy(mut self, policy: RollbackPolicy) -> Self {
        self.policy = policy;
        self
    }

    pub fn snapshot(&mut self, snapshot: ActionSnapshot) -> bool {
        if !self.policy.allowed_types.contains(&snapshot.action_type) {
            tracing::warn!(
                "Action type {:?} not allowed for snapshot",
                snapshot.action_type
            );
            return false;
        }

        if (chrono::Utc::now() - snapshot.timestamp).num_seconds() > self.policy.time_limit_seconds as i64 {
            tracing::warn!("Snapshot too old, cannot rollback");
            return false;
        }

        if self.snapshots.len() >= self.policy.max_undo_depth {
            self.snapshots.remove(0);
        }

        self.snapshots.push(snapshot);
        true
    }

    pub fn snapshot_action(
        &mut self,
        action_type: ActionType,
        description: &str,
        state_before: HashMap<String, serde_json::Value>,
        state_after: HashMap<String, serde_json::Value>,
        rollback_data: Option<serde_json::Value>,
    ) -> String {
        let id = uuid::Uuid::new_v4().to_string();
        
        let can_undo = self.policy.allowed_types.contains(&action_type)
            && rollback_data.is_some();

        let snapshot = ActionSnapshot {
            id: id.clone(),
            action_type,
            description: description.to_string(),
            state_before,
            state_after,
            timestamp: chrono::Utc::now(),
            can_undo,
            rollback_data,
        };

        self.snapshot(snapshot);

        id
    }

    pub fn undo(&mut self) -> Option<RollbackResult> {
        self.undo_n(1).into_iter().next()
    }

    pub fn undo_n(&mut self, count: usize) -> Vec<RollbackResult> {
        let mut results = Vec::new();

        for _ in 0..count {
            if let Some(snapshot) = self.snapshots.pop() {
                if !snapshot.can_undo {
                    results.push(RollbackResult {
                        success: false,
                        snapshot_id: snapshot.id,
                        restored_state: None,
                        error: Some("Action cannot be undone".to_string()),
                        cascading_effects: Vec::new(),
                    });
                    continue;
                }

                results.push(RollbackResult {
                    success: true,
                    snapshot_id: snapshot.id.clone(),
                    restored_state: Some(snapshot.state_before.clone()),
                    error: None,
                    cascading_effects: Vec::new(),
                });

                tracing::info!(
                    "Rolled back action {}: {}",
                    snapshot.id,
                    snapshot.description
                );
            }
        }

        results
    }

    pub fn undo_to(&mut self, snapshot_id: &str) -> Vec<RollbackResult> {
        let mut results = Vec::new();

        while let Some(snapshot) = self.snapshots.pop() {
            results.push(RollbackResult {
                success: true,
                snapshot_id: snapshot.id.clone(),
                restored_state: Some(snapshot.state_before.clone()),
                error: None,
                cascading_effects: Vec::new(),
            });

            if snapshot.id == snapshot_id {
                break;
            }
        }

        results
    }

    pub fn get_history(&self) -> Vec<&ActionSnapshot> {
        self.snapshots.iter().rev().collect()
    }

    pub fn get_last_snapshot(&self) -> Option<&ActionSnapshot> {
        self.snapshots.last()
    }

    pub fn can_undo(&self) -> bool {
        self.snapshots.iter().any(|s| s.can_undo)
    }

    pub fn undo_depth(&self) -> usize {
        self.snapshots.iter().filter(|s| s.can_undo).count()
    }

    pub fn clear(&mut self) {
        self.snapshots.clear();
    }
}

pub mod helpers {
    use super::*;

    pub fn create_state_map(items: Vec<(&str, serde_json::Value)>) -> HashMap<String, serde_json::Value> {
        items.into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect()
    }

    pub fn describe_action(action_type: ActionType, target: &str) -> String {
        match action_type {
            ActionType::Create => format!("Created {}", target),
            ActionType::Update => format!("Updated {}", target),
            ActionType::Delete => format!("Deleted {}", target),
            ActionType::Send => format!("Sent {}", target),
            ActionType::Approve => format!("Approved {}", target),
            ActionType::Reject => format!("Rejected {}", target),
            ActionType::ExternalCall => format!("Called external API: {}", target),
            ActionType::FileOperation => format!("Modified file: {}", target),
            ActionType::DatabaseWrite => format!("Wrote to database: {}", target),
            ActionType::EmailSent => format!("Sent email: {}", target),
            ActionType::Payment => format!("Processed payment: {}", target),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snapshot_and_undo() {
        let mut manager = RollbackManager::new();

        let state_before = helpers::create_state_map(vec![
            ("name", serde_json::json!("old_name")),
        ]);
        let state_after = helpers::create_state_map(vec![
            ("name", serde_json::json!("new_name")),
        ]);

        let id = manager.snapshot_action(
            ActionType::Update,
            "Updated user name",
            state_before.clone(),
            state_after,
            Some(serde_json::json!({})),
        );

        assert!(!id.is_empty());
        assert_eq!(manager.undo_depth(), 1);

        let result = manager.undo().unwrap();
        assert!(result.success);
        assert_eq!(result.restored_state, Some(state_before));
    }

    #[test]
    fn test_undo_depth_limit() {
        let mut policy = RollbackPolicy::default();
        policy.max_undo_depth = 3;

        let mut manager = RollbackManager::new().with_policy(policy);

        for i in 0..5 {
            let state = helpers::create_state_map(vec![
                ("value", serde_json::json!(i)),
            ]);
            manager.snapshot_action(
                ActionType::Update,
                &format!("Update {}", i),
                state.clone(),
                state,
                Some(serde_json::json!({})),
            );
        }

        assert_eq!(manager.undo_depth(), 3);
    }

    #[test]
    fn test_undo_disallowed_action() {
        let mut manager = RollbackManager::new();

        manager.snapshot(ActionSnapshot {
            id: "test".to_string(),
            action_type: ActionType::Payment,
            description: "Payment made".to_string(),
            state_before: HashMap::new(),
            state_after: HashMap::new(),
            timestamp: chrono::Utc::now(),
            can_undo: true,
            rollback_data: None,
        });

        assert_eq!(manager.undo_depth(), 0);
    }
}
