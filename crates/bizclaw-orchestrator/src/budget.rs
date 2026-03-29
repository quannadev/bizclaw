//! Token Budget System — per-agent cost tracking and enforcement.
//!
//! Token budget tracking — each agent has a monthly token budget.
//! When budget is exceeded, the system can alert, pause, or switch models.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Budget configuration for a single agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentBudget {
    /// Agent identifier
    pub agent_id: String,
    /// Monthly token limit (0 = unlimited)
    #[serde(default)]
    pub monthly_token_limit: u64,
    /// Monthly USD limit (0.0 = unlimited)
    #[serde(default)]
    pub monthly_usd_limit: f64,
    /// Alert threshold (percentage, e.g., 80.0 = alert at 80%)
    #[serde(default = "default_alert_threshold")]
    pub alert_at_percent: f32,
    /// Action when budget is exceeded
    #[serde(default)]
    pub on_exceed: BudgetExceedAction,
    /// Fallback model when switching (for SwitchToLocal action)
    #[serde(default)]
    pub fallback_model: String,
}

fn default_alert_threshold() -> f32 {
    80.0
}

/// What to do when budget is exceeded.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum BudgetExceedAction {
    /// Just notify, don't stop
    #[default]
    Notify,
    /// Pause the agent
    Pause,
    /// Switch to local/cheaper model
    SwitchToLocal,
    /// Hard stop — reject all requests
    HardStop,
}

/// Current usage stats for an agent.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentUsage {
    /// Tokens used this month (prompt + completion)
    pub tokens_used: u64,
    /// Estimated USD cost this month
    pub usd_spent: f64,
    /// Number of requests this month
    pub request_count: u64,
    /// Timestamp of last usage update
    pub last_updated: i64,
    /// Month key (e.g., "2026-03")
    pub month_key: String,
    /// Whether alert has been sent for this month
    pub alert_sent: bool,
    /// Whether budget has been exceeded this month
    pub exceeded: bool,
}

/// Budget check result.
#[derive(Debug, Clone)]
pub enum BudgetStatus {
    /// Within budget, proceed normally
    Ok {
        tokens_remaining: u64,
        usage_percent: f32,
    },
    /// Alert threshold reached but still allowed
    Alert { usage_percent: f32, message: String },
    /// Budget exceeded — action required
    Exceeded {
        action: BudgetExceedAction,
        message: String,
    },
    /// No budget configured (unlimited)
    Unlimited,
}

/// Budget manager — tracks all agent budgets and usage.
pub struct BudgetManager {
    budgets: Arc<RwLock<HashMap<String, AgentBudget>>>,
    usage: Arc<RwLock<HashMap<String, AgentUsage>>>,
}

