//! Confidence indicators for agent responses

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConfidenceLevel {
    VeryHigh,
    High,
    Medium,
    Low,
    VeryLow,
}

impl ConfidenceLevel {
    pub fn from_score(score: f32) -> Self {
        match score {
            s if s >= 0.95 => ConfidenceLevel::VeryHigh,
            s if s >= 0.80 => ConfidenceLevel::High,
            s if s >= 0.60 => ConfidenceLevel::Medium,
            s if s >= 0.40 => ConfidenceLevel::Low,
            _ => ConfidenceLevel::VeryLow,
        }
    }

    pub fn label(&self) -> &str {
        match self {
            ConfidenceLevel::VeryHigh => "Very High",
            ConfidenceLevel::High => "High",
            ConfidenceLevel::Medium => "Medium",
            ConfidenceLevel::Low => "Low",
            ConfidenceLevel::VeryLow => "Very Low",
        }
    }

    pub fn color(&self) -> &str {
        match self {
            ConfidenceLevel::VeryHigh => "#22c55e",
            ConfidenceLevel::High => "#84cc16",
            ConfidenceLevel::Medium => "#eab308",
            ConfidenceLevel::Low => "#f97316",
            ConfidenceLevel::VeryLow => "#ef4444",
        }
    }

    pub fn icon(&self) -> &str {
        match self {
            ConfidenceLevel::VeryHigh => "✓✓✓",
            ConfidenceLevel::High => "✓✓",
            ConfidenceLevel::Medium => "✓",
            ConfidenceLevel::Low => "⚠",
            ConfidenceLevel::VeryLow => "⚠⚠",
        }
    }

