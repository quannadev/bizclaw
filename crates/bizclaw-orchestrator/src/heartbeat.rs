//! Heartbeat Monitor — agent health tracking and auto-restart.
//!
//! Each agent sends periodic heartbeats. If heartbeats stop,
//! the monitor can alert, restart, or escalate.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Agent health status.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HealthStatus {
    /// Agent is responding normally
    Healthy,
    /// Agent is responding but slowly
    Degraded,
    /// Agent has stopped responding
    Unresponsive,
    /// Agent is being restarted
    Restarting,
    /// Agent is offline (intentionally stopped)
    Offline,
}

impl std::fmt::Display for HealthStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Healthy => write!(f, "🟢 Healthy"),
            Self::Degraded => write!(f, "🟡 Degraded"),
            Self::Unresponsive => write!(f, "🔴 Unresponsive"),
            Self::Restarting => write!(f, "🔄 Restarting"),
            Self::Offline => write!(f, "⚫ Offline"),
        }
    }
}

/// Heartbeat entry for a single agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatEntry {
    /// Agent identifier
    pub agent_id: String,
    /// Current status
    pub status: HealthStatus,
    /// Last heartbeat timestamp (Unix seconds)
    pub last_heartbeat: i64,
    /// Heartbeat interval in seconds
    pub interval_seconds: u64,
    /// Number of missed heartbeats
    pub missed_count: u32,
    /// Number of restarts attempted
    pub restart_count: u32,
    /// Custom metadata from the agent
    pub metadata: serde_json::Value,
    /// Channels this agent handles
    pub channels: Vec<String>,
    /// Current task description
    pub current_task: Option<String>,
    /// Uptime in seconds since last restart
    pub uptime_seconds: u64,
}

/// Heartbeat monitor configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatConfig {
    /// Default check interval in seconds
    #[serde(default = "default_check_interval")]
    pub check_interval_seconds: u64,
    /// Number of missed heartbeats before marking as degraded
    #[serde(default = "default_degraded_threshold")]
    pub degraded_after_misses: u32,
    /// Number of missed heartbeats before marking as unresponsive
    #[serde(default = "default_unresponsive_threshold")]
    pub unresponsive_after_misses: u32,
    /// Whether to auto-restart unresponsive agents
    #[serde(default)]
    pub auto_restart: bool,
    /// Max auto-restart attempts before giving up
    #[serde(default = "default_max_restarts")]
    pub max_restart_attempts: u32,
}

fn default_check_interval() -> u64 {
    30
}
fn default_degraded_threshold() -> u32 {
    2
}
fn default_unresponsive_threshold() -> u32 {
    5
}
fn default_max_restarts() -> u32 {
    3
}

impl Default for HeartbeatConfig {
    fn default() -> Self {
        Self {
            check_interval_seconds: default_check_interval(),
            degraded_after_misses: default_degraded_threshold(),
            unresponsive_after_misses: default_unresponsive_threshold(),
            auto_restart: false,
            max_restart_attempts: default_max_restarts(),
        }
    }
}

/// Heartbeat monitor — tracks all agent health.
pub struct HeartbeatMonitor {
    entries: Arc<RwLock<HashMap<String, HeartbeatEntry>>>,
    config: HeartbeatConfig,
    /// Callback for status changes (agent_id, old_status, new_status)
    on_status_change:
        Arc<RwLock<Option<Box<dyn Fn(&str, HealthStatus, HealthStatus) + Send + Sync>>>>,
}