impl BudgetManager {
    pub fn new() -> Self {
        Self {
            budgets: Arc::new(RwLock::new(HashMap::new())),
            usage: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Set budget for an agent.
    pub async fn set_budget(&self, budget: AgentBudget) {
        let id = budget.agent_id.clone();
        self.budgets.write().await.insert(id, budget);
    }

    /// Record token usage for an agent.
    pub async fn record_usage(
        &self,
        agent_id: &str,
        prompt_tokens: u64,
        completion_tokens: u64,
        cost_usd: f64,
    ) -> BudgetStatus {
        let month_key = current_month_key();
        let total_tokens = prompt_tokens + completion_tokens;

        // Update usage
        {
            let mut usage_map = self.usage.write().await;
            let usage = usage_map
                .entry(agent_id.to_string())
                .or_insert_with(|| AgentUsage {
                    month_key: month_key.clone(),
                    ..Default::default()
                });

            // Reset if new month
            if usage.month_key != month_key {
                *usage = AgentUsage {
                    month_key: month_key.clone(),
                    ..Default::default()
                };
            }

            usage.tokens_used += total_tokens;
            usage.usd_spent += cost_usd;
            usage.request_count += 1;
            usage.last_updated = chrono::Utc::now().timestamp();
        }

        // Check budget
        self.check_budget(agent_id).await
    }

    /// Check current budget status for an agent.
    pub async fn check_budget(&self, agent_id: &str) -> BudgetStatus {
        let budgets = self.budgets.read().await;
        let usage_map = self.usage.read().await;

        let budget = match budgets.get(agent_id) {
            Some(b) => b,
            None => return BudgetStatus::Unlimited,
        };

        let usage = match usage_map.get(agent_id) {
            Some(u) => u,
            None => {
                return BudgetStatus::Ok {
                    tokens_remaining: budget.monthly_token_limit,
                    usage_percent: 0.0,
                };
            }
        };

        // Check token limit
        if budget.monthly_token_limit > 0 {
            let percent = (usage.tokens_used as f32 / budget.monthly_token_limit as f32) * 100.0;

            if percent >= 100.0 {
                return BudgetStatus::Exceeded {
                    action: budget.on_exceed.clone(),
                    message: format!(
                        "Agent '{}' exceeded token budget: {}/{} tokens ({:.1}%), ${:.4} spent",
                        agent_id,
                        usage.tokens_used,
                        budget.monthly_token_limit,
                        percent,
                        usage.usd_spent
                    ),
                };
            }

            if percent >= budget.alert_at_percent {
                return BudgetStatus::Alert {
                    usage_percent: percent,
                    message: format!(
                        "Agent '{}' approaching token budget: {}/{} tokens ({:.1}%)",
                        agent_id, usage.tokens_used, budget.monthly_token_limit, percent
                    ),
                };
            }

            return BudgetStatus::Ok {
                tokens_remaining: budget.monthly_token_limit.saturating_sub(usage.tokens_used),
                usage_percent: percent,
            };
        }

        // Check USD limit
        if budget.monthly_usd_limit > 0.0 {
            let percent = (usage.usd_spent / budget.monthly_usd_limit) as f32 * 100.0;

            if percent >= 100.0 {
                return BudgetStatus::Exceeded {
                    action: budget.on_exceed.clone(),
                    message: format!(
                        "Agent '{}' exceeded USD budget: ${:.4}/${:.2}",
                        agent_id, usage.usd_spent, budget.monthly_usd_limit
                    ),
                };
            }

            if percent >= budget.alert_at_percent {
                return BudgetStatus::Alert {
                    usage_percent: percent,
                    message: format!(
                        "Agent '{}' approaching USD budget: ${:.4}/${:.2} ({:.1}%)",
                        agent_id, usage.usd_spent, budget.monthly_usd_limit, percent
                    ),
                };
            }
        }

        BudgetStatus::Unlimited
    }

    /// Get usage stats for an agent.
    pub async fn get_usage(&self, agent_id: &str) -> Option<AgentUsage> {
        self.usage.read().await.get(agent_id).cloned()
    }

    /// Get all usage stats (for dashboard).
    pub async fn all_usage(&self) -> Vec<(String, AgentUsage, Option<AgentBudget>)> {
        let usage_map = self.usage.read().await;
        let budgets = self.budgets.read().await;

        usage_map
            .iter()
            .map(|(id, usage)| (id.clone(), usage.clone(), budgets.get(id).cloned()))
            .collect()
    }

    /// Get summary for dashboard.
    pub async fn summary(&self) -> serde_json::Value {
        let all = self.all_usage().await;
        let entries: Vec<serde_json::Value> = all
            .iter()
            .map(|(id, usage, budget)| {
                let (limit, percent) = if let Some(b) = budget {
                    if b.monthly_token_limit > 0 {
                        (
                            b.monthly_token_limit,
                            (usage.tokens_used as f32 / b.monthly_token_limit as f32) * 100.0,
                        )
                    } else {
                        (0, 0.0)
                    }
                } else {
                    (0, 0.0)
                };

                serde_json::json!({
                    "agent_id": id,
                    "tokens_used": usage.tokens_used,
                    "token_limit": limit,
                    "usage_percent": percent,
                    "usd_spent": usage.usd_spent,
                    "request_count": usage.request_count,
                    "month": usage.month_key,
                    "exceeded": usage.exceeded,
                })
            })
            .collect();

        let total_tokens: u64 = all.iter().map(|(_, u, _)| u.tokens_used).sum();
        let total_usd: f64 = all.iter().map(|(_, u, _)| u.usd_spent).sum();

        serde_json::json!({
            "total_agents": all.len(),
            "total_tokens": total_tokens,
            "total_usd": total_usd,
            "agents": entries,
        })
    }
}

impl Default for BudgetManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Get current month key (e.g., "2026-03").
fn current_month_key() -> String {
    chrono::Utc::now().format("%Y-%m").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_budget_tracking() {
        let mgr = BudgetManager::new();

        mgr.set_budget(AgentBudget {
            agent_id: "agent-1".into(),
            monthly_token_limit: 10000,
            monthly_usd_limit: 0.0,
            alert_at_percent: 80.0,
            on_exceed: BudgetExceedAction::Notify,
            fallback_model: String::new(),
        })
        .await;

        // Under budget
        let status = mgr.record_usage("agent-1", 500, 200, 0.001).await;
        assert!(matches!(status, BudgetStatus::Ok { .. }));

        // At alert threshold
        let status = mgr.record_usage("agent-1", 4000, 4000, 0.01).await;
        assert!(matches!(status, BudgetStatus::Alert { .. }));

        // Exceeded
        let status = mgr.record_usage("agent-1", 2000, 2000, 0.01).await;
        assert!(matches!(status, BudgetStatus::Exceeded { .. }));
    }

    #[tokio::test]
    async fn test_unlimited_budget() {
        let mgr = BudgetManager::new();
        let status = mgr.record_usage("unknown-agent", 1000, 1000, 0.01).await;
        assert!(matches!(status, BudgetStatus::Unlimited));
    }

    #[tokio::test]
    async fn test_budget_summary() {
        let mgr = BudgetManager::new();
        mgr.set_budget(AgentBudget {
            agent_id: "agent-1".into(),
            monthly_token_limit: 50000,
            monthly_usd_limit: 0.0,
            alert_at_percent: 80.0,
            on_exceed: BudgetExceedAction::Notify,
            fallback_model: String::new(),
        })
        .await;

        mgr.record_usage("agent-1", 1000, 500, 0.005).await;

        let summary = mgr.summary().await;
        assert_eq!(summary["total_agents"], 1);
        assert_eq!(summary["total_tokens"], 1500);
    }

    #[tokio::test]
    async fn test_budget_usd_limit() {
        let mgr = BudgetManager::new();
        mgr.set_budget(AgentBudget {
            agent_id: "agent-2".into(),
            monthly_token_limit: 0, // no token limit
            monthly_usd_limit: 1.0, // $1 USD limit
            alert_at_percent: 80.0,
            on_exceed: BudgetExceedAction::HardStop,
            fallback_model: String::new(),
        })
        .await;

        let status = mgr.record_usage("agent-2", 5000, 5000, 0.50).await;
        assert!(
            matches!(status, BudgetStatus::Ok { .. }) || matches!(status, BudgetStatus::Unlimited)
        );

        // Exceed the USD limit
        let status = mgr.record_usage("agent-2", 10000, 10000, 0.60).await;
        assert!(matches!(status, BudgetStatus::Exceeded { .. }));
    }

    #[tokio::test]
    async fn test_budget_get_usage() {
        let mgr = BudgetManager::new();
        mgr.record_usage("agent-x", 100, 50, 0.001).await;

        let usage = mgr.get_usage("agent-x").await;
        assert!(usage.is_some());
        let u = usage.unwrap();
        assert_eq!(u.tokens_used, 150);
        assert_eq!(u.request_count, 1);

        let none = mgr.get_usage("nonexistent").await;
        assert!(none.is_none());
    }

    #[test]
    fn test_budget_exceed_action_default() {
        let action = BudgetExceedAction::default();
        assert_eq!(action, BudgetExceedAction::Notify);
    }
}
