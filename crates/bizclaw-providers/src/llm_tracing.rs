//! LLM Call Tracing — observability for LLM API calls.
//!
//! Ported from nextlevelbuilder/goclaw's tracing system.
//! Tracks every LLM call with spans: model, tokens, latency, cost, cache hits.
//! Provides aggregated metrics for dashboards and cost tracking.
//!
//! ## Usage
//! ```rust,ignore
//! let tracer = LlmTracer::new();
//!
//! // Record a call
//! tracer.record(LlmCallSpan {
//!     model: "gpt-4o-mini".into(),
//!     provider: "openai".into(),
//!     prompt_tokens: 150,
//!     completion_tokens: 80,
//!     latency_ms: 1200,
//!     cached: false,
//!     ..Default::default()
//! }).await;
//!
//! // Get metrics
//! let metrics = tracer.metrics().await;
//! println!("Total cost: ${:.4}", metrics.total_cost_usd);
//! ```

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

/// A single LLM API call trace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmCallSpan {
    /// Unique span ID.
    pub id: String,
    /// Timestamp of the call.
    pub timestamp: DateTime<Utc>,
    /// Provider name (openai, anthropic, ollama, etc.).
    pub provider: String,
    /// Model ID.
    pub model: String,
    /// Agent or session that initiated the call.
    #[serde(default)]
    pub agent_id: String,
    /// Session/thread ID.
    #[serde(default)]
    pub session_id: String,
    /// Number of prompt (input) tokens.
    pub prompt_tokens: u64,
    /// Number of completion (output) tokens.
    pub completion_tokens: u64,
    /// Total tokens.
    pub total_tokens: u64,
    /// Latency in milliseconds.
    pub latency_ms: u64,
    /// Whether the response was served from cache.
    #[serde(default)]
    pub cached: bool,
    /// Whether the call was successful.
    #[serde(default = "bool_true")]
    pub success: bool,
    /// Error message if failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Estimated cost in USD.
    pub cost_usd: f64,
    /// Whether the call used streaming.
    #[serde(default)]
    pub streaming: bool,
    /// Tool calls count in this turn.
    #[serde(default)]
    pub tool_call_count: u32,
}

fn bool_true() -> bool {
    true
}

impl Default for LlmCallSpan {
    fn default() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string()[..8].to_string(),
            timestamp: Utc::now(),
            provider: String::new(),
            model: String::new(),
            agent_id: String::new(),
            session_id: String::new(),
            prompt_tokens: 0,
            completion_tokens: 0,
            total_tokens: 0,
            latency_ms: 0,
            cached: false,
            success: true,
            error: None,
            cost_usd: 0.0,
            streaming: false,
            tool_call_count: 0,
        }
    }
}

impl LlmCallSpan {
    /// Create a new span with auto-generated ID and timestamp.
    pub fn new(provider: &str, model: &str) -> Self {
        Self {
            provider: provider.to_string(),
            model: model.to_string(),
            ..Default::default()
        }
    }

    /// Fill in total tokens and estimate cost.
    pub fn finalize(&mut self) {
        self.total_tokens = self.prompt_tokens + self.completion_tokens;
        if self.cost_usd == 0.0 {
            self.cost_usd = estimate_cost(&self.model, self.prompt_tokens, self.completion_tokens);
        }
    }
}

/// Aggregated metrics for dashboard display.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmMetrics {
    /// Total number of LLM calls.
    pub total_calls: u64,
    /// Total successful calls.
    pub successful_calls: u64,
    /// Total failed calls.
    pub failed_calls: u64,
    /// Total prompt tokens consumed.
    pub total_prompt_tokens: u64,
    /// Total completion tokens generated.
    pub total_completion_tokens: u64,
    /// Total tokens (prompt + completion).
    pub total_tokens: u64,
    /// Total estimated cost in USD.
    pub total_cost_usd: f64,
    /// Average latency in milliseconds.
    pub avg_latency_ms: f64,
    /// P95 latency in milliseconds.
    pub p95_latency_ms: u64,
    /// Cache hit rate (0.0 - 1.0).
    pub cache_hit_rate: f64,
    /// Breakdown per provider.
    pub by_provider: HashMap<String, ProviderMetrics>,
    /// Breakdown per model.
    pub by_model: HashMap<String, ModelMetrics>,
}

