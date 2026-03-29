//! Agent Team Loader — loads a 5-agent team from JSON config.
//!
//! Reads `data/agent-team/team.json`, creates all agents, sets up
//! delegation links, budgets, escalation rules, and heartbeat monitoring.

use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::budget::{AgentBudget, BudgetExceedAction};
use crate::team::{
    AgentRole, AgentTeam, EscalationAction, EscalationCondition, EscalationRule, TeamAgent,
};

/// A loaded team configuration from JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamConfig {
    pub team: TeamMeta,
    pub agents: Vec<AgentConfig>,
    #[serde(default)]
    pub escalation_rules: Vec<EscalationRuleConfig>,
    #[serde(default)]
    pub delegation_links: Vec<DelegationLinkConfig>,
}

/// Team metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamMeta {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub version: String,
}

/// Agent configuration from JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub id: String,
    pub name: String,
    pub role: String,
    #[serde(default)]
    pub title: String,
    pub description: String,
    pub model: String,
    #[serde(default)]
    pub provider: String,
    #[serde(default)]
    pub channels: Vec<String>,
    #[serde(default)]
    pub specialties: Vec<String>,
    #[serde(default = "default_true")]
    pub active: bool,
    #[serde(default)]
    pub system_prompt_file: String,
    #[serde(default)]
    pub budget: Option<BudgetConfig>,
    #[serde(default)]
    pub guard_rails: Option<serde_json::Value>,
}

fn default_true() -> bool {
    true
}

/// Budget configuration from JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetConfig {
    #[serde(default)]
    pub monthly_token_limit: u64,
    #[serde(default)]
    pub monthly_usd_limit: f64,
    #[serde(default = "default_alert")]
    pub alert_at_percent: f32,
    #[serde(default)]
    pub on_exceed: String,
    #[serde(default)]
    pub fallback_model: String,
}

fn default_alert() -> f32 {
    80.0
}

/// Escalation rule from JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscalationRuleConfig {
    pub id: String,
    pub condition: EscalationCondition,
    pub action: EscalationAction,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

/// Delegation link from JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegationLinkConfig {
    pub source: String,
    pub target: String,
    pub direction: String,
}

