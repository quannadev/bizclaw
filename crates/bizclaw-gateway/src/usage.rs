//! Usage Tracker — records token usage, costs, and request metrics.
//!
//! Tracks:
//! - Per-provider token usage (prompt/completion/total)
//! - Estimated costs based on model pricing
//! - Request counts and error rates
//! - Per-agent usage attribution
//!
//! Data is stored in-memory with periodic persistence to PostgreSQL.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

/// Global usage tracker — singleton pattern.
static TRACKER: std::sync::OnceLock<UsageTracker> = std::sync::OnceLock::new();

pub fn global_tracker() -> &'static UsageTracker {
    TRACKER.get_or_init(UsageTracker::new)
}

/// Usage statistics for a single provider.
#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct ProviderUsage {
    pub provider_name: String,
    pub total_requests: u64,
    pub total_errors: u64,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_tokens: u64,
    pub estimated_cost_usd: f64,
    pub last_request_at: Option<String>,
}

/// Usage statistics for a single agent.
#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct AgentUsage {
    pub agent_name: String,
    pub total_messages: u64,
    pub total_tokens: u64,
    pub estimated_cost_usd: f64,
}

/// Summary of all usage.
#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct UsageSummary {
    pub total_requests: u64,
    pub total_errors: u64,
    pub total_tokens: u64,
    pub total_cost_usd: f64,
    pub providers: Vec<ProviderUsage>,
    pub agents: Vec<AgentUsage>,
    pub uptime_secs: u64,
}

/// Thread-safe usage tracker.
pub struct UsageTracker {
    total_requests: AtomicU64,
    total_errors: AtomicU64,
    total_tokens: AtomicU64,
    providers: Mutex<HashMap<String, ProviderUsage>>,
    agents: Mutex<HashMap<String, AgentUsage>>,
    started_at: std::time::Instant,
}

impl Default for UsageTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl UsageTracker {
    pub fn new() -> Self {
        Self {
            total_requests: AtomicU64::new(0),
            total_errors: AtomicU64::new(0),
            total_tokens: AtomicU64::new(0),
            providers: Mutex::new(HashMap::new()),
            agents: Mutex::new(HashMap::new()),
            started_at: std::time::Instant::now(),
        }
    }

    /// Record a successful request.
    pub fn record_request(
        &self,
        provider: &str,
        agent: &str,
        prompt_tokens: u64,
        completion_tokens: u64,
        model: &str,
    ) {
        let total = prompt_tokens + completion_tokens;
        let cost = estimate_cost(model, prompt_tokens, completion_tokens);

        self.total_requests.fetch_add(1, Ordering::Relaxed);
        self.total_tokens.fetch_add(total, Ordering::Relaxed);

        // Update provider stats
        if let Ok(mut providers) = self.providers.lock() {
            let entry = providers
                .entry(provider.to_string())
                .or_insert_with(|| ProviderUsage {
                    provider_name: provider.to_string(),
                    ..Default::default()
                });
            entry.total_requests += 1;
            entry.prompt_tokens += prompt_tokens;
            entry.completion_tokens += completion_tokens;
            entry.total_tokens += total;
            entry.estimated_cost_usd += cost;
            entry.last_request_at = Some(chrono::Utc::now().to_rfc3339());
        }

        // Update agent stats
        if let Ok(mut agents) = self.agents.lock() {
            let entry = agents
                .entry(agent.to_string())
                .or_insert_with(|| AgentUsage {
                    agent_name: agent.to_string(),
                    ..Default::default()
                });
            entry.total_messages += 1;
            entry.total_tokens += total;
            entry.estimated_cost_usd += cost;
        }
    }

    /// Record an error.
    pub fn record_error(&self, provider: &str) {
        self.total_errors.fetch_add(1, Ordering::Relaxed);

        if let Ok(mut providers) = self.providers.lock() {
            let entry = providers
                .entry(provider.to_string())
                .or_insert_with(|| ProviderUsage {
                    provider_name: provider.to_string(),
                    ..Default::default()
                });
            entry.total_errors += 1;
        }
    }

