//! Distributed Tracing Module for BizClaw
//!
//! Provides tracing and metrics collection for distributed systems.
//!
//! ## Features
//! - Request tracing
//! - Metrics collection (counters, gauges, histograms)
//! - Business metrics tracking
//! - Span management
//!
//! ## Usage
//! ```rust
//! use bizclaw_tracing::{init_tracing, trace_span};
//!
//! // Initialize tracing
//! init_tracing("bizclaw-service")?;
//!
//! // Create spans
//! let span = trace_span!("api_request", operation = "fetch_data");
//! let _guard = span.enter();
//! ```

pub mod metrics;
pub mod exporter;
pub mod spans;

pub use metrics::{MetricsCollector, RequestTimer, BusinessMetrics, BusinessMetricsSnapshot};
pub use exporter::{ExporterConfig, ExporterType};
pub use spans::{Span, SpanCollector, SpanEvent, SpanStatus, generate_trace_id, generate_span_id};

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct TracingState {
    config: TracingConfig,
    span_collector: Arc<SpanCollector>,
    metrics_collector: Arc<MetricsCollector>,
}

#[derive(Debug, Clone)]
pub struct TracingConfig {
    pub service_name: String,
    pub service_version: String,
    pub enabled: bool,
}

impl Default for TracingConfig {
    fn default() -> Self {
        Self {
            service_name: "bizclaw".to_string(),
            service_version: env!("CARGO_PKG_VERSION").to_string(),
            enabled: true,
        }
    }
}

impl TracingState {
    pub fn new(config: TracingConfig) -> Self {
        Self {
            config,
            span_collector: Arc::new(SpanCollector::new(10000)),
            metrics_collector: Arc::new(MetricsCollector::new()),
        }
    }

    pub fn span_collector(&self) -> &Arc<SpanCollector> {
        &self.span_collector
    }

    pub fn metrics_collector(&self) -> &Arc<MetricsCollector> {
        &self.metrics_collector
    }
}

pub fn init_tracing(service_name: &str) -> TracingState {
    let config = TracingConfig {
        service_name: service_name.to_string(),
        service_version: env!("CARGO_PKG_VERSION").to_string(),
        enabled: true,
    };

    TracingState::new(config)
}

pub fn shutdown_tracing() {
}

pub fn current_trace_id() -> Option<String> {
    Some(generate_trace_id())
}

pub fn current_span_id() -> Option<String> {
    Some(generate_span_id())
}

#[macro_export]
macro_rules! trace_span {
    ($name:expr) => {
        tracing::info_span!(
            "bizclaw.span",
            trace_id = $crate::current_trace_id().unwrap_or_default(),
            span_id = $crate::current_span_id().unwrap_or_default(),
            operation = $name
        )
    };
    ($name:expr, $($key:ident = $value:expr),+) => {
        tracing::info_span!(
            "bizclaw.span",
            trace_id = $crate::current_trace_id().unwrap_or_default(),
            span_id = $crate::current_span_id().unwrap_or_default(),
            operation = $name,
            $($key = $value),+
        )
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tracing_config_default() {
        let config = TracingConfig::default();
        assert_eq!(config.service_name, "bizclaw");
        assert!(config.enabled);
    }

    #[test]
    fn test_init_tracing() {
        let state = init_tracing("test-service");
        assert_eq!(state.config.service_name, "test-service");
    }
}
