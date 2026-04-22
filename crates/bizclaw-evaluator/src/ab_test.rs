//! A/B evaluation framework for comparing agent versions

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ABVariant {
    pub id: String,
    pub name: String,
    pub description: String,
    pub config: HashMap<String, serde_json::Value>,
}

impl ABVariant {
    pub fn new(id: &str, name: &str) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            description: String::new(),
            config: HashMap::new(),
        }
    }

    pub fn with_description(mut self, description: &str) -> Self {
        self.description = description.to_string();
        self
    }

    pub fn with_config(mut self, key: &str, value: serde_json::Value) -> Self {
        self.config.insert(key.to_string(), value);
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrafficSplit {
    FiftyFifty,
    SeventyThirty,
    EightyTwenty,
    Custom { variant_a: f32, variant_b: f32 },
}

impl TrafficSplit {
    pub fn ratio(&self) -> (f32, f32) {
        match self {
            TrafficSplit::FiftyFifty => (0.5, 0.5),
            TrafficSplit::SeventyThirty => (0.7, 0.3),
            TrafficSplit::EightyTwenty => (0.8, 0.2),
            TrafficSplit::Custom { variant_a, variant_b } => (*variant_a, *variant_b),
        }
    }

    pub fn validate(&self) -> anyhow::Result<()> {
        let (a, b) = self.ratio();
        if (a + b - 1.0).abs() > 0.001 {
            anyhow::bail!("Traffic split must sum to 1.0, got {} + {}", a, b);
        }
        if a < 0.0 || a > 1.0 || b < 0.0 || b > 1.0 {
            anyhow::bail!("Traffic split ratios must be between 0.0 and 1.0");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ABTest {
    pub id: String,
    pub name: String,
    pub variant_a: ABVariant,
    pub variant_b: ABVariant,
    pub traffic_split: TrafficSplit,
    pub min_sample_size: usize,
    pub confidence_level: f32,
    pub start_time: chrono::DateTime<chrono::Utc>,
    pub end_time: Option<chrono::DateTime<chrono::Utc>>,
    pub status: ABTestStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ABTestStatus {
    Running,
    Paused,
    Completed,
    Terminated,
}

impl ABTest {
    pub fn new(id: &str, name: &str, variant_a: ABVariant, variant_b: ABVariant) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            variant_a,
            variant_b,
            traffic_split: TrafficSplit::FiftyFifty,
            min_sample_size: 100,
            confidence_level: 0.95,
            start_time: chrono::Utc::now(),
            end_time: None,
            status: ABTestStatus::Running,
        }
    }

    pub fn with_split(mut self, split: TrafficSplit) -> Self {
        self.traffic_split = split;
        self
    }

    pub fn with_sample_size(mut self, size: usize) -> Self {
        self.min_sample_size = size;
        self
    }

    pub fn assign_variant(&self) -> &ABVariant {
        let (a_ratio, _) = self.traffic_split.ratio();
        let rand_val = rand::random::<f32>();
        if rand_val < a_ratio {
            &self.variant_a
        } else {
            &self.variant_b
        }
    }

    pub fn is_conclusive(&self, results: &ABResult) -> bool {
        if results.total_samples < self.min_sample_size {
            return false;
        }
        results.confidence >= self.confidence_level
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sample {
    pub test_id: String,
    pub variant_id: String,
    pub input: String,
    pub output: String,
    pub score: f32,
    pub latency_ms: u64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub user_id: Option<String>,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl Sample {
    pub fn new(test_id: &str, variant_id: &str, input: &str, output: &str) -> Self {
        Self {
            test_id: test_id.to_string(),
            variant_id: variant_id.to_string(),
            input: input.to_string(),
            output: output.to_string(),
            score: 0.0,
            latency_ms: 0,
            timestamp: chrono::Utc::now(),
            user_id: None,
            metadata: HashMap::new(),
        }
    }

    pub fn with_score(mut self, score: f32) -> Self {
        self.score = score;
        self
    }

    pub fn with_latency(mut self, latency_ms: u64) -> Self {
        self.latency_ms = latency_ms;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariantStats {
    pub variant_id: String,
    pub sample_count: usize,
    pub mean_score: f32,
    pub std_dev: f32,
    pub mean_latency_ms: f32,
    pub min_score: f32,
    pub max_score: f32,
}

impl VariantStats {
    pub fn from_samples(samples: &[Sample], variant_id: &str) -> Self {
        let variant_samples: Vec<_> = samples.iter()
            .filter(|s| s.variant_id == variant_id)
            .collect();

        let count = variant_samples.len();
        if count == 0 {
            return Self {
                variant_id: variant_id.to_string(),
                sample_count: 0,
                mean_score: 0.0,
                std_dev: 0.0,
                mean_latency_ms: 0.0,
                min_score: 0.0,
                max_score: 0.0,
            };
        }

        let scores: Vec<f32> = variant_samples.iter().map(|s| s.score).collect();
        let latencies: Vec<f32> = variant_samples.iter().map(|s| s.latency_ms as f32).collect();

        let mean_score = scores.iter().sum::<f32>() / count as f32;
        let mean_latency = latencies.iter().sum::<f32>() / count as f32;

        let variance = scores.iter()
            .map(|s| (s - mean_score).powi(2))
            .sum::<f32>() / count as f32;
        let std_dev = variance.sqrt();

        let min_score = scores.iter().cloned().fold(f32::INFINITY, f32::min);
        let max_score = scores.iter().cloned().fold(f32::NEG_INFINITY, f32::max);

        Self {
            variant_id: variant_id.to_string(),
            sample_count: count,
            mean_score,
            std_dev,
            mean_latency_ms: mean_latency,
            min_score,
            max_score,
        }
    }

    pub fn standard_error(&self) -> f32 {
        if self.sample_count == 0 {
            return 0.0;
        }
        self.std_dev / (self.sample_count as f32).sqrt()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ABResult {
    pub test_id: String,
    pub variant_a_stats: VariantStats,
    pub variant_b_stats: VariantStats,
    pub total_samples: usize,
    pub confidence: f32,
    pub winner: Option<Winner>,
    pub improvement: Option<Improvement>,
    pub is_significant: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Winner {
    A,
    B,
    Tie,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Improvement {
    pub variant: String,
    pub absolute_improvement: f32,
    pub relative_improvement: f32,
}

impl ABResult {
    pub fn from_samples(test_id: &str, samples: &[Sample]) -> Self {
        let variant_a_stats = VariantStats::from_samples(samples, "a");
        let variant_b_stats = VariantStats::from_samples(samples, "b");
        let total = samples.len();

        let confidence = Self::calculate_confidence(&variant_a_stats, &variant_b_stats);
        
        let winner = if confidence >= 0.95 {
            let diff = variant_b_stats.mean_score - variant_a_stats.mean_score;
            let se = (variant_a_stats.standard_error().powi(2) 
                    + variant_b_stats.standard_error().powi(2)).sqrt();
            let z = diff / se;
            
            if z > 1.96 {
                Some(Winner::B)
            } else if z < -1.96 {
                Some(Winner::A)
            } else {
                Some(Winner::Tie)
            }
        } else {
            None
        };

        let improvement = winner.as_ref().map(|w| {
            let (improved_variant, diff) = match w {
                Winner::A => ("a", variant_a_stats.mean_score - variant_b_stats.mean_score),
                Winner::B => ("b", variant_b_stats.mean_score - variant_a_stats.mean_score),
                Winner::Tie => return Improvement {
                    variant: "none".to_string(),
                    absolute_improvement: 0.0,
                    relative_improvement: 0.0,
                },
            };
            
            let baseline = if improved_variant == "a" {
                variant_b_stats.mean_score
            } else {
                variant_a_stats.mean_score
            };

            let relative = if baseline > 0.0 {
                diff / baseline * 100.0
            } else {
                0.0
            };

            Improvement {
                variant: improved_variant.to_string(),
                absolute_improvement: diff,
                relative_improvement: relative,
            }
        });

        Self {
            test_id: test_id.to_string(),
            variant_a_stats,
            variant_b_stats,
            total_samples: total,
            confidence,
            winner,
            improvement,
            is_significant: confidence >= 0.95,
        }
    }

    fn calculate_confidence(stats_a: &VariantStats, stats_b: &VariantStats) -> f32 {
        if stats_a.sample_count == 0 || stats_b.sample_count == 0 {
            return 0.0;
        }

        let diff = (stats_b.mean_score - stats_a.mean_score).abs();
        let pooled_se = (stats_a.standard_error().powi(2) 
                       + stats_b.standard_error().powi(2)).sqrt();
        
        if pooled_se == 0.0 {
            return 1.0;
        }

        let z = diff / pooled_se;
        Self::normal_cdf(z)
    }

    fn normal_cdf(z: f32) -> f32 {
        let a1 =  0.254829592;
        let a2 = -0.284496736;
        let a3 =  1.421413741;
        let a4 = -1.453152027;
        let a5 =  1.061405429;
        let p  =  0.3275911;

        let sign = if z < 0.0 { -1.0 } else { 1.0 };
        let z = z.abs();

        let t = 1.0 / (1.0 + p * z);
        let y = 1.0 - (((((a5 * t + a4) * t) + a3) * t + a2) * t + a1) * t * (-z * z).exp();

        0.5 * (1.0 + sign * y)
    }
}
