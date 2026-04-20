//! # Content Generator Module
//!
//! Generates social media content from trending topics.
//! Supports multiple platforms and formats:
//! - Zalo OA
//! - Facebook
//! - Telegram
//! - Email Newsletter
//! - Twitter/X Threads

use super::{Sentiment, TrendNews};

/// Supported post formats
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PostFormat {
    Short,      // Zalo, SMS (max 500 chars)
    Medium,     // Facebook post (500-1500 chars)
    Long,       // Email, Article (1500+ chars)
    Thread,     // Twitter thread (multiple tweets)
}

impl PostFormat {
    pub fn max_length(&self) -> usize {
        match self {
            PostFormat::Short => 500,
            PostFormat::Medium => 1500,
            PostFormat::Long => 5000,
            PostFormat::Thread => 280 * 10,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            PostFormat::Short => "short",
            PostFormat::Medium => "medium",
            PostFormat::Long => "long",
            PostFormat::Thread => "thread",
        }
    }
}

/// Channel-specific configuration
#[derive(Debug, Clone)]
pub struct ChannelConfig {
    pub format: PostFormat,
    pub max_posts_per_day: usize,
    pub include_emoji: bool,
    pub include_hashtags: bool,
    pub include_cta: bool,
    pub schedule_times: Vec<String>,
}

impl Default for ChannelConfig {
    fn default() -> Self {
        Self {
            format: PostFormat::Medium,
            max_posts_per_day: 5,
            include_emoji: true,
            include_hashtags: true,
            include_cta: true,
            schedule_times: vec![
                "09:00".to_string(),
                "12:00".to_string(),
                "18:00".to_string(),
            ],
        }
    }
}

impl ChannelConfig {
    pub fn for_zalo() -> Self {
        Self {
            format: PostFormat::Short,
            max_posts_per_day: 10,
            include_emoji: true,
            include_hashtags: false,
            include_cta: true,
            schedule_times: vec![
                "08:00".to_string(),
                "10:00".to_string(),
                "12:00".to_string(),
                "14:00".to_string(),
                "18:00".to_string(),
            ],
        }
    }

    pub fn for_facebook() -> Self {
        Self {
            format: PostFormat::Medium,
            max_posts_per_day: 5,
            include_emoji: true,
            include_hashtags: true,
            include_cta: true,
            schedule_times: vec!["10:00".to_string(), "14:00".to_string(), "19:00".to_string()],
        }
    }

    pub fn for_telegram() -> Self {
        Self {
            format: PostFormat::Medium,
            max_posts_per_day: 8,
            include_emoji: true,
            include_hashtags: true,
            include_cta: false,
            schedule_times: vec![
                "09:00".to_string(),
                "11:00".to_string(),
                "13:00".to_string(),
                "17:00".to_string(),
            ],
        }
    }

    pub fn for_email() -> Self {
        Self {
            format: PostFormat::Long,
            max_posts_per_day: 1,
            include_emoji: false,
            include_hashtags: false,
            include_cta: true,
            schedule_times: vec!["08:00".to_string()],
        }
    }
}

/// Content template
#[derive(Debug, Clone)]
pub struct ContentTemplate {
    pub name: String,
    pub format: PostFormat,
    pub template: String,
    pub variables: Vec<String>,
}

impl ContentTemplate {
    pub fn hot_topic_post() -> Self {
        Self {
            name: "hot_topic".to_string(),
            format: PostFormat::Medium,
            template: r#"🔥 [SỐT] {title}

{summary}

📊 Thông tin:
• Nền tảng: {platform}
• Lượt thảo luận: {mentions}
• Xu hướng: {trajectory}

📖 Đọc thêm: {url}

{hashtags}
{cta}"#
                .to_string(),
            variables: vec![
                "title".to_string(),
                "summary".to_string(),
                "platform".to_string(),
                "mentions".to_string(),
                "trajectory".to_string(),
                "url".to_string(),
                "hashtags".to_string(),
                "cta".to_string(),
            ],
        }
    }

