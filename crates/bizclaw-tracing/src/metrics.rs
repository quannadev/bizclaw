use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

#[derive(Debug, Clone)]
pub struct MetricValue {
    pub name: String,
    pub value: f64,
    pub labels: HashMap<String, String>,
    pub timestamp: std::time::SystemTime,
}

pub struct MetricsCollector {
    metrics: Arc<RwLock<Vec<MetricValue>>>,
    counters: Arc<RwLock<HashMap<String, u64>>>,
    gauges: Arc<RwLock<HashMap<String, f64>>>,
    histograms: Arc<RwLock<HashMap<String, Vec<f64>>>>,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(RwLock::new(Vec::new())),
            counters: Arc::new(RwLock::new(HashMap::new())),
            gauges: Arc::new(RwLock::new(HashMap::new())),
            histograms: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn increment_counter(&self, name: &str, value: u64, labels: Option<HashMap<String, String>>) {
        let mut counters = self.counters.write().await;
        let key = name.to_string();
        *counters.entry(key).or_insert(0) += value;

        if let Some(labs) = labels {
            let mut metrics = self.metrics.write().await;
            metrics.push(MetricValue {
                name: name.to_string(),
                value: value as f64,
                labels: labs,
                timestamp: std::time::SystemTime::now(),
            });
        }
    }

    pub async fn set_gauge(&self, name: &str, value: f64) {
        let mut gauges = self.gauges.write().await;
        gauges.insert(name.to_string(), value);
    }

    pub async fn record_histogram(&self, name: &str, value: f64) {
        let mut histograms = self.histograms.write().await;
        histograms
            .entry(name.to_string())
            .or_insert_with(Vec::new)
            .push(value);
    }

    pub async fn get_counter(&self, name: &str) -> Option<u64> {
        let counters = self.counters.read().await;
        counters.get(name).copied()
    }

    pub async fn get_all_metrics(&self) -> Vec<MetricValue> {
        self.metrics.read().await.clone()
    }

    pub async fn clear(&self) {
        let mut metrics = self.metrics.write().await;
        metrics.clear();
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct RequestTimer {
    name: String,
    start_time: Instant,
    labels: HashMap<String, String>,
}

impl RequestTimer {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            start_time: Instant::now(),
            labels: HashMap::new(),
        }
    }

    pub fn with_label(mut self, key: &str, value: &str) -> Self {
        self.labels.insert(key.to_string(), value.to_string());
        self
    }

    pub async fn finish(self) -> MetricValue {
        let duration = self.start_time.elapsed();
        
        let mut labels = self.labels.clone();
        labels.insert("duration_ms".to_string(), duration.as_millis().to_string());
        
        MetricValue {
            name: self.name,
            value: duration.as_secs_f64(),
            labels,
            timestamp: std::time::SystemTime::now(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct BusinessMetrics {
    pub tools_executed: Arc<RwLock<u64>>,
    pub agents_spawned: Arc<RwLock<u64>>,
    pub messages_sent: Arc<RwLock<u64>>,
    pub database_queries: Arc<RwLock<u64>>,
    pub cache_hits: Arc<RwLock<u64>>,
    pub cache_misses: Arc<RwLock<u64>>,
}

impl BusinessMetrics {
    pub fn new() -> Self {
        Self {
            tools_executed: Arc::new(RwLock::new(0)),
            agents_spawned: Arc::new(RwLock::new(0)),
            messages_sent: Arc::new(RwLock::new(0)),
            database_queries: Arc::new(RwLock::new(0)),
            cache_hits: Arc::new(RwLock::new(0)),
            cache_misses: Arc::new(RwLock::new(0)),
        }
    }

    pub async fn record_tool(&self, _tool_name: &str) {
        let mut counter = self.tools_executed.write().await;
        *counter += 1;
    }

    pub async fn record_agent_spawn(&self, _agent_type: &str) {
        let mut counter = self.agents_spawned.write().await;
        *counter += 1;
    }

    pub async fn record_message(&self, _channel: &str) {
        let mut counter = self.messages_sent.write().await;
        *counter += 1;
    }

    pub async fn record_db_query(&self, _query_type: &str) {
        let mut counter = self.database_queries.write().await;
        *counter += 1;
    }

    pub async fn record_cache_hit(&self, _cache_name: &str) {
        let mut counter = self.cache_hits.write().await;
        *counter += 1;
    }

    pub async fn record_cache_miss(&self, _cache_name: &str) {
        let mut counter = self.cache_misses.write().await;
        *counter += 1;
    }

    pub async fn get_snapshot(&self) -> BusinessMetricsSnapshot {
        BusinessMetricsSnapshot {
            tools_executed: *self.tools_executed.read().await,
            agents_spawned: *self.agents_spawned.read().await,
            messages_sent: *self.messages_sent.read().await,
            database_queries: *self.database_queries.read().await,
            cache_hits: *self.cache_hits.read().await,
            cache_misses: *self.cache_misses.read().await,
        }
    }
}

impl Default for BusinessMetrics {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct BusinessMetricsSnapshot {
    pub tools_executed: u64,
    pub agents_spawned: u64,
    pub messages_sent: u64,
    pub database_queries: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
}

impl BusinessMetricsSnapshot {
    pub fn cache_hit_rate(&self) -> f64 {
        let total = self.cache_hits + self.cache_misses;
        if total == 0 {
            0.0
        } else {
            self.cache_hits as f64 / total as f64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_counter_increment() {
        let metrics = MetricsCollector::new();
        
        metrics.increment_counter("requests", 1, None).await;
        metrics.increment_counter("requests", 1, None).await;
        
        assert_eq!(metrics.get_counter("requests").await, Some(2));
    }

    #[tokio::test]
    async fn test_gauge() {
        let metrics = MetricsCollector::new();
        
        metrics.set_gauge("active_requests", 5.0).await;
        
        let gauges = metrics.gauges.read().await;
        assert_eq!(gauges.get("active_requests"), Some(&5.0));
    }

    #[tokio::test]
    async fn test_business_metrics() {
        let metrics = BusinessMetrics::new();
        
        metrics.record_tool("browser").await;
        metrics.record_tool("shell").await;
        
        let snapshot = metrics.get_snapshot().await;
        assert_eq!(snapshot.tools_executed, 2);
    }
}
