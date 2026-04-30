//! A/B Experiments Framework
//! 
//! Giống AGNT experiments system.
//! Cho phép so sánh skill versions, benchmarks, và regression tests.

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use uuid::Uuid;

use crate::evaluator::EvaluationResult;

/// An experiment definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Experiment {
    pub id: String,
    pub name: String,
    pub description: String,
    pub experiment_type: ExperimentType,
    pub status: ExperimentStatus,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    
    // Configuration
    pub variants: Vec<Variant>,
    pub metric: String,
    pub min_samples: u32,
    pub confidence_level: f32,
    
    // Results
    pub results: Option<ExperimentResults>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ExperimentType {
    AbTest,           // Compare A vs B
    Benchmark,        // Measure performance
    Regression,       // Check for regressions
    Dataset,          // Eval on dataset
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ExperimentStatus {
    Draft,
    Running,
    Paused,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Variant {
    pub id: String,
    pub name: String,
    pub description: String,
    pub config: HashMap<String, serde_json::Value>,
    pub samples: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentResults {
    pub winner: Option<String>,
    pub confidence: f32,
    pub variant_scores: HashMap<String, VariantScore>,
    pub statistical_significance: bool,
    pub recommendation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariantScore {
    pub variant_id: String,
    pub mean_score: f32,
    pub std_dev: f32,
    pub sample_count: u32,
    pub confidence_interval: (f32, f32),
}

/// A/B Test runner
#[derive(Debug, Clone)]
pub struct ExperimentRunner {
    experiments: HashMap<String, Experiment>,
    config: ExperimentConfig,
}

#[derive(Debug, Clone)]
pub struct ExperimentConfig {
    pub default_confidence: f32,
    pub default_min_samples: u32,
}

impl Default for ExperimentConfig {
    fn default() -> Self {
        Self {
            default_confidence: 0.95,
            default_min_samples: 30,
        }
    }
}

impl ExperimentRunner {
    pub fn new() -> Self {
        Self {
            experiments: HashMap::new(),
            config: ExperimentConfig::default(),
        }
    }

    /// Create a new A/B test experiment
    pub fn create_ab_test(&mut self, name: &str, variant_a: &str, variant_b: &str) -> String {
        let id = Uuid::new_v4().to_string();
        
        let experiment = Experiment {
            id: id.clone(),
            name: name.to_string(),
            description: format!("A/B test: {} vs {}", variant_a, variant_b),
            experiment_type: ExperimentType::AbTest,
            status: ExperimentStatus::Draft,
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
            variants: vec![
                Variant {
                    id: format!("{}_a", id),
                    name: variant_a.to_string(),
                    description: "Control variant".to_string(),
                    config: HashMap::new(),
                    samples: 0,
                },
                Variant {
                    id: format!("{}_b", id),
                    name: variant_b.to_string(),
                    description: "Test variant".to_string(),
                    config: HashMap::new(),
                    samples: 0,
                },
            ],
            metric: "score".to_string(),
            min_samples: self.config.default_min_samples,
            confidence_level: self.config.default_confidence,
            results: None,
        };
        
        self.experiments.insert(id.clone(), experiment);
        id
    }

    /// Create a benchmark experiment
    pub fn create_benchmark(&mut self, name: &str, targets: Vec<String>) -> String {
        let id = Uuid::new_v4().to_string();
        
        let variants: Vec<Variant> = targets
            .into_iter()
            .enumerate()
            .map(|(i, name)| Variant {
                id: format!("{}_{}", id, i),
                name,
                description: format!("Benchmark target {}", i + 1),
                config: HashMap::new(),
                samples: 0,
            })
            .collect();
        
        let experiment = Experiment {
            id: id.clone(),
            name: name.to_string(),
            description: format!("Benchmark: {}", name),
            experiment_type: ExperimentType::Benchmark,
            status: ExperimentStatus::Draft,
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
            variants,
            metric: "throughput".to_string(),
            min_samples: 10,
            confidence_level: 0.9,
            results: None,
        };
        
        self.experiments.insert(id.clone(), experiment);
        id
    }

    /// Start an experiment
    pub fn start(&mut self, experiment_id: &str) -> Result<(), String> {
        let experiment = self.experiments
            .get_mut(experiment_id)
            .ok_or("Experiment not found")?;
        
        if experiment.status != ExperimentStatus::Draft {
            return Err("Experiment already started".to_string());
        }
        
        experiment.status = ExperimentStatus::Running;
        experiment.started_at = Some(Utc::now());
        
        Ok(())
    }

    /// Record a result for a variant
    pub fn record_result(&mut self, experiment_id: &str, variant_id: &str, _result: f32) -> Result<(), String> {
        let experiment = self.experiments
            .get_mut(experiment_id)
            .ok_or("Experiment not found")?;
        
        if experiment.status != ExperimentStatus::Running {
            return Err("Experiment not running".to_string());
        }
        
        let variant = experiment.variants
            .iter_mut()
            .find(|v| v.id == variant_id)
            .ok_or("Variant not found")?;
        
        variant.samples += 1;
        
        // Check if we have enough samples
        if variant.samples >= experiment.min_samples {
            self.check_completion(experiment_id)?;
        }
        
        Ok(())
    }

    /// Record an evaluation result
    pub fn record_evaluation(&mut self, experiment_id: &str, variant_id: &str, eval: &EvaluationResult) -> Result<(), String> {
        self.record_result(experiment_id, variant_id, eval.score)
    }

    /// Check if experiment is complete and calculate results
    fn check_completion(&mut self, experiment_id: &str) -> Result<(), String> {
        let experiment = self.experiments
            .get_mut(experiment_id)
            .ok_or("Experiment not found")?;
        
        // Check if all variants have enough samples
        let all_ready = experiment.variants.iter().all(|v| v.samples >= experiment.min_samples);
        
        if !all_ready {
            return Ok(());
        }
        
        // Calculate results
        let results = Self::calculate_results_static(experiment);
        
        experiment.results = Some(results);
        experiment.status = ExperimentStatus::Completed;
        experiment.completed_at = Some(Utc::now());
        
        Ok(())
    }

    fn calculate_results_static(experiment: &Experiment) -> ExperimentResults {
        let mut variant_scores = HashMap::new();
        
        // For simplicity, we'll use stored sample counts
        // In real impl, would store actual results and compute statistics
        for variant in &experiment.variants {
            let score = (variant.samples as f32 / 100.0).min(1.0); // Placeholder
            variant_scores.insert(variant.id.clone(), VariantScore {
                variant_id: variant.id.clone(),
                mean_score: score,
                std_dev: 0.1,
                sample_count: variant.samples,
                confidence_interval: (score - 0.1, score + 0.1),
            });
        }
        
        // Find winner
        let winner = variant_scores
            .values()
            .max_by(|a, b| a.mean_score.partial_cmp(&b.mean_score).unwrap())
            .map(|s| s.variant_id.clone());
        
        ExperimentResults {
            winner,
            confidence: 0.95,
            variant_scores,
            statistical_significance: true,
            recommendation: "Based on current data".to_string(),
        }
    }

    /// Get experiment by ID
    pub fn get(&self, experiment_id: &str) -> Option<&Experiment> {
        self.experiments.get(experiment_id)
    }

    /// Get all experiments
    pub fn list(&self) -> Vec<&Experiment> {
        self.experiments.values().collect()
    }

    /// Get running experiments
    pub fn get_running(&self) -> Vec<&Experiment> {
        self.experiments
            .values()
            .filter(|e| e.status == ExperimentStatus::Running)
            .collect()
    }

    /// Pause an experiment
    pub fn pause(&mut self, experiment_id: &str) -> Result<(), String> {
        let experiment = self.experiments
            .get_mut(experiment_id)
            .ok_or("Experiment not found")?;
        
        if experiment.status != ExperimentStatus::Running {
            return Err("Experiment not running".to_string());
        }
        
        experiment.status = ExperimentStatus::Paused;
        Ok(())
    }

    /// Resume a paused experiment
    pub fn resume(&mut self, experiment_id: &str) -> Result<(), String> {
        let experiment = self.experiments
            .get_mut(experiment_id)
            .ok_or("Experiment not found")?;
        
        if experiment.status != ExperimentStatus::Paused {
            return Err("Experiment not paused".to_string());
        }
        
        experiment.status = ExperimentStatus::Running;
        Ok(())
    }
}

impl Default for ExperimentRunner {
    fn default() -> Self {
        Self::new()
    }
}

/// Quick A/B comparison helper
pub fn quick_ab_compare(results_a: &[f32], results_b: &[f32]) -> AbComparisonResult {
    let mean_a = if results_a.is_empty() { 0.0 } else { results_a.iter().sum::<f32>() / results_a.len() as f32 };
    let mean_b = if results_b.is_empty() { 0.0 } else { results_b.iter().sum::<f32>() / results_b.len() as f32 };
    
    let winner = if mean_a > mean_b { "A" } else if mean_b > mean_a { "B" } else { "tie" };
    
    AbComparisonResult {
        variant_a_mean: mean_a,
        variant_b_mean: mean_b,
        winner: winner.to_string(),
        difference: (mean_a - mean_b).abs(),
        recommendation: if winner == "A" {
            format!("Variant A wins ({:.2}% better)", (mean_a - mean_b) * 100.0)
        } else if winner == "B" {
            format!("Variant B wins ({:.2}% better)", (mean_b - mean_a) * 100.0)
        } else {
            "No significant difference".to_string()
        },
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbComparisonResult {
    pub variant_a_mean: f32,
    pub variant_b_mean: f32,
    pub winner: String,
    pub difference: f32,
    pub recommendation: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quick_ab_compare() {
        let results_a = vec![0.8, 0.85, 0.75, 0.9];
        let results_b = vec![0.7, 0.75, 0.72, 0.78];
        
        let result = quick_ab_compare(&results_a, &results_b);
        
        assert!(result.winner == "A");
    }

    #[tokio::test]
    async fn test_experiment_runner() {
        let mut runner = ExperimentRunner::new();
        
        let exp_id = runner.create_ab_test("Test", "Control", "Treatment");
        
        runner.start(&exp_id).unwrap();
        
        let exp = runner.get(&exp_id).unwrap();
        assert_eq!(exp.status, ExperimentStatus::Running);
    }
}