    pub fn requires_confirmation(&self, threshold: f32) -> bool {
        let score = match self {
            ConfidenceLevel::VeryHigh => 0.98,
            ConfidenceLevel::High => 0.90,
            ConfidenceLevel::Medium => 0.70,
            ConfidenceLevel::Low => 0.50,
            ConfidenceLevel::VeryLow => 0.20,
        };
        score < threshold
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidenceIndicator {
    pub score: f32,
    pub level: ConfidenceLevel,
    pub breakdown: Option<ConfidenceBreakdown>,
    pub display: ConfidenceDisplay,
    pub metadata: ConfidenceMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidenceBreakdown {
    pub semantic: f32,
    pub factual: f32,
    pub coherence: f32,
    pub relevance: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidenceDisplay {
    pub show_indicator: bool,
    pub show_score: bool,
    pub show_breakdown: bool,
    pub format: ConfidenceFormat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfidenceFormat {
    Icon,
    Bar,
    Percentage,
    Label,
    Hidden,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidenceMetadata {
    pub model_used: String,
    pub processing_time_ms: u64,
    pub context_length: usize,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidenceConfig {
    pub display_threshold: f32,
    pub confirmation_threshold: f32,
    pub handoff_threshold: f32,
    pub show_breakdown: bool,
    pub format: ConfidenceFormat,
}

impl Default for ConfidenceConfig {
    fn default() -> Self {
        Self {
            display_threshold: 0.3,
            confirmation_threshold: 0.7,
            handoff_threshold: 0.4,
            show_breakdown: false,
            format: ConfidenceFormat::Bar,
        }
    }
}

impl ConfidenceIndicator {
    pub fn new(score: f32) -> Self {
        let score = score.clamp(0.0, 1.0);
        let level = ConfidenceLevel::from_score(score);
        
        Self {
            score,
            level,
            breakdown: None,
            display: ConfidenceDisplay {
                show_indicator: true,
                show_score: true,
                show_breakdown: false,
                format: ConfidenceFormat::Bar,
            },
            metadata: ConfidenceMetadata {
                model_used: String::new(),
                processing_time_ms: 0,
                context_length: 0,
                timestamp: chrono::Utc::now(),
            },
        }
    }

    pub fn with_breakdown(
        mut self,
        semantic: f32,
        factual: f32,
        coherence: f32,
        relevance: f32,
    ) -> Self {
        self.breakdown = Some(ConfidenceBreakdown {
            semantic: semantic.clamp(0.0, 1.0),
            factual: factual.clamp(0.0, 1.0),
            coherence: coherence.clamp(0.0, 1.0),
            relevance: relevance.clamp(0.0, 1.0),
        });
        self.display.show_breakdown = true;
        self
    }

    pub fn with_metadata(mut self, model: &str, time_ms: u64, ctx_len: usize) -> Self {
        self.metadata.model_used = model.to_string();
        self.metadata.processing_time_ms = time_ms;
        self.metadata.context_length = ctx_len;
        self
    }

    pub fn overall_score(&self) -> f32 {
        match &self.breakdown {
            Some(b) => {
                b.semantic * 0.3 + b.factual * 0.3 + b.coherence * 0.2 + b.relevance * 0.2
            },
            None => self.score,
        }
    }

    pub fn should_confirm(&self, threshold: f32) -> bool {
        self.score < threshold
    }

    pub fn should_handoff(&self, threshold: f32) -> bool {
        self.score < threshold
    }

    pub fn render_html(&self) -> String {
        if !self.display.show_indicator {
            return String::new();
        }

        let color = self.level.color();
        let label = self.level.label();
        let percentage = (self.score * 100.0) as u32;
        let bar_width = (self.score * 100.0) as u32;

        match self.display.format {
            ConfidenceFormat::Bar => {
                format!(
                    r#"<div class="confidence-indicator" style="margin: 8px 0;">
                        <div style="display: flex; justify-content: space-between; margin-bottom: 4px;">
                            <span style="color: {}; font-weight: bold;">{}</span>
                            <span>{}%</span>
                        </div>
                        <div style="background: #e5e7eb; border-radius: 4px; height: 8px; width: 100%;">
                            <div style="background: {}; width: {}%; height: 100%; border-radius: 4px;"></div>
                        </div>
                    </div>"#,
                    color, label, percentage, color, bar_width
                )
            },
            ConfidenceFormat::Icon => {
                format!(
                    r#"<span class="confidence-indicator" style="color: {};" title="Confidence: {}%">{}</span>"#,
                    color, percentage, self.level.icon()
                )
            },
            ConfidenceFormat::Percentage => {
                format!(
                    r#"<span class="confidence-indicator" style="color: {}; font-weight: bold;">{}%</span>"#,
                    color, percentage
                )
            },
            ConfidenceFormat::Label => {
                format!(
                    r#"<span class="confidence-indicator" style="color: {};">{}</span>"#,
                    color, label
                )
            },
            ConfidenceFormat::Hidden => String::new(),
        }
    }

    pub fn render_text(&self) -> String {
        let percentage = (self.score * 100.0) as u32;
        match self.display.format {
            ConfidenceFormat::Icon => format!("{}{}", self.level.icon(), percentage),
            ConfidenceFormat::Bar => {
                let filled = (self.score * 10.0) as usize;
                let empty = 10 - filled;
                format!("[{}{}] {}%", "=".repeat(filled), " ".repeat(empty), percentage)
            },
            _ => format!("{}% ({})", percentage, self.level.label()),
        }
    }
}

pub struct ConfidenceCalculator {
    #[allow(dead_code)]
    config: ConfidenceConfig,
}

impl Default for ConfidenceCalculator {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfidenceCalculator {
    pub fn new() -> Self {
        Self {
            config: ConfidenceConfig::default(),
        }
    }

    pub fn with_config(config: ConfidenceConfig) -> Self {
        Self { config }
    }

    pub fn calculate(
        &self,
        response: &str,
        context: &str,
        expected: Option<&str>,
    ) -> ConfidenceIndicator {
        let semantic = self.calculate_semantic_match(response, context);
        let factual = self.calculate_factual_accuracy(response);
        let coherence = self.calculate_coherence(response);
        let relevance = self.calculate_relevance(response, context);

        let mut indicator = ConfidenceIndicator::new(0.0)
            .with_breakdown(semantic, factual, coherence, relevance);

        if let Some(expected) = expected {
            let expected_match = self.calculate_expected_match(response, expected);
            let score = indicator.overall_score() * 0.7 + expected_match * 0.3;
            indicator = ConfidenceIndicator::new(score)
                .with_breakdown(semantic, factual, coherence, relevance);
        }

        indicator
    }

    fn calculate_semantic_match(&self, _response: &str, _context: &str) -> f32 {
        0.85
    }

    fn calculate_factual_accuracy(&self, _response: &str) -> f32 {
        0.90
    }

    fn calculate_coherence(&self, response: &str) -> f32 {
        let words: Vec<&str> = response.split_whitespace().collect();
        if words.len() < 3 {
            return 0.5;
        }
        0.85
    }

    fn calculate_relevance(&self, _response: &str, _context: &str) -> f32 {
        0.80
    }

    fn calculate_expected_match(&self, response: &str, expected: &str) -> f32 {
        let response_lower = response.to_lowercase();
        let expected_lower = expected.to_lowercase();

        let response_words: std::collections::HashSet<_> = response_lower.split_whitespace().collect();
        let expected_words: std::collections::HashSet<_> = expected_lower.split_whitespace().collect();

        if expected_words.is_empty() {
            return 0.5;
        }

        let intersection = response_words.intersection(&expected_words).count();
        intersection as f32 / expected_words.len() as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_confidence_levels() {
        assert_eq!(ConfidenceLevel::from_score(0.99), ConfidenceLevel::VeryHigh);
        assert_eq!(ConfidenceLevel::from_score(0.85), ConfidenceLevel::High);
        assert_eq!(ConfidenceLevel::from_score(0.65), ConfidenceLevel::Medium);
        assert_eq!(ConfidenceLevel::from_score(0.45), ConfidenceLevel::Low);
        assert_eq!(ConfidenceLevel::from_score(0.20), ConfidenceLevel::VeryLow);
    }

    #[test]
    fn test_indicator_creation() {
        let indicator = ConfidenceIndicator::new(0.85);
        assert_eq!(indicator.level, ConfidenceLevel::High);
        assert!(!indicator.should_confirm(0.7));
        assert!(indicator.should_handoff(0.9));
    }

    #[test]
    fn test_indicator_with_breakdown() {
        let indicator = ConfidenceIndicator::new(0.5)
            .with_breakdown(0.9, 0.8, 0.7, 0.6);

        assert!(indicator.breakdown.is_some());
        let breakdown = indicator.breakdown.unwrap();
        assert_eq!(breakdown.semantic, 0.9);
    }
}
