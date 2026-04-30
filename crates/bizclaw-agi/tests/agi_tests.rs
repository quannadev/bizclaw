//! Integration tests cho bizclaw-agi crate
//! 
//! Test đầy đủ các tính năng AGI Loop:
//! - AGI Loop execution
//! - World-state tracking & snapshots
//! - Evaluator
//! - Replanner
//! - SES (Skill Evolution Score)
//! - A/B Experiments

use bizclaw_agi::{
    AgiLoop, AgiLoopConfig,
    WorldState, StateSnapshot,
    Evaluator, EvaluationResult,
    Replanner, ReplanSuggestion,
    SkillEvolutionScore,
    ExperimentRunner,
};
use bizclaw_agi::loop_engine::AgiLoopResult;
use chrono::Utc;
use std::collections::HashMap;

// ============================================================================
// AGI Loop Tests
// ============================================================================

#[tokio::test]
async fn test_agi_loop_initialization() {
    let loop_engine = AgiLoop::new(AgiLoopConfig::default());
    
    let state = loop_engine.get_state().await;
    assert_eq!(state.iteration, 0);
    assert!(state.current_plan.is_empty());
    assert_eq!(state.error_count, 0);
}

#[tokio::test]
async fn test_agi_loop_basic_execution() {
    let loop_engine = AgiLoop::new(AgiLoopConfig {
        max_iterations: 3,
        eval_threshold: 0.9,
        ..Default::default()
    });

    let plan = vec![
        "step_1".to_string(),
        "step_2".to_string(),
    ];

    let result = loop_engine.run("test goal", plan).await;
    
    match result {
        AgiLoopResult::MaxIterationsReached(iterations) => {
            assert!(iterations >= 3);
        }
        AgiLoopResult::GoalReached(data) => {
            assert!(data.iterations >= 1);
            assert!(data.final_score >= 0.0);
        }
        _ => {}
    }
}

#[tokio::test]
async fn test_agi_loop_reset() {
    let loop_engine = AgiLoop::new(AgiLoopConfig::default());
    
    // Run a few iterations
    let _ = loop_engine.run("test", vec!["step1".to_string()]).await;
    
    // Reset
    loop_engine.reset().await;
    
    let state = loop_engine.get_state().await;
    assert_eq!(state.iteration, 0);
    assert!(state.execution_history.is_empty());
}

// ============================================================================
// World State Tests
// ============================================================================

#[test]
fn test_world_state_creation() {
    let world = WorldState::new();
    assert!(world.data.entities.is_empty());
    assert!(world.snapshots.is_empty());
    assert!(world.history.is_empty());
}

#[test]
fn test_world_state_snapshot_and_revert() {
    let mut world = WorldState::new();
    
    // Add some state
    world.set_variable("var1", serde_json::json!("value1"));
    
    // Take snapshot
    let snapshot_id = world.save_snapshot();
    assert_eq!(world.snapshots.len(), 1);
    
    // Modify state
    world.set_variable("var2", serde_json::json!("value2"));
    assert_eq!(world.get_variable("var2"), Some(&serde_json::json!("value2")));
    
    // Revert to snapshot
    world.revert_to(&snapshot_id).unwrap();
    assert!(world.get_variable("var2").is_none());
    assert_eq!(world.get_variable("var1"), Some(&serde_json::json!("value1")));
}

#[test]
fn test_world_state_revert_to_nonexistent() {
    let mut world = WorldState::new();
    
    let result = world.revert_to("nonexistent_id");
    assert!(result.is_err());
    
    let result = world.revert_to_iteration(999);
    assert!(result.is_err());
}