    pub fn thread_post() -> Self {
        Self {
            name: "thread".to_string(),
            format: PostFormat::Thread,
            template: r#"🧵 THREAD: {topic}

1/ {hook}

2/ {context}

3/ {insights}

4/ {analysis}

5/ {action}

{hashtags}"#
                .to_string(),
            variables: vec![
                "topic".to_string(),
                "hook".to_string(),
                "context".to_string(),
                "insights".to_string(),
                "analysis".to_string(),
                "action".to_string(),
                "hashtags".to_string(),
            ],
        }
    }

    pub fn newsletter() -> Self {
        Self {
            name: "newsletter".to_string(),
            format: PostFormat::Long,
            template: r#"📊 BẢN TIN XU HƯỚNG - {date}

Xin chào!

Tuần này có những xu hướng đáng chú ý:

📈 TOP TRENDING
{top_trends}

💡 INSIGHTS
{insights}

📰 MUST-READ
{articles}

🎯 RECOMMENDED ACTIONS
{actions}

---
Đăng ký nhận tin: [Link]
Hủy đăng ký: [Link]"#
                .to_string(),
            variables: vec![
                "date".to_string(),
                "top_trends".to_string(),
                "insights".to_string(),
                "articles".to_string(),
                "actions".to_string(),
            ],
        }
    }
}

/// Generated content
#[derive(Debug, Clone, serde::Serialize)]
pub struct GeneratedContent {
    pub id: String,
    pub channel: String,
    pub format: String,
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
    pub hashtags_count: usize,
}

/// Content Generator
#[derive(Debug, Clone)]
pub struct ContentGenerator {
    templates: std::collections::HashMap<String, ContentTemplate>,
    channel_configs: std::collections::HashMap<String, ChannelConfig>,
}

impl Default for ContentGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl ContentGenerator {
    pub fn new() -> Self {
        let mut templates = std::collections::HashMap::new();
        templates.insert("hot_topic".to_string(), ContentTemplate::hot_topic_post());
        templates.insert("thread".to_string(), ContentTemplate::thread_post());
        templates.insert("newsletter".to_string(), ContentTemplate::newsletter());

        let mut channel_configs = std::collections::HashMap::new();
        channel_configs.insert("zalo".to_string(), ChannelConfig::for_zalo());
        channel_configs.insert("facebook".to_string(), ChannelConfig::for_facebook());
        channel_configs.insert("telegram".to_string(), ChannelConfig::for_telegram());
        channel_configs.insert("email".to_string(), ChannelConfig::for_email());

        Self {
            templates,
            channel_configs,
        }
    }

    /// Generate content from a single trend
    pub fn generate_from_trend(
        &self,
        trend: &TrendNews,
        channel: &str,
    ) -> GeneratedContent {
        let config = self
            .channel_configs
            .get(channel)
            .cloned()
            .unwrap_or_default();

        let template = self.templates.get("hot_topic").cloned().unwrap();

        let hashtags = self.generate_hashtags(trend);
        let hashtags_str = if config.include_hashtags {
            hashtags.join(" ")
        } else {
            String::new()
        };

        let cta = if config.include_cta {
            "\n---\n💡 Theo dõi để cập nhật tin nóng!"
        } else {
            ""
        };

        let content = template
            .template
            .replace("{title}", &trend.title)
            .replace(
                "{summary}",
                &format!(
                    "{} đang thu hút sự chú ý với {} thảo luận.",
                    trend.platform,
                    Self::format_mentions(trend.mentions)
                ),
            )
            .replace("{platform}", &trend.platform)
            .replace("{mentions}", &Self::format_mentions(trend.mentions))
            .replace("{trajectory}", &format!("{:.0}%", trend.hotness * 100.0))
            .replace("{url}", &trend.url)
            .replace("{hashtags}", &hashtags_str)
            .replace("{cta}", cta);

        let content = self.truncate_if_needed(&content, config.format.max_length());

        GeneratedContent {
            id: format!("content_{}_{}", channel, trend.id),
            channel: channel.to_string(),
            format: config.format.as_str().to_string(),
            content: content.clone(),
            hashtags: hashtags.clone(),
            scheduled_time: config.schedule_times.first().cloned(),
            metadata: ContentMetadata {
                source_trends: vec![trend.id.clone()],
                sentiment: trend.sentiment.to_string(),
                hotness: trend.hotness,
                character_count: content.len(),
                hashtags_count: hashtags.len(),
            },
        }
    }