impl TeamConfig {
    /// Load team configuration from a JSON file.
    pub fn load(path: &Path) -> Result<Self, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read team config '{}': {}", path.display(), e))?;
        serde_json::from_str(&content).map_err(|e| format!("Failed to parse team config: {e}"))
    }

    /// Load from default path (`data/agent-team/team.json`).
    pub fn load_default() -> Result<Self, String> {
        let path = Path::new("data/agent-team/team.json");
        Self::load(path)
    }

    /// Build an `AgentTeam` from this config.
    pub fn build_team(&self) -> AgentTeam {
        let agents: Vec<TeamAgent> = self
            .agents
            .iter()
            .map(|a| {
                let role = match a.role.to_lowercase().as_str() {
                    "lead" => AgentRole::Lead,
                    "specialist" => AgentRole::Specialist,
                    "observer" => AgentRole::Observer,
                    _ => AgentRole::Member,
                };
                TeamAgent {
                    id: a.id.clone(),
                    name: a.name.clone(),
                    role,
                    channels: a.channels.clone(),
                    specialties: a.specialties.clone(),
                    model: a.model.clone(),
                    active: a.active,
                }
            })
            .collect();

        let escalation_rules: Vec<EscalationRule> = self
            .escalation_rules
            .iter()
            .map(|r| EscalationRule {
                id: r.id.clone(),
                condition: r.condition.clone(),
                action: r.action.clone(),
                enabled: r.enabled,
            })
            .collect();

        AgentTeam {
            name: self.team.name.clone(),
            description: self.team.description.clone(),
            agents,
            escalation_rules,
        }
    }

    /// Build budget configurations for all agents.
    pub fn build_budgets(&self) -> Vec<AgentBudget> {
        self.agents
            .iter()
            .filter_map(|a| {
                a.budget.as_ref().map(|b| {
                    let on_exceed = match b.on_exceed.as_str() {
                        "pause" => BudgetExceedAction::Pause,
                        "switch_to_local" => BudgetExceedAction::SwitchToLocal,
                        "hard_stop" => BudgetExceedAction::HardStop,
                        _ => BudgetExceedAction::Notify,
                    };
                    AgentBudget {
                        agent_id: a.id.clone(),
                        monthly_token_limit: b.monthly_token_limit,
                        monthly_usd_limit: b.monthly_usd_limit,
                        alert_at_percent: b.alert_at_percent,
                        on_exceed,
                        fallback_model: b.fallback_model.clone(),
                    }
                })
            })
            .collect()
    }

    /// Load system prompt for an agent from its prompt file.
    pub fn load_system_prompt(&self, agent_id: &str) -> Option<String> {
        let agent = self.agents.iter().find(|a| a.id == agent_id)?;
        if agent.system_prompt_file.is_empty() {
            return None;
        }
        let prompt_path = Path::new("data/agent-team").join(&agent.system_prompt_file);
        std::fs::read_to_string(&prompt_path).ok()
    }

    /// Get agent config by ID.
    pub fn get_agent(&self, id: &str) -> Option<&AgentConfig> {
        self.agents.iter().find(|a| a.id == id)
    }

    /// Get guard rails for an agent (as JSON value).
    pub fn get_guard_rails(&self, agent_id: &str) -> Option<serde_json::Value> {
        self.get_agent(agent_id)?.guard_rails.clone()
    }

    /// Validate the configuration.
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        // Must have at least one lead
        let has_lead = self.agents.iter().any(|a| a.role.to_lowercase() == "lead");
        if !has_lead {
            errors.push("Team must have at least one Lead agent".to_string());
        }

        // All delegation link sources/targets must exist
        let agent_ids: Vec<&str> = self.agents.iter().map(|a| a.id.as_str()).collect();
        for link in &self.delegation_links {
            if !agent_ids.contains(&link.source.as_str()) {
                errors.push(format!(
                    "Delegation link source '{}' not found",
                    link.source
                ));
            }
            if !agent_ids.contains(&link.target.as_str()) {
                errors.push(format!(
                    "Delegation link target '{}' not found",
                    link.target
                ));
            }
        }

        // Agent IDs must be unique
        let mut seen_ids = std::collections::HashSet::new();
        for agent in &self.agents {
            if !seen_ids.insert(&agent.id) {
                errors.push(format!("Duplicate agent ID: '{}'", agent.id));
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Generate a summary of the team for display.
    pub fn summary(&self) -> String {
        let mut out = format!(
            "🤖 {} (v{})\n{}\n\n",
            self.team.name, self.team.version, self.team.description
        );
        out.push_str("📋 Agents:\n");
        for a in &self.agents {
            let status = if a.active { "🟢" } else { "⚫" };
            out.push_str(&format!(
                "  {} {} ({}) — {} [{}]\n",
                status, a.name, a.title, a.model, a.role
            ));
            if !a.specialties.is_empty() {
                out.push_str(&format!("    Specialties: {}\n", a.specialties.join(", ")));
            }
        }
        out.push_str(&format!(
            "\n🔗 Delegation links: {}\n",
            self.delegation_links.len()
        ));
        out.push_str(&format!(
            "⚡ Escalation rules: {}\n",
            self.escalation_rules.len()
        ));
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_json() -> &'static str {
        r#"{
            "team": {"name": "Test Team", "description": "Test", "version": "1.0"},
            "agents": [
                {
                    "id": "lead",
                    "name": "Lead",
                    "role": "Lead",
                    "description": "Team lead",
                    "model": "test-model",
                    "channels": ["telegram"],
                    "specialties": ["orchestration"]
                },
                {
                    "id": "worker",
                    "name": "Worker",
                    "role": "Member",
                    "description": "Worker agent",
                    "model": "test-model"
                }
            ],
            "delegation_links": [
                {"source": "lead", "target": "worker", "direction": "one_way"}
            ]
        }"#
    }

    #[test]
    fn test_parse_config() {
        let config: TeamConfig = serde_json::from_str(sample_json()).unwrap();
        assert_eq!(config.team.name, "Test Team");
        assert_eq!(config.agents.len(), 2);
        assert_eq!(config.delegation_links.len(), 1);
    }

    #[test]
    fn test_build_team() {
        let config: TeamConfig = serde_json::from_str(sample_json()).unwrap();
        let team = config.build_team();
        assert_eq!(team.name, "Test Team");
        assert_eq!(team.agents.len(), 2);
        assert!(team.lead().is_some());
    }

    #[test]
    fn test_validate_ok() {
        let config: TeamConfig = serde_json::from_str(sample_json()).unwrap();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_no_lead() {
        let json = r#"{
            "team": {"name": "T", "description": "T", "version": "1"},
            "agents": [
                {"id": "w", "name": "W", "role": "Member", "description": "W", "model": "m"}
            ]
        }"#;
        let config: TeamConfig = serde_json::from_str(json).unwrap();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_bad_link() {
        let json = r#"{
            "team": {"name": "T", "description": "T", "version": "1"},
            "agents": [
                {"id": "lead", "name": "L", "role": "Lead", "description": "L", "model": "m"}
            ],
            "delegation_links": [
                {"source": "lead", "target": "nonexistent", "direction": "one_way"}
            ]
        }"#;
        let config: TeamConfig = serde_json::from_str(json).unwrap();
        let errors = config.validate().unwrap_err();
        assert!(errors.iter().any(|e| e.contains("nonexistent")));
    }

    #[test]
    fn test_summary() {
        let config: TeamConfig = serde_json::from_str(sample_json()).unwrap();
        let summary = config.summary();
        assert!(summary.contains("Test Team"));
        assert!(summary.contains("Lead"));
        assert!(summary.contains("Worker"));
    }

    #[test]
    fn test_build_budgets_empty() {
        let config: TeamConfig = serde_json::from_str(sample_json()).unwrap();
        let budgets = config.build_budgets();
        assert!(budgets.is_empty()); // No budgets in sample
    }
}
