//! # Analyzer Module
//!
//! AI-powered analysis for news items.

use crate::NewsItem;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyzerConfig {
    pub provider: String,
    pub model: String,
    pub temperature: f32,
    pub max_tokens: u32,
}

impl Default for AnalyzerConfig {
    fn default() -> Self {
        Self {
            provider: "gemini".to_string(),
            model: "gemini-2.0-flash".to_string(),
            temperature: 0.7,
            max_tokens: 2048,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResult {
    pub news_id: String,
    pub sentiment: Sentiment,
    pub sentiment_score: f32,
    pub topics: Vec<String>,
    pub entities: Vec<String>,
    pub summary: String,
    pub key_insights: Vec<String>,
    pub relevance_score: f32,
}

impl AnalysisResult {
    pub fn from_news(news: &NewsItem) -> Self {
        let title_lower = news.title.to_lowercase();
        let summary_lower = news.summary.as_deref().unwrap_or("").to_lowercase();

        let positive_words = ["launch", "raise", "announce", "success", "grow", "profit", "innovative", "breakthrough", "best", "top", "win", "deal", "partner"];
        let negative_words = ["fail", "lose", "crash", "scandal", "lawsuit", "ban", "hack", "breach", "loss", "decline", "cut", "close", "shutdown"];

        let mut positive_count = 0usize;
        let mut negative_count = 0usize;

        for word in positive_words {
            if title_lower.contains(word) || summary_lower.contains(word) {
                positive_count += 1;
            }
        }

        for word in negative_words {
            if title_lower.contains(word) || summary_lower.contains(word) {
                negative_count += 1;
            }
        }

        let sentiment;
        let score;
        if positive_count > negative_count {
            sentiment = Sentiment::Positive;
            score = positive_count as f32 / (positive_count + negative_count + 1).max(1) as f32;
        } else if negative_count > positive_count {
            sentiment = Sentiment::Negative;
            score = negative_count as f32 / (positive_count + negative_count + 1).max(1) as f32;
        } else {
            sentiment = Sentiment::Neutral;
            score = 0.5;
        }

        let topics = extract_topics(&news.title);

        Self {
            news_id: news.id.clone(),
            sentiment,
            sentiment_score: score,
            topics,
            entities: Vec::new(),
            summary: news.summary.clone().unwrap_or_else(|| news.title.clone()),
            key_insights: Vec::new(),
            relevance_score: 0.7,
        }
    }
}

fn extract_topics(text: &str) -> Vec<String> {
    let mut topics = Vec::new();
    let text_lower = text.to_lowercase();

    let tech_keywords = ["ai", "artificial intelligence", "machine learning", "startup", "tech", "app", "software", "platform", "digital"];
    for keyword in tech_keywords {
        if text_lower.contains(keyword) && !topics.contains(&keyword.to_string()) {
            topics.push(keyword.to_string());
        }
    }

    let business_keywords = ["funding", "investment", "deal", "acquisition", "ipo", "revenue", "market", "stock"];
    for keyword in business_keywords {
        if text_lower.contains(keyword) && !topics.contains(&keyword.to_string()) {
            topics.push(keyword.to_string());
        }
    }

    if topics.is_empty() {
        let words: Vec<&str> = text.split_whitespace().take(3).collect();
        topics = words.iter().map(|s| s.to_string()).collect();
    }

    topics
}

#[derive(Debug, Clone)]
pub struct Analyzer {
    pub config: AnalyzerConfig,
}

impl Analyzer {
    pub fn new(config: AnalyzerConfig) -> Self {
        Self { config }
    }

    pub async fn analyze_batch(&self, news: &[NewsItem]) -> anyhow::Result<Vec<AnalysisResult>> {
        let mut results = Vec::new();
        for item in news {
            results.push(self.analyze_single(item).await);
        }
        Ok(results)
    }

    pub async fn analyze_single(&self, news: &NewsItem) -> AnalysisResult {
        AnalysisResult::from_news(news)
    }

    pub fn generate_report(&self, results: &[AnalysisResult]) -> AnalysisReport {
        let total = results.len() as f32;
        let positive = results.iter().filter(|r| matches!(r.sentiment, Sentiment::Positive)).count() as f32;
        let negative = results.iter().filter(|r| matches!(r.sentiment, Sentiment::Negative)).count() as f32;
        let neutral = results.iter().filter(|r| matches!(r.sentiment, Sentiment::Neutral)).count() as f32;

        let mut topic_counts: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
        for result in results {
            for topic in &result.topics {
                *topic_counts.entry(topic).or_insert(0) += 1;
            }
        }

        let mut top_topics: Vec<(&str, usize)> = topic_counts.into_iter().collect();
        top_topics.sort_by(|a, b| b.1.cmp(&a.1));

        AnalysisReport {
            total_analyzed: results.len(),
            sentiment_breakdown: SentimentBreakdown {
                positive: ((positive / total) * 100.0) as u32,
                neutral: ((neutral / total) * 100.0) as u32,
                negative: ((negative / total) * 100.0) as u32,
            },
            top_topics: top_topics.into_iter().take(10).map(|(t, _)| t.to_string()).collect(),
            avg_relevance: if results.is_empty() {
                0.0
            } else {
                results.iter().map(|r| r.relevance_score).sum::<f32>() / total
            },
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct AnalysisReport {
    pub total_analyzed: usize,
    pub sentiment_breakdown: SentimentBreakdown,
    pub top_topics: Vec<String>,
    pub avg_relevance: f32,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SentimentBreakdown {
    pub positive: u32,
    pub neutral: u32,
    pub negative: u32,
}