    /// Truncate content if too long
    fn truncate_if_needed(&self, content: &str, max_len: usize) -> String {
        if content.len() <= max_len {
            return content.to_string();
        }
        let truncated: String = content.chars().take(max_len - 3).collect();
        format!("{}...", truncated)
    }

    /// Format mentions number
    fn format_mentions(mentions: u64) -> String {
        if mentions >= 1_000_000 {
            format!("{:.1}M", mentions as f64 / 1_000_000.0)
        } else if mentions >= 1_000 {
            format!("{:.1}K", mentions as f64 / 1_000.0)
        } else {
            mentions.to_string()
        }
    }

    /// Generate hashtags from trend
    fn generate_hashtags(&self, trend: &TrendNews) -> Vec<String> {
        let mut tags = vec![
            "#Trending".to_string(),
            "#Hot".to_string(),
        ];

        match trend.platform.as_str() {
            "weibo" => tags.push("#Weibo".to_string()),
            "zhihu" => tags.push("#Zhihu".to_string()),
            "baidu" => tags.push("#Baidu".to_string()),
            "toutiao" => tags.push("#TouTiao".to_string()),
            _ => tags.push(format!("#{}", trend.platform)),
        }

        match trend.sentiment {
            Sentiment::Positive => tags.push("#TíchCực".to_string()),
            Sentiment::Negative => tags.push("#TiêuCực".to_string()),
            Sentiment::Neutral => {}
        }

        tags
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_post_format_max_length() {
        assert_eq!(PostFormat::Short.max_length(), 500);
        assert_eq!(PostFormat::Medium.max_length(), 1500);
        assert_eq!(PostFormat::Long.max_length(), 5000);
    }

    #[test]
    fn test_channel_config_defaults() {
        let config = ChannelConfig::default();
        assert_eq!(config.format, PostFormat::Medium);
        assert_eq!(config.max_posts_per_day, 5);
        assert!(config.include_emoji);
        assert!(config.include_hashtags);
    }

    #[test]
    fn test_content_generator_default() {
        let generator = ContentGenerator::default();
        assert!(generator.templates.contains_key("hot_topic"));
        assert!(generator.templates.contains_key("thread"));
        assert!(generator.templates.contains_key("newsletter"));
        assert!(generator.channel_configs.contains_key("zalo"));
        assert!(generator.channel_configs.contains_key("facebook"));
    }

    #[test]
    fn test_generate_hashtags() {
        let generator = ContentGenerator::default();
        let trend = TrendNews {
            id: "test".to_string(),
            title: "Test".to_string(),
            url: "".to_string(),
            platform: "weibo".to_string(),
            rank: 1,
            mentions: 1500,
            sentiment: Sentiment::Positive,
            published_at: "".to_string(),
            hotness: 0.8,
        };
        let tags = generator.generate_hashtags(&trend);
        assert!(tags.contains(&"#Trending".to_string()));
        assert!(tags.contains(&"#Weibo".to_string()));
        assert!(tags.contains(&"#TíchCực".to_string()));
    }

    #[test]
    fn test_format_mentions() {
        assert_eq!(ContentGenerator::format_mentions(1500000), "1.5M");
        assert_eq!(ContentGenerator::format_mentions(1500), "1.5K");
        assert_eq!(ContentGenerator::format_mentions(500), "500");
    }
}