    /// Get usage summary.
    pub fn summary(&self) -> UsageSummary {
        let providers: Vec<ProviderUsage> = self
            .providers
            .lock()
            .map(|p| p.values().cloned().collect())
            .unwrap_or_default();

        let agents: Vec<AgentUsage> = self
            .agents
            .lock()
            .map(|a| a.values().cloned().collect())
            .unwrap_or_default();

        let total_cost: f64 = providers.iter().map(|p| p.estimated_cost_usd).sum();

        UsageSummary {
            total_requests: self.total_requests.load(Ordering::Relaxed),
            total_errors: self.total_errors.load(Ordering::Relaxed),
            total_tokens: self.total_tokens.load(Ordering::Relaxed),
            total_cost_usd: total_cost,
            providers,
            agents,
            uptime_secs: self.started_at.elapsed().as_secs(),
        }
    }

    /// Get JSON summary.
    pub fn summary_json(&self) -> String {
        serde_json::to_string_pretty(&self.summary()).unwrap_or_else(|_| "{}".to_string())
    }
}

/// Estimate cost based on model name and token counts.
fn estimate_cost(model: &str, prompt_tokens: u64, completion_tokens: u64) -> f64 {
    let model_lower = model.to_lowercase();

    // Pricing per 1M tokens (prompt, completion)
    let (prompt_per_m, completion_per_m) = if model_lower.contains("gpt-4o") {
        (2.50, 10.00)
    } else if model_lower.contains("gpt-4") {
        (30.00, 60.00)
    } else if model_lower.contains("gpt-3.5") {
        (0.50, 1.50)
    } else if model_lower.contains("claude-3-5-sonnet") || model_lower.contains("claude-3.5-sonnet")
    {
        (3.00, 15.00)
    } else if model_lower.contains("claude-3-5-haiku") || model_lower.contains("claude-3.5-haiku") {
        (0.80, 4.00)
    } else if model_lower.contains("claude") {
        (3.00, 15.00)
    } else if model_lower.contains("gemini-2") || model_lower.contains("gemini-pro") {
        (1.25, 5.00)
    } else if model_lower.contains("gemini-flash") {
        (0.075, 0.30)
    } else if model_lower.contains("deepseek") {
        (0.14, 0.28)
    } else if model_lower.contains("llama") || model_lower.contains("mistral") {
        (0.10, 0.10) // local models — near free
    } else if model_lower.contains("qwen") {
        (0.15, 0.60)
    } else {
        (0.50, 1.50) // default
    };

    (prompt_tokens as f64 / 1_000_000.0 * prompt_per_m)
        + (completion_tokens as f64 / 1_000_000.0 * completion_per_m)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_usage_tracking() {
        let tracker = UsageTracker::new();

        tracker.record_request("openai", "agent-1", 100, 50, "gpt-4o");
        tracker.record_request("openai", "agent-1", 200, 100, "gpt-4o");
        tracker.record_request("deepseek", "agent-2", 500, 200, "deepseek-chat");

        let summary = tracker.summary();
        assert_eq!(summary.total_requests, 3);
        assert_eq!(summary.total_tokens, 1150);
        assert_eq!(summary.providers.len(), 2);
        assert_eq!(summary.agents.len(), 2);
        assert!(summary.total_cost_usd > 0.0);
    }

    #[test]
    fn test_error_tracking() {
        let tracker = UsageTracker::new();

        tracker.record_error("openai");
        tracker.record_error("openai");
        tracker.record_error("deepseek");

        let summary = tracker.summary();
        assert_eq!(summary.total_errors, 3);

        let openai = summary
            .providers
            .iter()
            .find(|p| p.provider_name == "openai")
            .unwrap();
        assert_eq!(openai.total_errors, 2);
    }

    #[test]
    fn test_cost_estimation() {
        // GPT-4o: $2.50/M prompt, $10.00/M completion
        let cost = estimate_cost("gpt-4o", 1_000_000, 1_000_000);
        assert!((cost - 12.50).abs() < 0.01);

        // DeepSeek: much cheaper
        let ds_cost = estimate_cost("deepseek-chat", 1_000_000, 1_000_000);
        assert!(ds_cost < 1.0);

        // Gemini Flash: cheapest cloud
        let flash_cost = estimate_cost("gemini-flash", 1_000_000, 1_000_000);
        assert!(flash_cost < 1.0);
    }

    #[test]
    fn test_uptime() {
        let tracker = UsageTracker::new();
        std::thread::sleep(std::time::Duration::from_millis(10));
        assert!(tracker.summary().uptime_secs < 2); // Should be ~0
    }
}
