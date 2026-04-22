//! # BizClaw Agent Evaluator
//!
//! Comprehensive agent evaluation framework for BizClaw.
//!
//! ## Features
//! - **Rubric-based Scoring**: Define custom rubrics with multiple criteria
//! - **LLM-as-Judge**: Use another LLM to evaluate agent outputs
//! - **Golden Datasets**: Store expected inputs/outputs for regression testing
//! - **A/B Evaluation**: Compare two agent versions on real traffic
//! - **Stochastic Detection**: Handle non-deterministic agent behavior
//! - **Trend Tracking**: Monitor quality drift over time
//!
//! ## Architecture
//! ```text
//! bizclaw-evaluator/
//! ├── rubric.rs       # Scoring rubrics and criteria
//! ├── judge.rs        # LLM-as-judge implementation
//! ├── dataset.rs      # Golden dataset management
//! ├── ab_test.rs      # A/B evaluation runner
//! ├── runner.rs       # Main evaluation engine
//! └── report.rs       # Report generation
//! ```

pub mod rubric;
pub mod judge;
pub mod dataset;
pub mod ab_test;
pub mod runner;
pub mod report;
pub mod metrics;

pub use rubric::{Rubric, RubricCriteria, ScoringMethod, Score};
pub use judge::{Judge, JudgeConfig, JudgeModel, Judgment};
pub use dataset::{GoldenDataset, DatasetEntry, DatasetStats};
pub use ab_test::{ABTest, ABVariant, ABResult, TrafficSplit};
pub use runner::{Evaluator, EvaluatorConfig, EvaluationResult, EvaluationRun};
pub use report::{Report, ReportFormat, TrendAnalysis};
pub use metrics::{Metrics, MetricType, Threshold};