/// Per-provider metrics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProviderMetrics {
    pub calls: u64,
    pub tokens: u64,
    pub cost_usd: f64,
    pub avg_latency_ms: f64,
    pub errors: u64,
}

/// Per-model metrics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModelMetrics {
    pub calls: u64,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub cost_usd: f64,
    pub avg_latency_ms: f64,
}

/// LLM Call Tracer — records and aggregates LLM API call metrics.
pub struct LlmTracer {
    /// All recorded spans (capped at max_spans).
    spans: Arc<RwLock<Vec<LlmCallSpan>>>,
    /// Maximum spans to keep in memory.
    max_spans: usize,
}

impl LlmTracer {
    /// Create a new tracer with default capacity (10,000 spans).
    pub fn new() -> Self {
        Self {
            spans: Arc::new(RwLock::new(Vec::new())),
            max_spans: 10_000,
        }
    }

    /// Create with custom max span capacity.
    pub fn with_capacity(max_spans: usize) -> Self {
        Self {
            spans: Arc::new(RwLock::new(Vec::with_capacity(max_spans.min(1000)))),
            max_spans,
        }
    }

    /// Record a completed LLM call span.
    pub async fn record(&self, mut span: LlmCallSpan) {
        span.finalize();

        debug!(
            "📊 LLM trace: {} {} | {}tok | {}ms | ${:.6}{}",
            span.provider,
            span.model,
            span.total_tokens,
            span.latency_ms,
            span.cost_usd,
            if span.cached { " (cached)" } else { "" }
        );

        let mut spans = self.spans.write().await;

        // Evict oldest spans if at capacity
        if spans.len() >= self.max_spans {
            let drain_count = self.max_spans / 10; // Remove 10% at a time
            spans.drain(..drain_count);
        }

        spans.push(span);
    }

    /// Get aggregated metrics.
    pub async fn metrics(&self) -> LlmMetrics {
        let spans = self.spans.read().await;
        Self::compute_metrics(&spans)
    }

    /// Get metrics for a specific time window (last N seconds).
    pub async fn metrics_window(&self, window_seconds: i64) -> LlmMetrics {
        let cutoff = Utc::now() - chrono::Duration::seconds(window_seconds);
        let spans = self.spans.read().await;
        let filtered: Vec<&LlmCallSpan> = spans.iter().filter(|s| s.timestamp >= cutoff).collect();

        let owned: Vec<LlmCallSpan> = filtered.into_iter().cloned().collect();
        Self::compute_metrics(&owned)
    }

    /// Get recent spans (last N).
    pub async fn recent(&self, n: usize) -> Vec<LlmCallSpan> {
        let spans = self.spans.read().await;
        spans.iter().rev().take(n).cloned().collect()
    }

    /// Get total span count.
    pub async fn count(&self) -> usize {
        self.spans.read().await.len()
    }

    /// Clear all spans.
    pub async fn clear(&self) {
        self.spans.write().await.clear();
        info!("📊 LLM tracer: cleared all spans");
    }

