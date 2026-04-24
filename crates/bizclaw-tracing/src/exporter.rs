use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ExporterType {
    Jaeger,
    Zipkin,
    Otlp,
    Console,
}

impl Default for ExporterType {
    fn default() -> Self {
        ExporterType::Otlp
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExporterConfig {
    pub exporter_type: ExporterType,
    pub endpoint: String,
    pub username: Option<String>,
    pub password: Option<String>,
    pub use_tls: bool,
    pub batch_size: usize,
    pub batch_timeout_ms: u64,
}

impl Default for ExporterConfig {
    fn default() -> Self {
        Self {
            exporter_type: ExporterType::Otlp,
            endpoint: "http://localhost:4318".to_string(),
            username: None,
            password: None,
            use_tls: false,
            batch_size: 512,
            batch_timeout_ms: 5000,
        }
    }
}

impl ExporterConfig {
    pub fn jaeger(endpoint: &str) -> Self {
        Self {
            exporter_type: ExporterType::Jaeger,
            endpoint: endpoint.to_string(),
            ..Default::default()
        }
    }

    pub fn zipkin(endpoint: &str) -> Self {
        Self {
            exporter_type: ExporterType::Zipkin,
            endpoint: endpoint.to_string(),
            ..Default::default()
        }
    }

    pub fn otlp(endpoint: &str) -> Self {
        Self {
            exporter_type: ExporterType::Otlp,
            endpoint: endpoint.to_string(),
            ..Default::default()
        }
    }

    pub fn console() -> Self {
        Self {
            exporter_type: ExporterType::Console,
            endpoint: "console".to_string(),
            ..Default::default()
        }
    }

    pub fn with_auth(mut self, username: &str, password: &str) -> Self {
        self.username = Some(username.to_string());
        self.password = Some(password.to_string());
        self
    }

    pub fn with_tls(mut self) -> Self {
        self.use_tls = true;
        self
    }

    pub fn with_batch_config(mut self, size: usize, timeout_ms: u64) -> Self {
        self.batch_size = size;
        self.batch_timeout_ms = timeout_ms;
        self
    }
}
