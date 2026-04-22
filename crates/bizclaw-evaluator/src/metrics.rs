//! Evaluation metrics calculation

use crate::EvaluationResult;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MetricType {
    PassRate,
    AverageScore,
    Precision,
    Recall,
    F1Score,
    Latency,
    Throughput,
    StochasticVariance,
}

impl MetricType {
    pub fn name(&self) -> &str {
        match self {
            MetricType::PassRate => "pass_rate",
            MetricType::AverageScore => "average_score",
            MetricType::Precision => "precision",
            MetricType::Recall => "recall",
            MetricType::F1Score => "f1_score",
            MetricType::Latency => "latency",
            MetricType::Throughput => "throughput",
            MetricType::StochasticVariance => "stochastic_variance",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Threshold {
    pub metric: MetricType,
    pub value: f32,
    pub comparison: ThresholdComparison,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThresholdComparison {
    GreaterThan,
    LessThan,
    GreaterOrEqual,
    LessOrEqual,
    Equal,
}

impl Threshold {
    pub fn is_met(&self, value: f32) -> bool {
        match self.comparison {
            ThresholdComparison::GreaterThan => value > self.value,
            ThresholdComparison::LessThan => value < self.value,
            ThresholdComparison::GreaterOrEqual => value >= self.value,
            ThresholdComparison::LessOrEqual => value <= self.value,
            ThresholdComparison::Equal => (value - self.value).abs() < 0.001,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metrics {
    pub pass_rate: f32,
    pub average_score: f32,
    pub precision: Option<f32>,
    pub recall: Option<f32>,
    pub f1_score: Option<f32>,
    pub average_latency_ms: f32,
    pub throughput: f32,
    pub stochastic_variance: Option<f32>,
    pub score_distribution: HashMap<String, usize>,
    pub thresholds_met: HashMap<String, bool>,
}

impl Metrics {
    pub fn calculate(results: &[EvaluationResult]) -> Self {
        let total = results.len();
        if total == 0 {
            return Self::empty();
        }

        let pass_count = results.iter().filter(|r| r.pass).count();
        let pass_rate = pass_count as f32 / total as f32;

        let total_score: f32 = results.iter()
            .map(|r| r.judgment.overall_score)
            .sum();
        let average_score = total_score / total as f32;

        let total_latency: u64 = results.iter()
            .map(|r| r.latency_ms)
            .sum();
        let average_latency = total_latency as f32 / total as f32;

        let mut score_distribution: HashMap<String, usize> = HashMap::new();
        for result in results {
            let bucket = match result.judgment.overall_score {
                s if s >= 0.9 => "90-100".to_string(),
                s if s >= 0.7 => "70-89".to_string(),
                s if s >= 0.5 => "50-69".to_string(),
                s if s >= 0.3 => "30-49".to_string(),
                _ => "0-29".to_string(),
            };
            *score_distribution.entry(bucket).or_insert(0) += 1;
        }

        let stochastic_variance = Self::calculate_stochastic_variance(results);

        Self {
            pass_rate,
            average_score,
            precision: None,
            recall: None,
            f1_score: None,
            average_latency_ms: average_latency,
            throughput: 1000.0 / average_latency.max(1.0),
            stochastic_variance,
            score_distribution,
            thresholds_met: HashMap::new(),
        }
    }

    fn calculate_stochastic_variance(results: &[EvaluationResult]) -> Option<f32> {
        let stochastic_results: Vec<_> = results.iter()
            .filter_map(|r| r.stochastic_scores.as_ref())
            .collect();

        if stochastic_results.is_empty() {
            return None;
        }

        let variances: Vec<f32> = stochastic_results.iter()
            .map(|scores| {
                let mean = scores.iter().sum::<f32>() / scores.len() as f32;
                scores.iter()
                    .map(|s| (s - mean).powi(2))
                    .sum::<f32>() / scores.len() as f32
            })
            .collect();

        Some(variances.iter().sum::<f32>() / variances.len() as f32)
    }

    fn empty() -> Self {
        Self {
            pass_rate: 0.0,
            average_score: 0.0,
            precision: None,
            recall: None,
            f1_score: None,
            average_latency_ms: 0.0,
            throughput: 0.0,
            stochastic_variance: None,
            score_distribution: HashMap::new(),
            thresholds_met: HashMap::new(),
        }
    }

    pub fn with_thresholds(mut self, thresholds: &[Threshold]) -> Self {
        for threshold in thresholds {
            let value = match threshold.metric {
                MetricType::PassRate => self.pass_rate,
                MetricType::AverageScore => self.average_score,
                MetricType::Precision => self.precision.unwrap_or(0.0),
                MetricType::Recall => self.recall.unwrap_or(0.0),
                MetricType::F1Score => self.f1_score.unwrap_or(0.0),
                MetricType::Latency => self.average_latency_ms,
                MetricType::Throughput => self.throughput,
                MetricType::StochasticVariance => self.stochastic_variance.unwrap_or(0.0),
            };

            self.thresholds_met.insert(
                threshold.metric.name().to_string(),
                threshold.is_met(value)
            );
        }
        self
    }

    pub fn is_healthy(&self) -> bool {
        self.pass_rate >= 0.7 && self.average_score >= 0.7
    }

    pub fn summary(&self) -> String {
        format!(
            "Pass Rate: {:.1}% | Avg Score: {:.2} | Latency: {:.0}ms | Throughput: {:.1}/s",
            self.pass_rate * 100.0,
            self.average_score,
            self.average_latency_ms,
            self.throughput
        )
    }
}

pub mod presets {
    use super::*;

    pub fn default_thresholds() -> Vec<Threshold> {
        vec![
            Threshold {
                metric: MetricType::PassRate,
                value: 0.7,
                comparison: ThresholdComparison::GreaterOrEqual,
            },
            Threshold {
                metric: MetricType::AverageScore,
                value: 0.7,
                comparison: ThresholdComparison::GreaterOrEqual,
            },
            Threshold {
                metric: MetricType::Latency,
                value: 5000.0,
                comparison: ThresholdComparison::LessThan,
            },
            Threshold {
                metric: MetricType::StochasticVariance,
                value: 0.1,
                comparison: ThresholdComparison::LessThan,
            },
        ]
    }

    pub fn strict_thresholds() -> Vec<Threshold> {
        vec![
            Threshold {
                metric: MetricType::PassRate,
                value: 0.9,
                comparison: ThresholdComparison::GreaterOrEqual,
            },
            Threshold {
                metric: MetricType::AverageScore,
                value: 0.85,
                comparison: ThresholdComparison::GreaterOrEqual,
            },
            Threshold {
                metric: MetricType::Latency,
                value: 2000.0,
                comparison: ThresholdComparison::LessThan,
            },
            Threshold {
                metric: MetricType::StochasticVariance,
                value: 0.05,
                comparison: ThresholdComparison::LessThan,
            },
        ]
    }
}
