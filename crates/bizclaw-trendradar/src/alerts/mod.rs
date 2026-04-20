//! # Alerts Module
//!
//! Alert generation and publishing for detected trends.

use crate::detector::{Trend, Trajectory};
use crate::analyzer::Sentiment;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AlertLevel {
    Info,
    Warning,
    Critical,
}

impl std::fmt::Display for AlertLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AlertLevel::Info => write!(f, "info"),
            AlertLevel::Warning => write!(f, "warning"),
            AlertLevel::Critical => write!(f, "critical"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub source: Vec<String>,
    pub hotness: f32,
    pub sentiment: Sentiment,
    pub trajectory: Trajectory,
    pub timestamp: String,
    pub url: String,
    pub level: AlertLevel,
    pub action_required: bool,
    pub suggested_action: String,
}

impl Alert {
    pub fn from_trend(trend: Trend) -> Self {
        let level = Self::calculate_level(trend.score.hotness, &trend.trajectory);
        let action_required = level != AlertLevel::Info;

        let suggested_action = match level {
            AlertLevel::Critical => "Tạo content phản hồi ngay, kiểm tra sentiment trước khi đăng.".to_string(),
            AlertLevel::Warning => "Cân nhắc tạo content, theo dõi phát triển.".to_string(),
            AlertLevel::Info => "Theo dõi để cập nhật sau.".to_string(),
        };

        Self {
            id: trend.id.clone(),
            title: trend.title.clone(),
            summary: format!(
                "{} thảo luận trên {} | {} | {:.0}% nhiệt",
                trend.score.mentions,
                trend.sources.join(", "),
                format!("{:?}", trend.trajectory),
                trend.score.hotness * 100.0
            ),
            source: trend.sources,
            hotness: trend.score.hotness,
            sentiment: Sentiment::Neutral,
            trajectory: trend.trajectory,
            timestamp: trend.last_updated,
            url: String::new(),
            level,
            action_required,
            suggested_action,
        }
    }

    fn calculate_level(hotness: f32, trajectory: &Trajectory) -> AlertLevel {
        match (hotness, trajectory) {
            (h, Trajectory::Viral) if h > 0.8 => AlertLevel::Critical,
            (h, Trajectory::Rising) if h > 0.8 => AlertLevel::Warning,
            (h, _) if h > 0.9 => AlertLevel::Critical,
            (h, _) if h > 0.7 => AlertLevel::Warning,
            _ => AlertLevel::Info,
        }
    }
}

#[async_trait::async_trait]
pub trait Publisher: Send + Sync {
    fn name(&self) -> &str;
    async fn publish(&self, alert: &Alert, content: &str) -> anyhow::Result<()>;
}

#[derive(Debug, Clone)]
pub struct AlertScheduler {
    alerts: Vec<Alert>,
    max_per_batch: usize,
    cooldown_mins: u64,
    last_published: HashMap<String, Instant>,
}

impl AlertScheduler {
    pub fn new() -> Self {
        Self {
            alerts: Vec::new(),
            max_per_batch: 10,
            cooldown_mins: 30,
            last_published: HashMap::new(),
        }
    }

    pub fn with_max_batch(mut self, max: usize) -> Self {
        self.max_per_batch = max;
        self
    }

    pub fn with_cooldown(mut self, mins: u64) -> Self {
        self.cooldown_mins = mins;
        self
    }

    pub fn add(&mut self, alert: Alert) {
        if let Some(last) = self.last_published.get(&alert.id) {
            if last.elapsed() < Duration::from_mins(self.cooldown_mins) {
                tracing::debug!("Alert {} still in cooldown, skipping", alert.id);
                return;
            }
        }
        self.alerts.push(alert);
    }

    pub fn get_batch(&mut self) -> Vec<Alert> {
        let batch: Vec<Alert> = self.alerts.drain(..self.max_per_batch).collect();
        let now = Instant::now();
        for alert in &batch {
            self.last_published.insert(alert.id.clone(), now);
        }
        batch
    }

    pub fn has_pending(&self) -> bool {
        !self.alerts.is_empty()
    }

    pub fn pending_count(&self) -> usize {
        self.alerts.len()
    }
}

impl Default for AlertScheduler {
    fn default() -> Self {
        Self::new()
    }
}