    /// Compute metrics from a slice of spans.
    fn compute_metrics(spans: &[LlmCallSpan]) -> LlmMetrics {
        let total_calls = spans.len() as u64;
        let successful_calls = spans.iter().filter(|s| s.success).count() as u64;
        let failed_calls = total_calls - successful_calls;

        let total_prompt_tokens: u64 = spans.iter().map(|s| s.prompt_tokens).sum();
        let total_completion_tokens: u64 = spans.iter().map(|s| s.completion_tokens).sum();
        let total_tokens = total_prompt_tokens + total_completion_tokens;
        let total_cost_usd: f64 = spans.iter().map(|s| s.cost_usd).sum();

        let cached_count = spans.iter().filter(|s| s.cached).count() as f64;
        let cache_hit_rate = if total_calls > 0 {
            cached_count / total_calls as f64
        } else {
            0.0
        };

        // Latency stats
        let mut latencies: Vec<u64> = spans.iter().map(|s| s.latency_ms).collect();
        latencies.sort();
        let avg_latency_ms = if !latencies.is_empty() {
            latencies.iter().sum::<u64>() as f64 / latencies.len() as f64
        } else {
            0.0
        };
        let p95_latency_ms = if !latencies.is_empty() {
            let idx = (latencies.len() as f64 * 0.95) as usize;
            latencies[idx.min(latencies.len() - 1)]
        } else {
            0
        };

        // Per-provider breakdown
        let mut by_provider: HashMap<String, ProviderMetrics> = HashMap::new();
        for span in spans {
            let entry = by_provider.entry(span.provider.clone()).or_default();
            entry.calls += 1;
            entry.tokens += span.total_tokens;
            entry.cost_usd += span.cost_usd;
            if !span.success {
                entry.errors += 1;
            }
            // Running average
            entry.avg_latency_ms = (entry.avg_latency_ms * (entry.calls - 1) as f64
                + span.latency_ms as f64)
                / entry.calls as f64;
        }

        // Per-model breakdown
        let mut by_model: HashMap<String, ModelMetrics> = HashMap::new();
        for span in spans {
            let entry = by_model.entry(span.model.clone()).or_default();
            entry.calls += 1;
            entry.prompt_tokens += span.prompt_tokens;
            entry.completion_tokens += span.completion_tokens;
            entry.cost_usd += span.cost_usd;
            entry.avg_latency_ms = (entry.avg_latency_ms * (entry.calls - 1) as f64
                + span.latency_ms as f64)
                / entry.calls as f64;
        }

        LlmMetrics {
            total_calls,
            successful_calls,
            failed_calls,
            total_prompt_tokens,
            total_completion_tokens,
            total_tokens,
            total_cost_usd,
            avg_latency_ms,
            p95_latency_ms,
            cache_hit_rate,
            by_provider,
            by_model,
        }
    }
}

impl Default for LlmTracer {
    fn default() -> Self {
        Self::new()
    }
}

