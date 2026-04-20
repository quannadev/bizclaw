//! # TrendRadar Integration for BizClaw
//!
//! Connects BizClaw to TrendRadar MCP server for:
//! - Multi-source news aggregation
//! - AI-powered trend analysis
//! - Sentiment tracking
//! - Content generation from trends
//!
//! ## Architecture
//! ```text
//! BizClaw Agent ──calls──► TrendRadar MCP ──aggregates──► News Sources
//!                                    │                    (11 platforms)
//!                                    │                    + RSS feeds
//!                                    ↓
//!                              AI Analysis
//!                                    │
//!                                    ↓
//!                         Trend + Sentiment + Insights
//!                                    │
//!                                    ↓
//!                         Content Generator ──► Social Channels
//!                                            (Zalo, FB, Telegram)
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub mod content_generator;
pub mod trend_monitor;

pub use content_generator::{ContentGenerator, ContentTemplate, PostFormat};
pub use trend_monitor::{TrendAlert, TrendMonitor, TrendSource};

/// TrendRadar MCP tool definitions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendRadarConfig {
    pub api_url: String,
    pub interests: Vec<String>,
    pub sources: Vec<String>,
    pub language: String,
    pub sentiment_threshold: f32,
}

impl Default for TrendRadarConfig {
    fn default() -> Self {
        Self {
            api_url: "http://localhost:3333".to_string(),
            interests: vec![
                "AI".to_string(),
                "technology".to_string(),
                "startup".to_string(),
            ],
            sources: vec![
                "weibo".to_string(),
                "zhihu".to_string(),
                "baidu".to_string(),
            ],
            language: "vi".to_string(),
            sentiment_threshold: 0.5,
        }
    }
}

/// News item from TrendRadar
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendNews {
    pub id: String,
    pub title: String,
    pub url: String,
    pub platform: String,
    pub rank: u32,
    pub mentions: u64,
    pub sentiment: Sentiment,
    pub published_at: String,
    pub hotness: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Sentiment {
    Positive,
    Neutral,
    Negative,
}

impl std::fmt::Display for Sentiment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Sentiment::Positive => write!(f, "positive"),
            Sentiment::Neutral => write!(f, "neutral"),
            Sentiment::Negative => write!(f, "negative"),
        }
    }
}

/// Trend analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendAnalysis {
    pub topic: String,
    pub total_mentions: u64,
    pub platforms: Vec<String>,
    pub sentiment_breakdown: HashMap<String, u32>,
    pub trajectory: TrendTrajectory,
    pub insights: Vec<String>,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TrendTrajectory {
    Rising,
    Stable,
    Declining,
    Viral,
}

/// MCP tool call result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallResult {
    pub success: bool,
    pub data: Option<serde_json::Value>,
    pub error: Option<String>,
    pub cached: bool,
}

impl ToolCallResult {
    pub fn success(data: impl Serialize) -> Self {
        Self {
            success: true,
            data: Some(serde_json::to_value(data).unwrap_or_default()),
            error: None,
            cached: false,
        }
    }

    pub fn error(msg: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(msg.into()),
            cached: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trend_radar_config_default() {
        let config = TrendRadarConfig::default();
        assert_eq!(config.language, "vi");
        assert!(!config.interests.is_empty());
    }

    #[test]
    fn test_tool_call_result_success() {
        let result = ToolCallResult::success(vec!["test"]);
        assert!(result.success);
        assert!(result.data.is_some());
        assert!(result.error.is_none());
    }

    #[test]
    fn test_tool_call_result_error() {
        let result = ToolCallResult::<String>::error("test error");
        assert!(!result.success);
        assert!(result.data.is_none());
        assert_eq!(result.error.unwrap(), "test error");
    }

    #[test]
    fn test_sentiment_display() {
        assert_eq!(Sentiment::Positive.to_string(), "positive");
        assert_eq!(Sentiment::Neutral.to_string(), "neutral");
        assert_eq!(Sentiment::Negative.to_string(), "negative");
    }
}
