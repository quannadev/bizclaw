//! Self-Evolution - GoClaw-style agent self-improvement
//! 
//! Features:
//! - Metrics collection
//! - Suggestion analysis
//! - Auto-adaptation with guardrails
//! - Communication style learning
//! - Capability tracking
//! - Performance feedback loops

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionConfig {
    pub enabled: bool,
    pub collection_interval_secs: u64,
    pub analysis_interval_secs: u64,
    pub adaptation_threshold: f32,
    pub guardrails_enabled: bool,
    pub max_changes_per_day: usize,
}

impl Default for EvolutionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            collection_interval_secs: 300,
            analysis_interval_secs: 3600,
            adaptation_threshold: 0.8,
            guardrails_enabled: true,
            max_changes_per_day: 10,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMetrics {
    pub agent_id: String,
    pub timestamp: DateTime<Utc>,
    pub conversation_count: u64,
    pub message_count: u64,
    pub tool_usage: HashMap<String, u64>,
    pub tool_success_rate: HashMap<String, f32>,
    pub avg_response_time_ms: u64,
    pub user_satisfaction: Option<f32>,
    pub error_count: u64,
    pub context_switches: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionSuggestion {
    pub id: String,
    pub agent_id: String,
    pub suggestion_type: SuggestionType,
    pub description: String,
    pub impact: SuggestionImpact,
    pub confidence: f32,
    pub evidence: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub status: SuggestionStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SuggestionType {
    CommunicationStyle,
    ToolUsage,
    DomainExpertise,
    ResponseFormat,
    PromptOptimization,
    ProcessImprovement,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SuggestionImpact {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SuggestionStatus {
    Pending,
    Approved,
    Rejected,
    Applied,
    RolledBack,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdaptationChange {
    pub id: String,
    pub agent_id: String,
    pub change_type: String,
    pub before: serde_json::Value,
    pub after: serde_json::Value,
    pub reason: String,
    pub applied_at: DateTime<Utc>,
    pub rolled_back_at: Option<DateTime<Utc>>,
    pub success: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capabilities {
    pub agent_id: String,
    pub communication_style: CommunicationStyle,
    pub domain_expertise: Vec<String>,
    pub preferred_tools: Vec<String>,
    pub response_patterns: Vec<ResponsePattern>,
    pub strengths: Vec<String>,
    pub weaknesses: Vec<String>,
    pub last_updated: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunicationStyle {
    pub tone: String,
    pub formality_level: f32,
    pub emoji_usage: f32,
    pub code_explanation_level: String,
    pub response_length_preference: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponsePattern {
    pub pattern: String,
    pub frequency: u64,
    pub success_rate: f32,
}

pub struct SelfEvolution {
    config: EvolutionConfig,
    metrics_history: Vec<AgentMetrics>,
    suggestions: Vec<EvolutionSuggestion>,
    adaptations: Vec<AdaptationChange>,
    capabilities: HashMap<String, Capabilities>,
    changes_today: HashMap<String, usize>,
}

impl SelfEvolution {
    pub fn new(config: EvolutionConfig) -> Self {
        Self {
            config,
            metrics_history: Vec::new(),
            suggestions: Vec::new(),
            adaptations: Vec::new(),
            capabilities: HashMap::new(),
            changes_today: HashMap::new(),
        }
    }

    pub fn collect_metrics(&mut self, metrics: AgentMetrics) {
        self.metrics_history.push(metrics.clone());
        
        // Keep only last 1000 entries
        if self.metrics_history.len() > 1000 {
            self.metrics_history.remove(0);
        }
    }

    pub fn analyze_and_suggest(&self, agent_id: &str) -> Vec<EvolutionSuggestion> {
        let mut suggestions = Vec::new();
        
        let agent_metrics: Vec<&AgentMetrics> = self.metrics_history
            .iter()
            .filter(|m| m.agent_id == agent_id)
            .collect();
        
        if agent_metrics.len() < 10 {
            return suggestions;
        }
        
        // Analyze communication style
        if let Some(style_suggestion) = self.analyze_communication_style(agent_id, &agent_metrics) {
            suggestions.push(style_suggestion);
        }
        
        // Analyze tool usage
        if let Some(tool_suggestion) = self.analyze_tool_usage(agent_id, &agent_metrics) {
            suggestions.push(tool_suggestion);
        }
        
        // Analyze domain expertise
        if let Some(expertise_suggestion) = self.analyze_domain_expertise(agent_id, &agent_metrics) {
            suggestions.push(expertise_suggestion);
        }
        
        suggestions
    }

    fn analyze_communication_style(
        &self,
        agent_id: &str,
        metrics: &[&AgentMetrics],
    ) -> Option<EvolutionSuggestion> {
        // Analyze response patterns
        let avg_length: u64 = metrics.iter().map(|m| m.avg_response_time_ms).sum::<u64>() / metrics.len() as u64;
        
        if avg_length > 5000 {
            return Some(EvolutionSuggestion {
                id: format!("suggestion_{}_{}", agent_id, Utc::now().timestamp()),
                agent_id: agent_id.to_string(),
                suggestion_type: SuggestionType::CommunicationStyle,
                description: "Consider being more concise in responses".to_string(),
                impact: SuggestionImpact::Medium,
                confidence: 0.75,
                evidence: vec!["Average response time is high".to_string()],
                created_at: Utc::now(),
                status: SuggestionStatus::Pending,
            });
        }
        
        None
    }

    fn analyze_tool_usage(
        &self,
        agent_id: &str,
        metrics: &[&AgentMetrics],
    ) -> Option<EvolutionSuggestion> {
        let mut tool_usage = HashMap::new();
        
        for m in metrics {
            for (tool, count) in &m.tool_usage {
                *tool_usage.entry(tool.clone()).or_insert(0) += count;
            }
        }
        
        // Find underutilized tools
        let unused_tools: Vec<String> = tool_usage
            .iter()
            .filter(|(_, &count)| count < 5)
            .map(|(tool, _)| tool.clone())
            .collect();
        
        if !unused_tools.is_empty() {
            return Some(EvolutionSuggestion {
                id: format!("suggestion_tool_{}_{}", agent_id, Utc::now().timestamp()),
                agent_id: agent_id.to_string(),
                suggestion_type: SuggestionType::ToolUsage,
                description: format!("Consider using more: {}", unused_tools.join(", ")),
                impact: SuggestionImpact::Low,
                confidence: 0.6,
                evidence: vec![format!("{} tools used less than 5 times", unused_tools.len())],
                created_at: Utc::now(),
                status: SuggestionStatus::Pending,
            });
        }
        
        None
    }

    fn analyze_domain_expertise(
        &self,
        agent_id: &str,
        metrics: &[&AgentMetrics],
    ) -> Option<EvolutionSuggestion> {
        let total_errors: u64 = metrics.iter().map(|m| m.error_count).sum();
        let total_messages: u64 = metrics.iter().map(|m| m.message_count).sum();
        
        if total_messages > 0 {
            let error_rate = total_errors as f32 / total_messages as f32;
            
            if error_rate > 0.1 {
                return Some(EvolutionSuggestion {
                    id: format!("suggestion_error_{}_{}", agent_id, Utc::now().timestamp()),
                    agent_id: agent_id.to_string(),
                    suggestion_type: SuggestionType::DomainExpertise,
                    description: "Error rate is high, consider improving domain knowledge".to_string(),
                    impact: SuggestionImpact::High,
                    confidence: 0.85,
                    evidence: vec![format!("Error rate: {:.2}%", error_rate * 100.0)],
                    created_at: Utc::now(),
                    status: SuggestionStatus::Pending,
                });
            }
        }
        
        None
    }

    pub fn apply_adaptation(
        &mut self,
        suggestion: &EvolutionSuggestion,
    ) -> Result<AdaptationChange, String> {
        if !self.config.enabled {
            return Err("Self-evolution is disabled".to_string());
        }
        
        // Check guardrails
        if self.config.guardrails_enabled {
            if suggestion.impact == SuggestionImpact::Critical {
                return Err("Critical changes require manual approval".to_string());
            }
        }
        
        // Check daily limit
        let today_changes = self.changes_today
            .get(&suggestion.agent_id)
            .copied()
            .unwrap_or(0);
        
        if today_changes >= self.config.max_changes_per_day {
            return Err("Daily change limit reached".to_string());
        }
        
        // Create adaptation change
        let change = AdaptationChange {
            id: format!("change_{}", Utc::now().timestamp()),
            agent_id: suggestion.agent_id.clone(),
            change_type: format!("{:?}", suggestion.suggestion_type),
            before: serde_json::json!({}),
            after: serde_json::json!({}),
            reason: suggestion.description.clone(),
            applied_at: Utc::now(),
            rolled_back_at: None,
            success: None,
        };
        
        // Record change
        *self.changes_today.entry(suggestion.agent_id.clone()).or_insert(0) += 1;
        self.adaptations.push(change.clone());
        
        Ok(change)
    }

    pub fn rollback_adaptation(&mut self, change_id: &str) -> Result<(), String> {
        let change = self.adaptations
            .iter_mut()
            .find(|c| c.id == change_id)
            .ok_or_else(|| "Change not found".to_string())?;
        
        if change.rolled_back_at.is_some() {
            return Err("Change already rolled back".to_string());
        }
        
        change.rolled_back_at = Some(Utc::now());
        change.success = Some(false);
        
        Ok(())
    }

    pub fn generate_capabilities_md(&self, agent_id: &str) -> String {
        let caps = self.capabilities.get(agent_id);
        
        let mut md = format!(
            r#"# Agent Capabilities: {}

> Auto-generated by BizClaw Self-Evolution
> Last Updated: {}

## Communication Style

| Attribute | Value |
|-----------|-------|
| Tone | {} |
| Formality | {:.0}% |
| Emoji Usage | {:.0}% |
| Code Explanation | {} |
| Response Length | {} |

## Domain Expertise

{}
## Preferred Tools

{}

## Strengths

{}

## Weaknesses

{}

## Response Patterns

| Pattern | Frequency | Success Rate |
|---------|-----------|--------------|
{}
"#,
            agent_id,
            Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
            caps.map(|c| c.communication_style.tone.as_str()).unwrap_or("Balanced"),
            caps.map(|c| c.communication_style.formality_level * 100.0).unwrap_or(50.0),
            caps.map(|c| c.communication_style.emoji_usage * 100.0).unwrap_or(20.0),
            caps.map(|c| c.communication_style.code_explanation_level.as_str()).unwrap_or("Standard"),
            caps.map(|c| c.communication_style.response_length_preference.as_str()).unwrap_or("Medium"),
        );
        
        if let Some(c) = caps {
            // Domain expertise
            md.push_str("| Domain | Confidence |\n");
            md.push_str("|--------|------------|\n");
            for domain in &c.domain_expertise {
                md.push_str(&format!("| {} | High |\n", domain));
            }
            
            // Preferred tools
            md.push_str("\n## Preferred Tools\n\n");
            for tool in &c.preferred_tools {
                md.push_str(&format!("- {}\n", tool));
            }
            
            // Strengths
            md.push_str("\n## Strengths\n\n");
            for strength in &c.strengths {
                md.push_str(&format!("- {}\n", strength));
            }
            
            // Weaknesses
            md.push_str("\n## Weaknesses\n\n");
            for weakness in &c.weaknesses {
                md.push_str(&format!("- {}\n", weakness));
            }
            
            // Response patterns
            for pattern in &c.response_patterns {
                md.push_str(&format!(
                    "| {} | {} | {:.0}% |\n",
                    pattern.pattern,
                    pattern.frequency,
                    pattern.success_rate * 100.0
                ));
            }
        }
        
        md
    }

    pub fn get_suggestions(&self, agent_id: &str, status: Option<SuggestionStatus>) -> Vec<&EvolutionSuggestion> {
        self.suggestions
            .iter()
            .filter(|s| {
                s.agent_id == agent_id &&
                (status.is_none() || Some(&s.status) == status.as_ref())
            })
            .collect()
    }

    pub fn update_capabilities(&mut self, agent_id: &str, capabilities: Capabilities) {
        self.capabilities.insert(agent_id.to_string(), capabilities);
    }
}

impl Default for SelfEvolution {
    fn default() -> Self {
        Self::new(EvolutionConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_metrics(agent_id: &str) -> AgentMetrics {
        AgentMetrics {
            agent_id: agent_id.to_string(),
            timestamp: Utc::now(),
            conversation_count: 10,
            message_count: 100,
            tool_usage: HashMap::new(),
            tool_success_rate: HashMap::new(),
            avg_response_time_ms: 2000,
            user_satisfaction: Some(4.5),
            error_count: 5,
            context_switches: 2,
        }
    }

    #[test]
    fn test_metrics_collection() {
        let mut evolution = SelfEvolution::new(EvolutionConfig::default());
        let metrics = create_test_metrics("test-agent");
        
        evolution.collect_metrics(metrics.clone());
        
        assert_eq!(evolution.metrics_history.len(), 1);
    }

    #[test]
    fn test_suggestion_generation() {
        let mut evolution = SelfEvolution::new(EvolutionConfig::default());
        let agent_id = "test-agent";
        
        // Add enough metrics for analysis
        for _ in 0..15 {
            evolution.collect_metrics(create_test_metrics(agent_id));
        }
        
        let suggestions = evolution.analyze_and_suggest(agent_id);
        assert!(!suggestions.is_empty());
    }

    #[test]
    fn test_adaptation_with_guardrails() {
        let mut evolution = SelfEvolution::new(EvolutionConfig::default());
        let agent_id = "test-agent";
        
        let suggestion = EvolutionSuggestion {
            id: "test-suggestion".to_string(),
            agent_id: agent_id.to_string(),
            suggestion_type: SuggestionType::CommunicationStyle,
            description: "Test change".to_string(),
            impact: SuggestionImpact::Medium,
            confidence: 0.8,
            evidence: vec![],
            created_at: Utc::now(),
            status: SuggestionStatus::Pending,
        };
        
        let result = evolution.apply_adaptation(&suggestion);
        assert!(result.is_ok());
    }

    #[test]
    fn test_capabilities_generation() {
        let evolution = SelfEvolution::new(EvolutionConfig::default());
        let md = evolution.generate_capabilities_md("test-agent");
        
        assert!(md.contains("# Agent Capabilities: test-agent"));
    }
}
