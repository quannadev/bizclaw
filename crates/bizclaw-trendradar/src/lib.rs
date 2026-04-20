//! # BizClaw TrendRadar
//!
//! Native TrendRadar module for BizClaw - Multi-source trend monitoring and content generation.
//!
//! ## Features
//!
//! - **Multi-source aggregation**: RSS feeds + API-based platforms
//! - **AI-powered analysis**: Sentiment, topic extraction, insights
//! - **Trend detection**: Hot topics, trajectory tracking
//! - **Auto-publishing**: Content generation for Zalo, Facebook, Telegram, Email
//! - **Scheduling**: Cron-based execution via bizclaw-hands integration

pub mod sources;
pub mod analyzer;
pub mod detector;
pub mod alerts;

pub use sources::{Source, RssSource, ApiSource, SourceConfig};
pub use analyzer::{Analyzer, AnalysisResult, Sentiment};
pub use detector::{Detector, Trend, TrendScore, Trajectory};
pub use alerts::{Alert, AlertLevel, Publisher};

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, Duration};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SourceType {
    Rss,
    Api,
    Webhook,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewsItem {
    pub id: String,
    pub title: String,
    pub url: String,
    pub summary: Option<String>,
    pub source: String,
    pub source_type: SourceType,
    pub published_at: String,
    pub rank: Option<u32>,
    pub mentions: u64,
    pub hotness: f32,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendRadarConfig {
    pub sources: Vec<sources::SourceConfig>,
    pub interests: Vec<String>,
    pub language: String,
    pub sentiment_threshold: f32,
    pub mentions_threshold: u64,
    pub hotness_threshold: f32,
    pub cache_duration_secs: u64,
    pub analyzer: analyzer::AnalyzerConfig,
}

impl Default for TrendRadarConfig {
    fn default() -> Self {
        Self {
            sources: vec![],
            interests: vec!["AI".to_string(), "technology".to_string(), "startup".to_string()],
            language: "vi".to_string(),
            sentiment_threshold: 0.5,
            mentions_threshold: 100,
            hotness_threshold: 0.7,
            cache_duration_secs: 900,
            analyzer: analyzer::AnalyzerConfig::default(),
        }
    }
}

pub struct TrendRadar {
    config: TrendRadarConfig,
    sources: Vec<Box<dyn sources::Source>>,
    analyzer: analyzer::Analyzer,
    detector: detector::Detector,
    cache: HashMap<String, (SystemTime, Vec<NewsItem>)>,
}

impl TrendRadar {
    pub fn new(config: TrendRadarConfig) -> Self {
        let sources: Vec<Box<dyn sources::Source>> = config
            .sources
            .iter()
            .filter(|s| s.enabled)
            .map(|s| sources::create_source(s))
            .collect();

        let analyzer = analyzer::Analyzer::new(config.analyzer.clone());
        let detector = detector::Detector::new(config.interests.clone());

        Self {
            config,
            sources,
            analyzer,
            detector,
            cache: HashMap::new(),
        }
    }

    pub async fn scan(&mut self) -> anyhow::Result<Vec<NewsItem>> {
        let mut all_news = Vec::new();

        for source in &mut self.sources {
            match source.fetch().await {
                Ok(news) => {
                    tracing::info!("Fetched {} items from {}", news.len(), source.name());
                    all_news.extend(news);
                }
                Err(e) => {
                    tracing::warn!("Failed to fetch from {}: {}", source.name(), e);
                }
            }
        }

        all_news.sort_by(|a, b| b.published_at.cmp(&a.published_at));
        all_news.dedup_by(|a, b| a.url == b.url);

        let now = SystemTime::now();
        self.cache.insert("scan_results".to_string(), (now, all_news.clone()));

        Ok(all_news)
    }

    pub async fn analyze(&self, news: &[NewsItem]) -> anyhow::Result<Vec<analyzer::AnalysisResult>> {
        self.analyzer.analyze_batch(news).await
    }

    pub fn detect_trends(&self, news: &[NewsItem]) -> Vec<detector::Trend> {
        self.detector.detect(news)
    }

    pub fn generate_alerts(&self, news: &[NewsItem]) -> Vec<alerts::Alert> {
        let trends = self.detector.detect(news);
        let mut alerts = Vec::new();

        for trend in trends {
            if trend.score.hotness >= self.config.hotness_threshold
                || trend.score.mentions >= self.config.mentions_threshold
            {
                alerts.push(alerts::Alert::from_trend(trend));
            }
        }

        alerts.sort_by(|a, b| b.hotness.partial_cmp(&a.hotness).unwrap());
        alerts
    }

    pub fn get_cached(&self) -> Option<Vec<NewsItem>> {
        if let Some((time, news)) = self.cache.get("scan_results") {
            let elapsed = SystemTime::now()
                .duration_since(*time)
                .unwrap_or(Duration::from_secs(0));
            if elapsed.as_secs() < self.config.cache_duration_secs {
                return Some(news.clone());
            }
        }
        None
    }
}

pub struct ContentGenerator {
    templates: HashMap<String, ContentTemplate>,
    channel_configs: HashMap<String, ChannelConfig>,
}

impl Default for ContentGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl ContentGenerator {
    pub fn new() -> Self {
        let mut templates = HashMap::new();
        templates.insert("hot_topic".to_string(), ContentTemplate::hot_topic());
        templates.insert("thread".to_string(), ContentTemplate::thread());
        templates.insert("newsletter".to_string(), ContentTemplate::newsletter());

        let mut channel_configs = HashMap::new();
        channel_configs.insert("zalo".to_string(), ChannelConfig::zalo());
        channel_configs.insert("facebook".to_string(), ChannelConfig::facebook());
        channel_configs.insert("telegram".to_string(), ChannelConfig::telegram());
        channel_configs.insert("email".to_string(), ChannelConfig::email());

        Self {
            templates,
            channel_configs,
        }
    }

    pub fn generate_from_alert(&self, alert: &alerts::Alert, channel: &str) -> GeneratedContent {
        let config = self
            .channel_configs
            .get(channel)
            .cloned()
            .unwrap_or_else(ChannelConfig::default);

        let content = self.render_template(alert, &config);

        GeneratedContent {
            id: format!("content_{}_{}", channel, alert.id),
            channel: channel.to_string(),
            content: content.clone(),
            hashtags: self.generate_hashtags(alert),
            scheduled_time: config.schedule.first().cloned(),
            metadata: ContentMetadata {
                source_trends: vec![alert.id.clone()],
                sentiment: format!("{:?}", alert.sentiment),
                hotness: alert.hotness,
                character_count: content.len(),
            },
        }
    }

    fn render_template(&self, alert: &alerts::Alert, config: &ChannelConfig) -> String {
        let template = self.templates.get("hot_topic").cloned().unwrap();

        let mut content = template.template.clone();
        content = content.replace("{title}", &alert.title);
        content = content.replace("{summary}", &alert.summary);
        content = content.replace("{source}", &alert.source.join(", "));
        content = content.replace("{trajectory}", &format!("{:?}", alert.trajectory));
        content = content.replace("{hotness}", &format!("{:.0}", alert.hotness * 100.0));

        if content.len() > config.max_chars {
            let truncated: String = content.chars().take(config.max_chars - 3).collect();
            content = format!("{}...", truncated);
        }

        content
    }

    fn generate_hashtags(&self, alert: &alerts::Alert) -> Vec<String> {
        let mut tags = vec!["#Trending".to_string(), "#Hot".to_string()];

        if let Some(source) = alert.source.first() {
            tags.push(format!("#{}", source.replace(' ', "")));
        }

        match alert.sentiment {
            analyzer::Sentiment::Positive => tags.push("#TíchCực".to_string()),
            analyzer::Sentiment::Negative => tags.push("#TiêuCực".to_string()),
            analyzer::Sentiment::Neutral => {}
        }

        tags
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct GeneratedContent {
    pub id: String,
    pub channel: String,
    pub content: String,
    pub hashtags: Vec<String>,
    pub scheduled_time: Option<String>,
    pub metadata: ContentMetadata,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ContentMetadata {
    pub source_trends: Vec<String>,
    pub sentiment: String,
    pub hotness: f32,
    pub character_count: usize,
}

#[derive(Debug, Clone)]
pub struct ContentTemplate {
    pub name: String,
    pub format: String,
    pub template: String,
}

impl ContentTemplate {
    pub fn hot_topic() -> Self {
        Self {
            name: "hot_topic".to_string(),
            format: "medium".to_string(),
            template: r#"🔥 [SỐT] {title}

{summary}

📊 Thông tin:
• Nguồn: {source}
• Xu hướng: {trajectory}
• Nhiệt độ: {hotness}%

---
💡 Theo dõi để cập nhật tin nóng!"#.to_string(),
        }
    }

    pub fn thread() -> Self {
        Self {
            name: "thread".to_string(),
            format: "thread".to_string(),
            template: r#"🧵 THREAD: {title}

1/ {summary}

---
#Trending #Hot"#.to_string(),
        }
    }

    pub fn newsletter() -> Self {
        Self {
            name: "newsletter".to_string(),
            format: "long".to_string(),
            template: r#"📊 BẢN TIN XU HƯỚNG

📈 TOP: {title}

💡 {summary}

---
Đăng ký nhận tin: [Link]"#.to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ChannelConfig {
    pub name: String,
    pub max_chars: usize,
    pub max_posts_per_day: usize,
    pub include_emoji: bool,
    pub include_hashtags: bool,
    pub schedule: Vec<String>,
}

impl Default for ChannelConfig {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
            max_chars: 1000,
            max_posts_per_day: 5,
            include_emoji: true,
            include_hashtags: true,
            schedule: vec!["09:00".to_string(), "12:00".to_string(), "18:00".to_string()],
        }
    }
}

impl ChannelConfig {
    pub fn zalo() -> Self {
        Self {
            name: "zalo".to_string(),
            max_chars: 500,
            max_posts_per_day: 10,
            include_emoji: true,
            include_hashtags: false,
            schedule: vec!["08:00".to_string(), "10:00".to_string(), "12:00".to_string(), "14:00".to_string(), "18:00".to_string()],
        }
    }

    pub fn facebook() -> Self {
        Self {
            name: "facebook".to_string(),
            max_chars: 1500,
            max_posts_per_day: 5,
            include_emoji: true,
            include_hashtags: true,
            schedule: vec!["10:00".to_string(), "14:00".to_string(), "19:00".to_string()],
        }
    }

    pub fn telegram() -> Self {
        Self {
            name: "telegram".to_string(),
            max_chars: 4000,
            max_posts_per_day: 20,
            include_emoji: true,
            include_hashtags: true,
            schedule: vec!["09:00".to_string(), "11:00".to_string(), "13:00".to_string(), "17:00".to_string()],
        }
    }

    pub fn email() -> Self {
        Self {
            name: "email".to_string(),
            max_chars: 10000,
            max_posts_per_day: 1,
            include_emoji: false,
            include_hashtags: false,
            schedule: vec!["08:00".to_string()],
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
    fn test_content_generator_default() {
        let generator = ContentGenerator::new();
        assert!(generator.templates.contains_key("hot_topic"));
        assert!(generator.templates.contains_key("thread"));
        assert!(generator.templates.contains_key("newsletter"));
    }

    #[test]
    fn test_channel_config_zalo() {
        let config = ChannelConfig::zalo();
        assert_eq!(config.max_chars, 500);
        assert_eq!(config.name, "zalo");
    }
}
