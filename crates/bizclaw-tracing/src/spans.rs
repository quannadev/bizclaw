use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SpanEvent {
    pub name: String,
    pub timestamp: SystemTime,
    pub attributes: HashMap<String, String>,
}

impl SpanEvent {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            timestamp: SystemTime::now(),
            attributes: HashMap::new(),
        }
    }

    pub fn with_attribute(mut self, key: &str, value: &str) -> Self {
        self.attributes.insert(key.to_string(), value.to_string());
        self
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Span {
    pub trace_id: String,
    pub span_id: String,
    pub parent_span_id: Option<String>,
    pub operation_name: String,
    pub service_name: String,
    pub start_time: SystemTime,
    pub end_time: Option<SystemTime>,
    pub status: SpanStatus,
    pub events: Vec<SpanEvent>,
    pub attributes: HashMap<String, String>,
    pub resource: HashMap<String, String>,
}

impl Span {
    pub fn new(trace_id: &str, span_id: &str, operation_name: &str, service_name: &str) -> Self {
        Self {
            trace_id: trace_id.to_string(),
            span_id: span_id.to_string(),
            parent_span_id: None,
            operation_name: operation_name.to_string(),
            service_name: service_name.to_string(),
            start_time: SystemTime::now(),
            end_time: None,
            status: SpanStatus::Ok,
            events: Vec::new(),
            attributes: HashMap::new(),
            resource: HashMap::new(),
        }
    }

    pub fn with_parent(mut self, parent_id: &str) -> Self {
        self.parent_span_id = Some(parent_id.to_string());
        self
    }

    pub fn with_attribute(mut self, key: &str, value: &str) -> Self {
        self.attributes.insert(key.to_string(), value.to_string());
        self
    }

    pub fn with_resource(mut self, key: &str, value: &str) -> Self {
        self.resource.insert(key.to_string(), value.to_string());
        self
    }

    pub fn add_event(&mut self, event: SpanEvent) {
        self.events.push(event);
    }

    pub fn record_error(&mut self, message: &str, error_type: &str) {
        self.status = SpanStatus::Error;
        self.events.push(
            SpanEvent::new("exception")
                .with_attribute("exception.message", message)
                .with_attribute("exception.type", error_type)
        );
    }

    pub fn finish(&mut self) {
        self.end_time = Some(SystemTime::now());
    }

    pub fn duration_ms(&self) -> Option<u128> {
        self.end_time.and_then(|end| {
            end.duration_since(self.start_time)
                .ok()
                .map(|d| d.as_millis())
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SpanStatus {
    Ok,
    Error,
    Unset,
}

impl Default for SpanStatus {
    fn default() -> Self {
        SpanStatus::Unset
    }
}

impl std::fmt::Display for SpanStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SpanStatus::Ok => write!(f, "Ok"),
            SpanStatus::Error => write!(f, "Error"),
            SpanStatus::Unset => write!(f, "Unset"),
        }
    }
}

pub struct SpanCollector {
    spans: Arc<RwLock<Vec<Span>>>,
    max_spans: usize,
}

impl SpanCollector {
    pub fn new(max_spans: usize) -> Self {
        Self {
            spans: Arc::new(RwLock::new(Vec::new())),
            max_spans,
        }
    }

    pub async fn add_span(&self, span: Span) {
        let mut spans = self.spans.write().await;
        
        if spans.len() >= self.max_spans {
            spans.remove(0);
        }
        
        spans.push(span);
    }

    pub async fn get_spans(&self) -> Vec<Span> {
        self.spans.read().await.clone()
    }

    pub async fn get_spans_by_trace(&self, trace_id: &str) -> Vec<Span> {
        self.spans
            .read()
            .await
            .iter()
            .filter(|s| s.trace_id == trace_id)
            .cloned()
            .collect()
    }

    pub async fn clear(&self) {
        self.spans.write().await.clear();
    }

    pub async fn export_json(&self) -> String {
        let spans = self.get_spans().await;
        serde_json::to_string_pretty(&spans).unwrap_or_default()
    }
}

pub fn generate_trace_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    
    let random: u128 = rand::random();
    
    format!("{:032x}", timestamp ^ random)
}

pub fn generate_span_id() -> String {
    format!("{:016x}", rand::random::<u64>())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_span_creation() {
        let span = Span::new("trace123", "span456", "test_operation", "test_service");
        
        assert_eq!(span.trace_id, "trace123");
        assert_eq!(span.span_id, "span456");
        assert_eq!(span.operation_name, "test_operation");
        assert!(span.end_time.is_none());
    }

    #[test]
    fn test_span_finish() {
        let mut span = Span::new("trace123", "span456", "test", "service");
        span.finish();
        
        assert!(span.end_time.is_some());
        assert!(span.duration_ms().is_some());
    }

    #[test]
    fn test_span_events() {
        let mut span = Span::new("trace", "span", "op", "service");
        
        span.add_event(SpanEvent::new("event1"));
        span.record_error("something went wrong", "RuntimeError");
        
        assert_eq!(span.events.len(), 2);
        assert_eq!(span.status, SpanStatus::Error);
    }

    #[test]
    fn test_generate_ids() {
        let trace_id = generate_trace_id();
        let span_id = generate_span_id();
        
        assert_eq!(trace_id.len(), 32);
        assert_eq!(span_id.len(), 16);
    }
}