/// Estimate cost in USD based on model pricing.
/// Uses approximate pricing as of 2026-Q1.
fn estimate_cost(model: &str, prompt_tokens: u64, completion_tokens: u64) -> f64 {
    let (prompt_per_m, completion_per_m) = match model {
        // OpenAI
        m if m.contains("gpt-4o-mini") => (0.15, 0.60),
        m if m.contains("gpt-4o") => (2.50, 10.0),
        m if m.contains("gpt-4") => (30.0, 60.0),
        m if m.contains("o1-mini") => (3.0, 12.0),
        m if m.contains("o1") => (15.0, 60.0),
        // Anthropic
        m if m.contains("claude-3-5-haiku") || m.contains("claude-3.5-haiku") => (0.80, 4.0),
        m if m.contains("claude-3-5-sonnet") || m.contains("claude-3.5-sonnet") => (3.0, 15.0),
        m if m.contains("claude-3-opus") || m.contains("claude-3-haiku") => (15.0, 75.0),
        // DeepSeek
        m if m.contains("deepseek") => (0.14, 0.28),
        // Groq (free tier / very cheap)
        m if m.contains("llama") && m.contains("groq") => (0.05, 0.08),
        // Gemini
        m if m.contains("gemini-1.5-flash") || m.contains("gemini-2.0-flash") => (0.075, 0.30),
        m if m.contains("gemini-1.5-pro") || m.contains("gemini-2.0-pro") => (1.25, 5.0),
        // Local models (free)
        m if m.contains("ollama")
            || m.contains("llamacpp")
            || m.contains("local")
            || m.contains("brain") =>
        {
            (0.0, 0.0)
        }
        // Default: approximate mid-range pricing
        _ => (0.50, 2.0),
    };

    (prompt_tokens as f64 * prompt_per_m / 1_000_000.0)
        + (completion_tokens as f64 * completion_per_m / 1_000_000.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_span(provider: &str, model: &str, latency: u64, tokens: u64) -> LlmCallSpan {
        let mut span = LlmCallSpan::new(provider, model);
        span.prompt_tokens = tokens;
        span.completion_tokens = tokens / 2;
        span.latency_ms = latency;
        span
    }

    #[tokio::test]
    async fn test_record_and_metrics() {
        let tracer = LlmTracer::new();

        tracer
            .record(sample_span("openai", "gpt-4o-mini", 500, 100))
            .await;
        tracer
            .record(sample_span("openai", "gpt-4o-mini", 600, 200))
            .await;
        tracer
            .record(sample_span("anthropic", "claude-3-5-sonnet", 1200, 150))
            .await;

        let metrics = tracer.metrics().await;
        assert_eq!(metrics.total_calls, 3);
        assert_eq!(metrics.successful_calls, 3);
        assert_eq!(metrics.failed_calls, 0);
        assert!(metrics.total_tokens > 0);
        assert!(metrics.total_cost_usd > 0.0);
        assert_eq!(metrics.by_provider.len(), 2);
        assert_eq!(metrics.by_model.len(), 2);
    }

    #[tokio::test]
    async fn test_recent_spans() {
        let tracer = LlmTracer::new();

        for i in 0..5 {
            let mut span = sample_span("openai", "gpt-4o-mini", 500, 100);
            span.id = format!("span-{}", i);
            tracer.record(span).await;
        }

        let recent = tracer.recent(3).await;
        assert_eq!(recent.len(), 3);
        assert_eq!(recent[0].id, "span-4"); // Most recent first
    }

    #[tokio::test]
    async fn test_cache_hit_rate() {
        let tracer = LlmTracer::new();

        let mut cached = sample_span("openai", "gpt-4o-mini", 50, 100);
        cached.cached = true;
        tracer.record(cached).await;

        tracer
            .record(sample_span("openai", "gpt-4o-mini", 500, 100))
            .await;

        let metrics = tracer.metrics().await;
        assert!((metrics.cache_hit_rate - 0.5).abs() < 0.01);
    }

    #[tokio::test]
    async fn test_failed_calls() {
        let tracer = LlmTracer::new();

        let mut failed = sample_span("openai", "gpt-4o-mini", 5000, 0);
        failed.success = false;
        failed.error = Some("Rate limit exceeded".into());
        tracer.record(failed).await;

        tracer
            .record(sample_span("openai", "gpt-4o-mini", 500, 100))
            .await;

        let metrics = tracer.metrics().await;
        assert_eq!(metrics.total_calls, 2);
        assert_eq!(metrics.failed_calls, 1);
        assert_eq!(metrics.successful_calls, 1);
    }

    #[tokio::test]
    async fn test_eviction() {
        let tracer = LlmTracer::with_capacity(100);

        for _ in 0..110 {
            tracer
                .record(sample_span("openai", "gpt-4o-mini", 100, 50))
                .await;
        }

        // Should have evicted 10% when hitting 100
        assert!(tracer.count().await <= 100);
    }

    #[test]
    fn test_cost_estimation() {
        // GPT-4o-mini: $0.15/1M input, $0.60/1M output
        let cost = estimate_cost("gpt-4o-mini", 1_000_000, 1_000_000);
        assert!((cost - 0.75).abs() < 0.01);

        // Local models = free
        let cost = estimate_cost("ollama-qwen3", 1_000_000, 1_000_000);
        assert_eq!(cost, 0.0);
    }

    #[tokio::test]
    async fn test_clear() {
        let tracer = LlmTracer::new();
        tracer.record(sample_span("test", "model", 100, 50)).await;
        assert_eq!(tracer.count().await, 1);

        tracer.clear().await;
        assert_eq!(tracer.count().await, 0);
    }

    #[tokio::test]
    async fn test_p95_latency() {
        let tracer = LlmTracer::new();

        // Record 20 spans with increasing latency
        for i in 1..=20 {
            tracer
                .record(sample_span("test", "model", i * 100, 50))
                .await;
        }

        let metrics = tracer.metrics().await;
        // P95 of [100, 200, ..., 2000] should be around 1900-2000
        assert!(metrics.p95_latency_ms >= 1900);
    }
}
