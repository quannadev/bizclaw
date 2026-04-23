//! Agent Feedback Collection + Self-Learning.
//! Collects feedback from interactions to improve agent behavior.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackEntry {
    pub session_id: String,
    pub agent_id: String,
    pub interaction_type: InteractionType,
    pub helpful: bool,
    pub improvement_text: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InteractionType {
    ToolUse,
    Response,
    TaskCompletion,
    Error,
    UserFeedback,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FeedbackStore {
    pub entries: Vec<FeedbackEntry>,
    pub agent_metrics: HashMap<String, AgentMetrics>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentMetrics {
    pub total_interactions: u64,
    pub helpful_count: u64,
    pub unhelpful_count: u64,
}

impl AgentMetrics {
    pub fn helpfulness_score(&self) -> f32 {
        if self.total_interactions == 0 {
            return 0.5;
        }
        self.helpful_count as f32 / self.total_interactions as f32
    }
}

impl FeedbackStore {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            agent_metrics: HashMap::new(),
        }
    }
    
    pub fn record(&mut self, entry: FeedbackEntry) {
        self.entries.push(entry.clone());
        
        let metrics = self.agent_metrics
            .entry(entry.agent_id.clone())
            .or_default();
        
        metrics.total_interactions += 1;
        if entry.helpful {
            metrics.helpful_count += 1;
        } else {
            metrics.unhelpful_count += 1;
        }
    }
    
    pub fn get_hints(&self, agent_id: &str) -> String {
        self.agent_metrics
            .get(agent_id)
            .map(|m| {
                if m.helpfulness_score() < 0.5 {
                    "Consider improving response quality".to_string()
                } else {
                    String::new()
                }
            })
            .unwrap_or_default()
    }
}

pub struct FeedbackCollector {
    store: FeedbackStore,
    pub threshold: f32,
}

impl Default for FeedbackCollector {
    fn default() -> Self {
        Self::new(0.3)
    }
}

impl FeedbackCollector {
    pub fn new(threshold: f32) -> Self {
        Self {
            store: FeedbackStore::new(),
            threshold,
        }
    }
    
    pub fn record(&mut self, entry: FeedbackEntry) {
        self.store.record(entry);
    }
    
    pub fn get_optimization_hints(&self, agent_id: &str) -> String {
        self.store.get_hints(agent_id)
    }
}
