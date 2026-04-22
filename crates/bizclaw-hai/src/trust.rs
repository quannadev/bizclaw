//! User trust modeling for AI agents

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustScore {
    pub overall: f32,
    pub components: TrustComponents,
    pub level: TrustLevel,
    pub factors: Vec<TrustFactor>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustComponents {
    pub competence: f32,
    pub benevolence: f32,
    pub integrity: f32,
    pub transparency: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustFactor {
    pub name: String,
    pub value: f32,
    pub weight: f32,
    pub impact: TrustImpact,
    pub description: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrustImpact {
    Positive,
    Neutral,
    Negative,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TrustLevel {
    VeryHigh,
    High,
    Medium,
    Low,
    VeryLow,
}

impl TrustLevel {
    pub fn from_score(score: f32) -> Self {
        match score {
            s if s >= 0.9 => TrustLevel::VeryHigh,
            s if s >= 0.7 => TrustLevel::High,
            s if s >= 0.5 => TrustLevel::Medium,
            s if s >= 0.3 => TrustLevel::Low,
            _ => TrustLevel::VeryLow,
        }
    }

    pub fn label(&self) -> &str {
        match self {
            TrustLevel::VeryHigh => "Very High",
            TrustLevel::High => "High",
            TrustLevel::Medium => "Medium",
            TrustLevel::Low => "Low",
            TrustLevel::VeryLow => "Very Low",
        }
    }

    pub fn recommendations(&self) -> Vec<&str> {
        match self {
            TrustLevel::VeryHigh => vec!["Maintain current practices"],
            TrustLevel::High => vec!["Continue building on success"],
            TrustLevel::Medium => vec!["Focus on transparency", "Improve error handling"],
            TrustLevel::Low => vec!["Prioritize transparency", "Reduce errors", "Show more empathy"],
            TrustLevel::VeryLow => vec!["Fundamental redesign needed", "Start with transparency basics"],
        }.into_iter().map(|s| s).collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustFactors {
    pub session_id: String,
    pub interaction_count: usize,
    pub successful_interactions: usize,
    pub failed_interactions: usize,
    pub transparency_shown: f32,
    pub errors_recovered: usize,
    pub response_time_avg_ms: u64,
    pub helpfulness_ratings: Vec<u8>,
    pub corrections_made: usize,
    pub context_maintained: bool,
    pub personal_data_handled_appropriately: bool,
}

impl Default for TrustFactors {
    fn default() -> Self {
        Self {
            session_id: String::new(),
            interaction_count: 0,
            successful_interactions: 0,
            failed_interactions: 0,
            transparency_shown: 0.5,
            errors_recovered: 0,
            response_time_avg_ms: 0,
            helpfulness_ratings: Vec::new(),
            corrections_made: 0,
            context_maintained: true,
            personal_data_handled_appropriately: true,
        }
    }
}

pub struct TrustModel {
    weights: TrustWeights,
    decay_factor: f32,
}

#[derive(Debug, Clone)]
pub struct TrustWeights {
    pub competence: f32,
    pub benevolence: f32,
    pub integrity: f32,
    pub transparency: f32,
}

impl Default for TrustWeights {
    fn default() -> Self {
        Self {
            competence: 0.35,
            benevolence: 0.25,
            integrity: 0.25,
            transparency: 0.15,
        }
    }
}

impl Default for TrustModel {
    fn default() -> Self {
        Self::new()
    }
}

impl TrustModel {
    pub fn new() -> Self {
        Self {
            weights: TrustWeights::default(),
            decay_factor: 0.95,
        }
    }

    pub fn with_weights(mut self, weights: TrustWeights) -> Self {
        self.weights = weights;
        self
    }

    pub fn with_decay(mut self, decay: f32) -> Self {
        self.decay_factor = decay;
        self
    }

    pub fn calculate(&self, factors: &TrustFactors) -> TrustScore {
        let competence = self.calculate_competence(factors);
        let benevolence = self.calculate_benevolence(factors);
        let integrity = self.calculate_integrity(factors);
        let transparency = factors.transparency_shown;

        let overall = competence * self.weights.competence
            + benevolence * self.weights.benevolence
            + integrity * self.weights.integrity
            + transparency * self.weights.transparency;

        let components = TrustComponents {
            competence,
            benevolence,
            integrity,
            transparency,
        };

        let mut trust_factors = Vec::new();

        trust_factors.push(TrustFactor {
            name: "Success Rate".to_string(),
            value: if factors.interaction_count > 0 {
                factors.successful_interactions as f32 / factors.interaction_count as f32
            } else {
                0.5
            },
            weight: 0.3,
            impact: TrustImpact::Positive,
            description: "Ratio of successful to total interactions".to_string(),
        });

        trust_factors.push(TrustFactor {
            name: "Error Recovery".to_string(),
            value: if factors.failed_interactions > 0 {
                factors.errors_recovered as f32 / factors.failed_interactions as f32
            } else {
                1.0
            },
            weight: 0.2,
            impact: TrustImpact::Positive,
            description: "Ability to recover from errors".to_string(),
        });

        trust_factors.push(TrustFactor {
            name: "Response Time".to_string(),
            value: self.score_response_time(factors.response_time_avg_ms),
            weight: 0.15,
            impact: if factors.response_time_avg_ms < 3000 {
                TrustImpact::Positive
            } else {
                TrustImpact::Neutral
            },
            description: "Average response time in milliseconds".to_string(),
        });

        trust_factors.push(TrustFactor {
            name: "User Ratings".to_string(),
            value: if !factors.helpfulness_ratings.is_empty() {
                factors.helpfulness_ratings.iter().map(|&r| r as f32 / 5.0).sum::<f32>()
                    / factors.helpfulness_ratings.len() as f32
            } else {
                0.5
            },
            weight: 0.15,
            impact: TrustImpact::Positive,
            description: "Average helpfulness rating from users".to_string(),
        });

        trust_factors.push(TrustFactor {
            name: "Transparency".to_string(),
            value: factors.transparency_shown,
            weight: 0.1,
            impact: if factors.transparency_shown > 0.6 {
                TrustImpact::Positive
            } else {
                TrustImpact::Negative
            },
            description: "Level of transparency shown to user".to_string(),
        });

        trust_factors.push(TrustFactor {
            name: "Corrections".to_string(),
            value: if factors.interaction_count > 0 {
                1.0 - (factors.corrections_made as f32 / factors.interaction_count as f32)
            } else {
                1.0
            },
            weight: 0.1,
            impact: if factors.corrections_made < 2 {
                TrustImpact::Positive
            } else {
                TrustImpact::Negative
            },
            description: "Need for user corrections (lower is better)".to_string(),
        });

        TrustScore {
            overall: overall.clamp(0.0, 1.0),
            components,
            level: TrustLevel::from_score(overall),
            factors: trust_factors,
        }
    }

    fn calculate_competence(&self, factors: &TrustFactors) -> f32 {
        let success_rate = if factors.interaction_count > 0 {
            factors.successful_interactions as f32 / factors.interaction_count as f32
        } else {
            0.5
        };

        let error_recovery = if factors.failed_interactions > 0 {
            factors.errors_recovered as f32 / factors.failed_interactions as f32
        } else {
            1.0
        };

        let response_quality = self.score_response_time(factors.response_time_avg_ms);

        let accuracy = if !factors.helpfulness_ratings.is_empty() {
            factors.helpfulness_ratings.iter().map(|&r| r as f32 / 5.0).sum::<f32>()
                / factors.helpfulness_ratings.len() as f32
        } else {
            0.5
        };

        success_rate * 0.4 + error_recovery * 0.2 + response_quality * 0.2 + accuracy * 0.2
    }

    fn calculate_benevolence(&self, factors: &TrustFactors) -> f32 {
        let user_satisfaction = if !factors.helpfulness_ratings.is_empty() {
            factors.helpfulness_ratings.iter().map(|&r| r as f32 / 5.0).sum::<f32>()
                / factors.helpfulness_ratings.len() as f32
        } else {
            0.5
        };

        let privacy_respect = if factors.personal_data_handled_appropriately {
            1.0
        } else {
            0.3
        };

        user_satisfaction * 0.7 + privacy_respect * 0.3
    }

    fn calculate_integrity(&self, factors: &TrustFactors) -> f32 {
        let accuracy = if factors.interaction_count > 0 {
            1.0 - (factors.corrections_made as f32 / factors.interaction_count as f32)
        } else {
            1.0
        };

        let consistency = if factors.context_maintained {
            1.0
        } else {
            0.5
        };

        accuracy * 0.6 + consistency * 0.4
    }

    fn score_response_time(&self, time_ms: u64) -> f32 {
        match time_ms {
            0..=1000 => 1.0,
            1001..=3000 => 0.9,
            3001..=5000 => 0.7,
            5001..=10000 => 0.5,
            _ => 0.3,
        }
    }

    pub fn update_transparency(&self, factors: &mut TrustFactors, delta: f32) {
        factors.transparency_shown = (factors.transparency_shown + delta).clamp(0.0, 1.0);
    }

    pub fn record_interaction(&self, factors: &mut TrustFactors, successful: bool, rating: Option<u8>) {
        factors.interaction_count += 1;
        
        if successful {
            factors.successful_interactions += 1;
        } else {
            factors.failed_interactions += 1;
        }

        if let Some(r) = rating {
            factors.helpfulness_ratings.push(r);
        }
    }

    pub fn record_correction(&self, factors: &mut TrustFactors) {
        factors.corrections_made += 1;
    }

    pub fn record_error_recovery(&self, factors: &mut TrustFactors) {
        factors.errors_recovered += 1;
    }
}

pub mod display {
    use super::*;

    pub fn render_trust_badge(score: &TrustScore) -> String {
        let color = match score.level {
            TrustLevel::VeryHigh => "#22c55e",
            TrustLevel::High => "#84cc16",
            TrustLevel::Medium => "#eab308",
            TrustLevel::Low => "#f97316",
            TrustLevel::VeryLow => "#ef4444",
        };

        format!(
            r#"<div class="trust-badge" style="display: inline-flex; align-items: center; padding: 4px 8px; border-radius: 4px; background: {}20; border: 1px solid {};">
                <span style="font-size: 12px; font-weight: bold; color: {};">Trust: {}</span>
            </div>"#,
            color, color, color, score.level.label()
        )
    }

    pub fn render_trust_indicator(score: &TrustScore) -> String {
        let bar_width = (score.overall * 100.0) as u32;
        let color = match score.level {
            TrustLevel::VeryHigh => "#22c55e",
            TrustLevel::High => "#84cc16",
            TrustLevel::Medium => "#eab308",
            TrustLevel::Low => "#f97316",
            TrustLevel::VeryLow => "#ef4444",
        };

        format!(
            r#"<div class="trust-indicator" style="width: 200px;">
                <div style="display: flex; justify-content: space-between; margin-bottom: 2px;">
                    <span>Trust</span>
                    <span>{:.0}%</span>
                </div>
                <div style="background: #e5e7eb; border-radius: 4px; height: 6px;">
                    <div style="background: {}; width: {}%; height: 100%; border-radius: 4px;"></div>
                </div>
            </div>"#,
            score.overall * 100.0,
            color,
            bar_width
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trust_calculation() {
        let model = TrustModel::new();

        let factors = TrustFactors {
            session_id: "test".to_string(),
            interaction_count: 10,
            successful_interactions: 8,
            failed_interactions: 2,
            transparency_shown: 0.7,
            errors_recovered: 1,
            response_time_avg_ms: 2000,
            helpfulness_ratings: vec![4, 5, 4, 5],
            corrections_made: 1,
            context_maintained: true,
            personal_data_handled_appropriately: true,
        };

        let score = model.calculate(&factors);
        
        assert!(score.overall > 0.6);
        assert!(matches!(score.level, TrustLevel::High | TrustLevel::Medium));
    }

    #[test]
    fn test_trust_level_from_score() {
        assert_eq!(TrustLevel::from_score(0.95), TrustLevel::VeryHigh);
        assert_eq!(TrustLevel::from_score(0.75), TrustLevel::High);
        assert_eq!(TrustLevel::from_score(0.55), TrustLevel::Medium);
        assert_eq!(TrustLevel::from_score(0.35), TrustLevel::Low);
        assert_eq!(TrustLevel::from_score(0.15), TrustLevel::VeryLow);
    }

    #[test]
    fn test_interaction_recording() {
        let model = TrustModel::new();
        let mut factors = TrustFactors::default();

        model.record_interaction(&mut factors, true, Some(5));
        model.record_interaction(&mut factors, false, Some(3));
        model.record_interaction(&mut factors, true, None);

        assert_eq!(factors.interaction_count, 3);
        assert_eq!(factors.successful_interactions, 2);
        assert_eq!(factors.failed_interactions, 1);
        assert_eq!(factors.helpfulness_ratings.len(), 2);
    }
}