impl HeartbeatMonitor {
    pub fn new(config: HeartbeatConfig) -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
            config,
            on_status_change: Arc::new(RwLock::new(None)),
        }
    }

    /// Register an agent for monitoring.
    pub async fn register(&self, agent_id: &str, channels: Vec<String>, interval_seconds: u64) {
        let entry = HeartbeatEntry {
            agent_id: agent_id.to_string(),
            status: HealthStatus::Healthy,
            last_heartbeat: chrono::Utc::now().timestamp(),
            interval_seconds,
            missed_count: 0,
            restart_count: 0,
            metadata: serde_json::Value::Null,
            channels,
            current_task: None,
            uptime_seconds: 0,
        };

        self.entries
            .write()
            .await
            .insert(agent_id.to_string(), entry);
        tracing::info!("Heartbeat: registered agent '{}'", agent_id);
    }

    /// Record a heartbeat from an agent.
    pub async fn heartbeat(
        &self,
        agent_id: &str,
        metadata: Option<serde_json::Value>,
        current_task: Option<String>,
    ) {
        let mut entries = self.entries.write().await;
        if let Some(entry) = entries.get_mut(agent_id) {
            let now = chrono::Utc::now().timestamp();
            let old_status = entry.status;

            entry.last_heartbeat = now;
            entry.missed_count = 0;
            entry.status = HealthStatus::Healthy;

            if let Some(meta) = metadata {
                entry.metadata = meta;
            }
            entry.current_task = current_task;

            if old_status != HealthStatus::Healthy {
                tracing::info!(
                    "Heartbeat: agent '{}' recovered ({} → {})",
                    agent_id,
                    old_status,
                    HealthStatus::Healthy
                );
            }
        } else {
            tracing::warn!("Heartbeat: unknown agent '{}', ignoring", agent_id);
        }
    }

    /// Check all agents and update statuses.
    /// Call this periodically (e.g., every check_interval_seconds).
    pub async fn check_all(&self) -> Vec<(String, HealthStatus, HealthStatus)> {
        let now = chrono::Utc::now().timestamp();
        let mut changes = Vec::new();
        let mut entries = self.entries.write().await;

        for (id, entry) in entries.iter_mut() {
            if entry.status == HealthStatus::Offline {
                continue; // Skip offline agents
            }

            let elapsed = (now - entry.last_heartbeat) as u64;
            let expected_interval = entry.interval_seconds;
            let old_status = entry.status;

            if elapsed > expected_interval {
                entry.missed_count = (elapsed / expected_interval) as u32;
            }

            let new_status = if entry.missed_count >= self.config.unresponsive_after_misses {
                HealthStatus::Unresponsive
            } else if entry.missed_count >= self.config.degraded_after_misses {
                HealthStatus::Degraded
            } else {
                HealthStatus::Healthy
            };

            if new_status != old_status {
                entry.status = new_status;
                changes.push((id.clone(), old_status, new_status));

                tracing::warn!(
                    "Heartbeat: agent '{}' status changed: {} → {} (missed: {})",
                    id,
                    old_status,
                    new_status,
                    entry.missed_count
                );
            }
        }

        changes
    }

    /// Start the background monitoring loop.
    pub fn start_monitoring(self: Arc<Self>) -> tokio::task::JoinHandle<()> {
        let interval = self.config.check_interval_seconds;

        tokio::spawn(async move {
            let mut tick = tokio::time::interval(std::time::Duration::from_secs(interval));

            loop {
                tick.tick().await;
                let changes = self.check_all().await;

                for (agent_id, old, new) in &changes {
                    if let Some(ref cb) = *self.on_status_change.read().await {
                        cb(agent_id, *old, *new);
                    }
                }
            }
        })
    }

    /// Set status change callback.
    pub async fn on_change(
        &self,
        callback: impl Fn(&str, HealthStatus, HealthStatus) + Send + Sync + 'static,
    ) {
        *self.on_status_change.write().await = Some(Box::new(callback));
    }

    /// Get current status of an agent.
    pub async fn status(&self, agent_id: &str) -> Option<HealthStatus> {
        self.entries.read().await.get(agent_id).map(|e| e.status)
    }

    /// Mark an agent as offline.
    pub async fn mark_offline(&self, agent_id: &str) {
        if let Some(entry) = self.entries.write().await.get_mut(agent_id) {
            entry.status = HealthStatus::Offline;
            tracing::info!("Heartbeat: agent '{}' marked offline", agent_id);
        }
    }

    /// Get all entries (for dashboard).
    pub async fn all_entries(&self) -> Vec<HeartbeatEntry> {
        self.entries.read().await.values().cloned().collect()
    }

    /// Get dashboard summary.
    pub async fn summary(&self) -> serde_json::Value {
        let entries = self.entries.read().await;
        let total = entries.len();
        let healthy = entries
            .values()
            .filter(|e| e.status == HealthStatus::Healthy)
            .count();
        let degraded = entries
            .values()
            .filter(|e| e.status == HealthStatus::Degraded)
            .count();
        let unresponsive = entries
            .values()
            .filter(|e| e.status == HealthStatus::Unresponsive)
            .count();
        let offline = entries
            .values()
            .filter(|e| e.status == HealthStatus::Offline)
            .count();

        let agents: Vec<serde_json::Value> = entries
            .values()
            .map(|e| {
                serde_json::json!({
                    "agent_id": e.agent_id,
                    "status": e.status.to_string(),
                    "missed_heartbeats": e.missed_count,
                    "channels": e.channels,
                    "current_task": e.current_task,
                    "restart_count": e.restart_count,
                    "last_heartbeat_ago_seconds":
                        chrono::Utc::now().timestamp() - e.last_heartbeat,
                })
            })
            .collect();

        serde_json::json!({
            "total": total,
            "healthy": healthy,
            "degraded": degraded,
            "unresponsive": unresponsive,
            "offline": offline,
            "agents": agents,
        })
    }
}

