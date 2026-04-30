//! # BizClaw AGI Loop Engine
//! 
//! Fully autonomous AGI loop implementation giống AGNT.
//! 
//! ## Cycle: Execute → Evaluate → Replan
//! 
//! Features:
//! - AGI Loop với configurable iterations
//! - World-state tracking & snapshots
//! - Skill Evolution Score (SES)
//! - A/B Experiments framework

pub mod loop_engine;
pub mod world_state;
pub mod evaluator;
pub mod replanner;
pub mod ses;
pub mod experiments;

pub use loop_engine::{AgiLoop, AgiLoopConfig, AgiLoopEvent};
pub use world_state::{WorldState, StateSnapshot};
pub use evaluator::{Evaluator, EvaluationResult, EvaluationCriteria};
pub use replanner::{Replanner, ReplanSuggestion};
pub use ses::{SkillEvolutionScore, SesMetrics};
pub use experiments::{Experiment, ExperimentRunner};