#[test]
fn test_world_state_change_tracking() {
    let mut world = WorldState::new();
    
    // Create entity
    let entity = bizclaw_agi::world_state::Entity {
        id: "track_test".to_string(),
        entity_type: "test".to_string(),
        properties: HashMap::new(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    world.update_entity(entity);
    assert_eq!(world.history.len(), 1);
    assert_eq!(world.history[0].change_type, bizclaw_agi::world_state::ChangeType::Created);
}

// ============================================================================
// Evaluator Tests
// ============================================================================

#[tokio::test]
async fn test_evaluator_basic() {
    let evaluator = Evaluator::new();
    
    let snapshot = StateSnapshot {
        id: "test".to_string(),
        iteration: 1,
        timestamp: Utc::now(),
        state: bizclaw_agi::world_state::WorldStateData::default(),
        diff_from_previous: Vec::new(),
    };
    
    let result = evaluator.evaluate("test goal", &snapshot).await;
    
    assert!(result.score >= 0.0 && result.score <= 1.0);
    assert!(!result.feedback.is_empty());
}

#[tokio::test]
async fn test_evaluator_with_history() {
    let evaluator = Evaluator::new();
    
    let snapshot = StateSnapshot {
        id: "test".to_string(),
        iteration: 2,
        timestamp: Utc::now(),
        state: bizclaw_agi::world_state::WorldStateData::default(),
        diff_from_previous: Vec::new(),
    };
    
    let result = evaluator.evaluate_with_history("test goal", &snapshot, Some(0.8)).await;
    
    assert_eq!(result.previous_score, Some(0.8));
}

// ============================================================================
// Replanner Tests
// ============================================================================

#[tokio::test]
async fn test_replanner_low_quality() {
    let replanner = Replanner::new();
    
    let eval = EvaluationResult {
        iteration: 1,
        timestamp: Utc::now(),
        score: 0.3,
        goal_achieved: false,
        quality_score: 0.2,
        relevance_score: 0.4,
        completeness_score: 0.3,
        regressed: false,
        previous_score: None,
        feedback: "Low quality".to_string(),
        criteria_scores: HashMap::new(),
    };
    
    let plan = vec!["step1".to_string()];
    
    let suggestions = replanner.replan("test goal", &eval, plan).await;
    
    // Should have suggestions for low quality
    assert!(!suggestions.is_empty());
}

#[tokio::test]
async fn test_replanner_regression() {
    let replanner = Replanner::new();
    
    let eval = EvaluationResult {
        iteration: 2,
        timestamp: Utc::now(),
        score: 0.5,
        goal_achieved: false,
        quality_score: 0.5,
        relevance_score: 0.5,
        completeness_score: 0.5,
        regressed: true,
        previous_score: Some(0.7),
        feedback: "Regression detected".to_string(),
        criteria_scores: HashMap::new(),
    };
    
    let plan = vec!["step1".to_string(), "step2".to_string()];
    
    let suggestions = replanner.replan("test goal", &eval, plan).await;
    
    // Should have retry suggestion for regression (Critical priority)
    assert!(suggestions.iter().any(|s| {
        matches!(s.priority, bizclaw_agi::replanner::Priority::Critical)
    }));
}

#[test]
fn test_replanner_apply_add_step() {
    let replanner = Replanner::new();
    
    let suggestions = vec![ReplanSuggestion {
        suggestion_type: bizclaw_agi::replanner::SuggestionType::AddStep,
        description: "Add new step".to_string(),
        priority: bizclaw_agi::replanner::Priority::Medium,
        confidence: 0.8,
        action: bizclaw_agi::replanner::ReplanAction::AddStep("new_step".to_string()),
    }];
    
    let plan = vec!["step1".to_string(), "step2".to_string()];
    let new_plan = replanner.apply_suggestions(plan.clone(), &suggestions);
    
    assert_eq!(new_plan.len(), 3);
    assert_eq!(new_plan[2], "new_step");
}

#[test]
fn test_replanner_apply_insert_after() {
    let replanner = Replanner::new();
    
    let suggestions = vec![ReplanSuggestion {
        suggestion_type: bizclaw_agi::replanner::SuggestionType::AddStep,
        description: "Insert after".to_string(),
        priority: bizclaw_agi::replanner::Priority::Medium,
        confidence: 0.8,
        action: bizclaw_agi::replanner::ReplanAction::InsertAfter(0, "inserted".to_string()),
    }];
    
    let plan = vec!["step1".to_string(), "step2".to_string()];
    let new_plan = replanner.apply_suggestions(plan, &suggestions);
    
    assert_eq!(new_plan[1], "inserted");
    assert_eq!(new_plan.len(), 3);
}

#[test]
fn test_replanner_apply_replace_step() {
    let replanner = Replanner::new();
    
    let suggestions = vec![ReplanSuggestion {
        suggestion_type: bizclaw_agi::replanner::SuggestionType::ModifyStep,
        description: "Replace step".to_string(),
        priority: bizclaw_agi::replanner::Priority::High,
        confidence: 0.9,
        action: bizclaw_agi::replanner::ReplanAction::ReplaceStep(1, "replaced".to_string()),
    }];
    
    let plan = vec!["step1".to_string(), "step2".to_string()];
    let new_plan = replanner.apply_suggestions(plan, &suggestions);
    
    assert_eq!(new_plan[1], "replaced");
}

#[test]
fn test_replanner_apply_remove_step() {
    let replanner = Replanner::new();
    
    let suggestions = vec![ReplanSuggestion {
        suggestion_type: bizclaw_agi::replanner::SuggestionType::RemoveStep,
        description: "Remove step".to_string(),
        priority: bizclaw_agi::replanner::Priority::Medium,
        confidence: 0.7,
        action: bizclaw_agi::replanner::ReplanAction::RemoveStep(0),
    }];
    
    let plan = vec!["step1".to_string(), "step2".to_string()];
    let new_plan = replanner.apply_suggestions(plan, &suggestions);
    
    assert_eq!(new_plan.len(), 1);
    assert_eq!(new_plan[0], "step2");
}

#[test]
fn test_generate_plan_from_goal_create() {
    let plan = bizclaw_agi::replanner::generate_plan_from_goal("create a web app");
    
    assert!(plan.contains(&"research_requirements".to_string()));
    assert!(plan.contains(&"design_solution".to_string()));
    assert!(plan.contains(&"implement_core".to_string()));
}

#[test]
fn test_generate_plan_from_goal_analyze() {
    let plan = bizclaw_agi::replanner::generate_plan_from_goal("analyze the data");
    
    assert!(plan.contains(&"gather_data".to_string()));
    assert!(plan.contains(&"analyze_patterns".to_string()));
    assert!(plan.contains(&"generate_insights".to_string()));
}

#[test]
fn test_generate_plan_from_goal_fix() {
    let plan = bizclaw_agi::replanner::generate_plan_from_goal("fix the bug");
    
    assert!(plan.contains(&"identify_issue".to_string()));
    assert!(plan.contains(&"analyze_root_cause".to_string()));
    assert!(plan.contains(&"implement_fix".to_string()));
}

// ============================================================================
// SES (Skill Evolution Score) Tests
// ============================================================================

#[tokio::test]
async fn test_ses_initialization() {
    let ses = SkillEvolutionScore::new();
    
    let score = ses.get_ses_score("nonexistent").await;
    assert!(score.is_none());
}

#[tokio::test]
async fn test_ses_record_iteration() {
    let ses = SkillEvolutionScore::new();
    
    let eval = EvaluationResult {
        iteration: 1,
        timestamp: Utc::now(),
        score: 0.85,
        goal_achieved: true,
        quality_score: 0.8,
        relevance_score: 0.9,
        completeness_score: 0.85,
        regressed: false,
        previous_score: None,
        feedback: "Good".to_string(),
        criteria_scores: HashMap::new(),
    };
    
    ses.record_iteration(1, &eval).await;
    
    let score = ses.get_ses_score("skill_iteration_1").await;
    assert!(score.is_some());
    assert!(score.unwrap() > 0.0);
}

#[tokio::test]
async fn test_ses_multiple_iterations() {
    let ses = SkillEvolutionScore::new();
    let skill_id = "test_skill";
    
    for i in 1..=5 {
        let eval = EvaluationResult {
            iteration: i,
            timestamp: Utc::now(),
            score: 0.5 + (i as f32 * 0.1), // Improving score
            goal_achieved: i >= 4,
            quality_score: 0.6,
            relevance_score: 0.7,
            completeness_score: 0.6,
            regressed: false,
            previous_score: None,
            feedback: format!("Iteration {}", i),
            criteria_scores: HashMap::new(),
        };
        
        ses.record_execution(skill_id, &eval, i >= 4).await;
    }
    
    let history = ses.get_metrics_history(skill_id).await;
    assert!(history.is_some());
    assert_eq!(history.unwrap().len(), 5);
}

#[tokio::test]
async fn test_ses_compare_versions() {
    let ses = SkillEvolutionScore::new();
    
    // Record for version A
    let eval_a = EvaluationResult {
        iteration: 1,
        timestamp: Utc::now(),
        score: 0.9,
        goal_achieved: true,
        quality_score: 0.9,
        relevance_score: 0.9,
        completeness_score: 0.9,
        regressed: false,
        previous_score: None,
        feedback: "Good".to_string(),
        criteria_scores: HashMap::new(),
    };
    ses.record_execution("version_a", &eval_a, true).await;
    
    // Record for version B
    let eval_b = EvaluationResult {
        iteration: 1,
        timestamp: Utc::now(),
        score: 0.7,
        goal_achieved: false,
        quality_score: 0.7,
        relevance_score: 0.7,
        completeness_score: 0.7,
        regressed: false,
        previous_score: None,
        feedback: "Okay".to_string(),
        criteria_scores: HashMap::new(),
    };
    ses.record_execution("version_b", &eval_b, false).await;
    
    let comparison = ses.compare_versions("version_a", "version_b").await;
    
    assert_eq!(comparison.winner, "version_a");
    assert!(comparison.score_a > comparison.score_b);
}

#[tokio::test]
async fn test_ses_ranked_skills() {
    let ses = SkillEvolutionScore::new();
    
    // Record multiple skills
    for i in 1..=3 {
        let eval = EvaluationResult {
            iteration: 1,
            timestamp: Utc::now(),
            score: 0.5 + (i as f32 * 0.2),
            goal_achieved: i == 3,
            quality_score: 0.7,
            relevance_score: 0.7,
            completeness_score: 0.7,
            regressed: false,
            previous_score: None,
            feedback: format!("Skill {}", i),
            criteria_scores: HashMap::new(),
        };
        
        ses.record_execution(&format!("skill_{}", i), &eval, i == 3).await;
    }
    
    let ranked = ses.get_ranked_skills().await;
    
    // Should be sorted by score descending
    if ranked.len() >= 2 {
        for window in ranked.windows(2) {
            assert!(window[0].1 >= window[1].1);
        }
    }
}

// ============================================================================
// A/B Experiments Tests
// ============================================================================

#[test]
fn test_experiment_runner_creation() {
    let mut runner = ExperimentRunner::new();
    
    let exp_id = runner.create_ab_test("Test Experiment", "Control", "Treatment");
    
    assert!(!exp_id.is_empty());
    
    let exp = runner.get(&exp_id);
    assert!(exp.is_some());
    assert_eq!(exp.unwrap().status, bizclaw_agi::experiments::ExperimentStatus::Draft);
}

#[test]
fn test_experiment_runner_start() {
    let mut runner = ExperimentRunner::new();
    
    let exp_id = runner.create_ab_test("Test", "A", "B");
    
    runner.start(&exp_id).unwrap();
    
    let exp = runner.get(&exp_id).unwrap();
    assert_eq!(exp.status, bizclaw_agi::experiments::ExperimentStatus::Running);
    assert!(exp.started_at.is_some());
}

#[test]
fn test_experiment_runner_pause_resume() {
    let mut runner = ExperimentRunner::new();
    
    let exp_id = runner.create_ab_test("Test", "A", "B");
    runner.start(&exp_id).unwrap();
    
    runner.pause(&exp_id).unwrap();
    let exp = runner.get(&exp_id).unwrap();
    assert_eq!(exp.status, bizclaw_agi::experiments::ExperimentStatus::Paused);
    
    runner.resume(&exp_id).unwrap();
    let exp = runner.get(&exp_id).unwrap();
    assert_eq!(exp.status, bizclaw_agi::experiments::ExperimentStatus::Running);
}

#[test]
fn test_experiment_runner_invalid_operations() {
    let mut runner = ExperimentRunner::new();
    
    // Start non-existent experiment
    let result = runner.start("nonexistent");
    assert!(result.is_err());
    
    let exp_id = runner.create_ab_test("Test", "A", "B");
    
    // Pause not-running experiment
    let result = runner.pause(&exp_id);
    assert!(result.is_err());
    
    // Start already-started experiment
    runner.start(&exp_id).unwrap();
    let result = runner.start(&exp_id);
    assert!(result.is_err());
}

#[test]
fn test_experiment_runner_list_operations() {
    let mut runner = ExperimentRunner::new();
    
    runner.create_ab_test("Test 1", "A", "B");
    runner.create_ab_test("Test 2", "C", "D");
    
    let all = runner.list();
    assert_eq!(all.len(), 2);
    
    let running = runner.get_running();
    assert_eq!(running.len(), 0); // None started yet
}

#[test]
fn test_benchmark_creation() {
    let mut runner = ExperimentRunner::new();
    
    let exp_id = runner.create_benchmark(
        "Performance Test",
        vec!["tool_a".to_string(), "tool_b".to_string(), "tool_c".to_string()]
    );
    
    let exp = runner.get(&exp_id).unwrap();
    assert_eq!(exp.experiment_type, bizclaw_agi::experiments::ExperimentType::Benchmark);
    assert_eq!(exp.variants.len(), 3);
}

#[test]
fn test_quick_ab_compare_a_wins() {
    let results_a = vec![0.8, 0.85, 0.75, 0.9, 0.82];
    let results_b = vec![0.7, 0.75, 0.72, 0.78, 0.71];
    
    let result = bizclaw_agi::experiments::quick_ab_compare(&results_a, &results_b);
    
    assert_eq!(result.winner, "A");
    assert!(result.difference > 0.0);
    assert!(result.recommendation.contains("A wins"));
}

#[test]
fn test_quick_ab_compare_b_wins() {
    let results_a = vec![0.6, 0.65, 0.62, 0.68];
    let results_b = vec![0.8, 0.85, 0.82, 0.88];
    
    let result = bizclaw_agi::experiments::quick_ab_compare(&results_a, &results_b);
    
    assert_eq!(result.winner, "B");
}

#[test]
fn test_quick_ab_compare_tie() {
    let results_a = vec![0.75, 0.75, 0.75, 0.75];
    let results_b = vec![0.75, 0.75, 0.75, 0.75];
    
    let result = bizclaw_agi::experiments::quick_ab_compare(&results_a, &results_b);
    
    assert_eq!(result.winner, "tie");
    assert_eq!(result.difference, 0.0);
}

#[test]
fn test_quick_ab_compare_empty() {
    let results_a: Vec<f32> = vec![];
    let results_b = vec![0.8, 0.9];
    
    let result = bizclaw_agi::experiments::quick_ab_compare(&results_a, &results_b);
    
    // Empty should result in 0.0 mean, B should win
    assert_eq!(result.variant_a_mean, 0.0);
    assert_eq!(result.winner, "B");
}

// ============================================================================
// Serialization Tests
// ============================================================================

#[test]
fn test_state_snapshot_serialization() {
    let snapshot = StateSnapshot {
        id: "test_id".to_string(),
        iteration: 1,
        timestamp: Utc::now(),
        state: bizclaw_agi::world_state::WorldStateData::default(),
        diff_from_previous: Vec::new(),
    };
    
    let json = serde_json::to_string(&snapshot).unwrap();
    let deserialized: StateSnapshot = serde_json::from_str(&json).unwrap();
    
    assert_eq!(deserialized.id, snapshot.id);
    assert_eq!(deserialized.iteration, snapshot.iteration);
}

#[test]
fn test_evaluation_result_serialization() {
    let eval = EvaluationResult {
        iteration: 1,
        timestamp: Utc::now(),
        score: 0.85,
        goal_achieved: true,
        quality_score: 0.8,
        relevance_score: 0.9,
        completeness_score: 0.85,
        regressed: false,
        previous_score: Some(0.7),
        feedback: "Good progress".to_string(),
        criteria_scores: HashMap::new(),
    };
    
    let json = serde_json::to_string(&eval).unwrap();
    let deserialized: EvaluationResult = serde_json::from_str(&json).unwrap();
    
    assert_eq!(deserialized.score, eval.score);
    assert_eq!(deserialized.goal_achieved, eval.goal_achieved);
}