impl Default for HeartbeatMonitor {
    fn default() -> Self {
        Self::new(HeartbeatConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_heartbeat_registration() {
        let monitor = HeartbeatMonitor::default();
        monitor.register("agent-1", vec!["zalo".into()], 30).await;

        assert_eq!(monitor.status("agent-1").await, Some(HealthStatus::Healthy));
    }

    #[tokio::test]
    async fn test_heartbeat_update() {
        let monitor = HeartbeatMonitor::default();
        monitor.register("agent-1", vec!["zalo".into()], 30).await;

        monitor
            .heartbeat(
                "agent-1",
                Some(serde_json::json!({"load": 0.5})),
                Some("Processing chat".into()),
            )
            .await;

        let entries = monitor.all_entries().await;
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].current_task, Some("Processing chat".into()));
    }

    #[tokio::test]
    async fn test_offline_marking() {
        let monitor = HeartbeatMonitor::default();
        monitor.register("agent-1", vec!["zalo".into()], 30).await;

        monitor.mark_offline("agent-1").await;
        assert_eq!(monitor.status("agent-1").await, Some(HealthStatus::Offline));
    }

    #[tokio::test]
    async fn test_heartbeat_summary() {
        let monitor = HeartbeatMonitor::default();
        monitor.register("agent-1", vec!["zalo".into()], 30).await;
        monitor
            .register("agent-2", vec!["telegram".into()], 60)
            .await;
        monitor.mark_offline("agent-2").await;

        let summary = monitor.summary().await;
        assert_eq!(summary["total"], 2);
        assert_eq!(summary["healthy"], 1);
        assert_eq!(summary["offline"], 1);
    }

    #[tokio::test]
    async fn test_check_all_empty() {
        let monitor = HeartbeatMonitor::default();
        let changes = monitor.check_all().await;
        assert!(changes.is_empty());
    }

    #[tokio::test]
    async fn test_unknown_agent_status() {
        let monitor = HeartbeatMonitor::default();
        assert_eq!(monitor.status("nobody").await, None);
    }

    #[test]
    fn test_health_status_display() {
        assert_eq!(HealthStatus::Healthy.to_string(), "🟢 Healthy");
        assert_eq!(HealthStatus::Degraded.to_string(), "🟡 Degraded");
        assert_eq!(HealthStatus::Unresponsive.to_string(), "🔴 Unresponsive");
        assert_eq!(HealthStatus::Offline.to_string(), "⚫ Offline");
    }

    #[test]
    fn test_heartbeat_config_defaults() {
        let config = HeartbeatConfig::default();
        assert_eq!(config.check_interval_seconds, 30);
        assert_eq!(config.degraded_after_misses, 2);
        assert_eq!(config.unresponsive_after_misses, 5);
        assert!(!config.auto_restart);
        assert_eq!(config.max_restart_attempts, 3);
    }
}
