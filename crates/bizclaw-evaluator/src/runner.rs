//! Main evaluation runner

use crate::{GoldenDataset, Judgment, Rubric};
use crate::judge::Judge;
use crate::metrics::Metrics;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluatorConfig {
    pub judge_config: crate::judge::JudgeConfig,
    pub default_rubric: Option<String>,
    pub enable_stochastic_analysis: bool,
    pub run_count: usize,
    pub parallel_samples: usize,
    pub save_results: bool,
    pub results_path: Option<std::path::PathBuf>,
}

impl Default for EvaluatorConfig {
    fn default() -> Self {
        Self {
            judge_config: crate::judge::JudgeConfig::default(),
            default_rubric: None,
            enable_stochastic_analysis: true,
            run_count: 3,
            parallel_samples: 5,
            save_results: true,
            results_path: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationResult {
    pub id: String,
    pub rubric_id: String,
    pub sample_id: String,
    pub input: String,
    pub output: String,
    pub expected: Option<String>,
    pub judgment: Judgment,
    pub pass: bool,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub latency_ms: u64,
    pub stochastic_scores: Option<Vec<f32>>,
}

impl EvaluationResult {
    pub fn new(
        id: &str,
        rubric_id: &str,
        sample_id: &str,
        input: &str,
        output: &str,
        judgment: Judgment,
    ) -> Self {
        let pass = judgment.overall_score >= 0.7;
        Self {
            id: id.to_string(),
            rubric_id: rubric_id.to_string(),
            sample_id: sample_id.to_string(),
            input: input.to_string(),
            output: output.to_string(),
            expected: None,
            judgment,
            pass,
            timestamp: chrono::Utc::now(),
            latency_ms: 0,
            stochastic_scores: None,
        }
    }

    pub fn with_expected(mut self, expected: &str) -> Self {
        self.expected = Some(expected.to_string());
        self
    }

    pub fn with_stochastic_scores(mut self, scores: Vec<f32>) -> Self {
        self.stochastic_scores = Some(scores);
        self
    }
}

pub struct Evaluator {
    config: EvaluatorConfig,
    judge: Judge,
    rubric: Option<Rubric>,
    results: Arc<RwLock<Vec<EvaluationResult>>>,
}

impl Evaluator {
    pub fn new(config: EvaluatorConfig) -> Self {
        let judge = Judge::new(config.judge_config.clone());
        Self {
            config,
            judge,
            rubric: None,
            results: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn with_rubric(mut self, rubric: Rubric) -> Self {
        self.rubric = Some(rubric);
        self
    }

    pub async fn evaluate_single(
        &self,
        input: &str,
        output: &str,
        expected: Option<&str>,
        context: Option<&str>,
    ) -> anyhow::Result<EvaluationResult> {
        let rubric = self.rubric.as_ref()
            .ok_or_else(|| anyhow::anyhow!("No rubric configured"))?;

        let judgment = self.judge.evaluate(
            input,
            output,
            expected,
            context,
            &rubric.criteria,
        ).await?;

        let result = EvaluationResult::new(
            &uuid::Uuid::new_v4().to_string(),
            &rubric.id,
            "single",
            input,
            output,
            judgment.clone(),
        );

        Ok(result)
    }

    pub async fn evaluate_dataset(
        &self,
        dataset: &GoldenDataset,
    ) -> anyhow::Result<EvaluationRun> {
        let rubric = self.rubric.as_ref()
            .ok_or_else(|| anyhow::anyhow!("No rubric configured"))?;

        let mut results = Vec::new();
        let mut pass_count = 0;
        let mut total_score = 0.0;

        for entry in &dataset.entries {
            let judgment = self.judge.evaluate(
                &entry.input,
                "agent_output",
                entry.expected_output.as_deref(),
                entry.context.as_deref(),
                &rubric.criteria,
            ).await?;

            let mut result = EvaluationResult::new(
                &uuid::Uuid::new_v4().to_string(),
                &rubric.id,
                &entry.id,
                &entry.input,
                "agent_output",
                judgment.clone(),
            );

            if self.config.enable_stochastic_analysis && self.config.run_count > 1 {
                let mut scores = Vec::new();
                for _ in 0..self.config.run_count {
                    let j = self.judge.evaluate(
                        &entry.input,
                        "agent_output",
                        entry.expected_output.as_deref(),
                        entry.context.as_deref(),
                        &rubric.criteria,
                    ).await?;
                    scores.push(j.overall_score);
                }
                result = result.with_stochastic_scores(scores);
            }

            if result.pass {
                pass_count += 1;
            }
            total_score += judgment.overall_score;
            results.push(result);
        }

        let avg_score = if !results.is_empty() {
            total_score / results.len() as f32
        } else {
            0.0
        };

        let pass_rate = if !results.is_empty() {
            pass_count as f32 / results.len() as f32
        } else {
            0.0
        };

        let metrics = Metrics::calculate(&results);
        
        let run = EvaluationRun {
            id: uuid::Uuid::new_v4().to_string(),
            dataset_id: dataset.id.clone(),
            rubric_id: rubric.id.clone(),
            timestamp: chrono::Utc::now(),
            total_samples: results.len(),
            pass_count,
            pass_rate,
            average_score: avg_score,
            results,
            metrics,
        };

        {
            let mut stored = self.results.write().await;
            stored.extend(run.results.clone());
        }

        Ok(run)
    }

    pub async fn get_results(&self) -> Vec<EvaluationResult> {
        self.results.read().await.clone()
    }

    pub async fn clear_results(&self) {
        self.results.write().await.clear();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationRun {
    pub id: String,
    pub dataset_id: String,
    pub rubric_id: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub total_samples: usize,
    pub pass_count: usize,
    pub pass_rate: f32,
    pub average_score: f32,
    pub results: Vec<EvaluationResult>,
    pub metrics: Metrics,
}

impl EvaluationRun {
    pub fn summary(&self) -> String {
        format!(
            "Evaluation Run {}:\n\
             Dataset: {}\n\
             Rubric: {}\n\
             Samples: {} | Pass: {} ({:.1}%)\n\
             Average Score: {:.2}\n\
             Latency: {:.0}ms avg",
            self.id,
            self.dataset_id,
            self.rubric_id,
            self.total_samples,
            self.pass_count,
            self.pass_rate * 100.0,
            self.average_score,
            self.results.iter()
                .map(|r| r.latency_ms as u64)
                .sum::<u64>() as f32 / self.total_samples.max(1) as f32
        )
    }
}
