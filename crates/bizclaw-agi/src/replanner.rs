//! Replanner - Tạo lại kế hoạch khi cần
//! 
//! Giống AGNT replanner trong AGI loop.
//! Phân tích evaluation result và tạo suggestions để cải thiện plan.

use serde::{Deserialize, Serialize};

use crate::evaluator::EvaluationResult;

/// Replan suggestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplanSuggestion {
    pub suggestion_type: SuggestionType,
    pub description: String,
    pub priority: Priority,
    pub confidence: f32,
    pub action: ReplanAction,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SuggestionType {
    AddStep,
    RemoveStep,
    ModifyStep,
    ReorderSteps,
    ChangeTool,
    AddValidation,
    AddErrorHandling,
    IncreaseQuality,
    RetryFailed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Priority {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReplanAction {
    AddStep(String),
    RemoveStep(usize),
    ReplaceStep(usize, String),
    InsertAfter(usize, String),
    ModifyTool(usize, String),
}

/// Replanner
pub struct Replanner {
    config: ReplannerConfig,
}

#[derive(Debug, Clone)]
pub struct ReplannerConfig {
    pub max_suggestions: usize,
    pub aggressive_replan: bool,
}

impl Default for ReplannerConfig {
    fn default() -> Self {
        Self {
            max_suggestions: 5,
            aggressive_replan: false,
        }
    }
}

impl Replanner {
    pub fn new() -> Self {
        Self {
            config: ReplannerConfig::default(),
        }
    }

    /// Analyze evaluation result and generate replanning suggestions
    pub async fn replan(
        &self,
        goal: &str,
        eval_result: &EvaluationResult,
        current_plan: Vec<String>,
    ) -> Vec<ReplanSuggestion> {
        let mut suggestions = Vec::new();

        // Analyze quality issues
        if eval_result.quality_score < 0.7 {
            suggestions.push(ReplanSuggestion {
                suggestion_type: SuggestionType::IncreaseQuality,
                description: "Quality score is low, consider adding validation steps".to_string(),
                priority: Priority::High,
                confidence: 0.8,
                action: ReplanAction::InsertAfter(
                    current_plan.len().saturating_sub(1),
                    "validate_output".to_string(),
                ),
            });
        }

        // Analyze completeness issues
        if eval_result.completeness_score < 0.7 {
            suggestions.push(ReplanSuggestion {
                suggestion_type: SuggestionType::AddStep,
                description: "Plan may be incomplete, add more steps".to_string(),
                priority: Priority::High,
                confidence: 0.75,
                action: ReplanAction::AddStep("complete_missing".to_string()),
            });
        }

        // Analyze regression
        if eval_result.regressed {
            suggestions.push(ReplanSuggestion {
                suggestion_type: SuggestionType::RetryFailed,
                description: "Regression detected, revert to previous successful state".to_string(),
                priority: Priority::Critical,
                confidence: 0.9,
                action: ReplanAction::ReplaceStep(
                    current_plan.len().saturating_sub(1),
                    "revert_to_checkpoint".to_string(),
                ),
            });
        }

        // Analyze score trends
        if eval_result.score < 0.5 {
            suggestions.push(ReplanSuggestion {
                suggestion_type: SuggestionType::ModifyStep,
                description: "Score is very low, consider a different approach".to_string(),
                priority: Priority::High,
                confidence: 0.85,
                action: ReplanAction::ReplaceStep(
                    0,
                    format!("new_approach_for_{}", goal.replace(' ', "_")),
                ),
            });
        }

        // Add error handling if errors occurred
        if eval_result.iteration > 1 {
            suggestions.push(ReplanSuggestion {
                suggestion_type: SuggestionType::AddErrorHandling,
                description: "Add error handling for robustness".to_string(),
                priority: Priority::Medium,
                confidence: 0.7,
                action: ReplanAction::InsertAfter(0, "handle_errors".to_string()),
            });
        }

        // Limit suggestions
        suggestions.truncate(self.config.max_suggestions);
        
        // Sort by priority
        suggestions.sort_by(|a, b| {
            let a_priority = match a.priority {
                Priority::Critical => 0,
                Priority::High => 1,
                Priority::Medium => 2,
                Priority::Low => 3,
            };
            let b_priority = match b.priority {
                Priority::Critical => 0,
                Priority::High => 1,
                Priority::Medium => 2,
                Priority::Low => 3,
            };
            a_priority.cmp(&b_priority)
        });

        suggestions
    }

    /// Apply suggestions to current plan
    pub fn apply_suggestions(
        &self,
        mut plan: Vec<String>,
        suggestions: &[ReplanSuggestion],
    ) -> Vec<String> {
        for suggestion in suggestions {
            match &suggestion.action {
                ReplanAction::AddStep(step) => {
                    plan.push(step.clone());
                }
                ReplanAction::InsertAfter(idx, step) => {
                    let insert_idx = (*idx + 1).min(plan.len());
                    plan.insert(insert_idx, step.clone());
                }
                ReplanAction::ReplaceStep(idx, step) => {
                    if *idx < plan.len() {
                        plan[*idx] = step.clone();
                    }
                }
                ReplanAction::RemoveStep(idx) => {
                    if *idx < plan.len() {
                        plan.remove(*idx);
                    }
                }
                ReplanAction::ModifyTool(idx, tool) => {
                    if *idx < plan.len() {
                        plan[*idx] = format!("{}_{}", plan[*idx], tool);
                    }
                }
            }
        }
        
        plan
    }
}

impl Default for Replanner {
    fn default() -> Self {
        Self::new()
    }
}

/// Generate a new plan from scratch based on goal
pub fn generate_plan_from_goal(goal: &str) -> Vec<String> {
    let goal_lower = goal.to_lowercase();
    
    let mut plan = Vec::new();
    
    // Add steps based on goal keywords
    if goal_lower.contains("create") || goal_lower.contains("build") {
        plan.push("research_requirements".to_string());
        plan.push("design_solution".to_string());
        plan.push("implement_core".to_string());
        plan.push("test_solution".to_string());
        plan.push("validate_output".to_string());
    } else if goal_lower.contains("analyze") || goal_lower.contains("review") {
        plan.push("gather_data".to_string());
        plan.push("analyze_patterns".to_string());
        plan.push("generate_insights".to_string());
        plan.push("format_results".to_string());
    } else if goal_lower.contains("fix") || goal_lower.contains("debug") {
        plan.push("identify_issue".to_string());
        plan.push("analyze_root_cause".to_string());
        plan.push("implement_fix".to_string());
        plan.push("test_fix".to_string());
    } else {
        // Generic plan
        plan.push("understand_goal".to_string());
        plan.push("plan_steps".to_string());
        plan.push("execute_plan".to_string());
        plan.push("validate_result".to_string());
    }
    
    plan
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_replan() {
        let replanner = Replanner::new();
        
        let eval = EvaluationResult {
            iteration: 1,
            timestamp: chrono::Utc::now(),
            score: 0.4,
            goal_achieved: false,
            quality_score: 0.3,
            relevance_score: 0.5,
            completeness_score: 0.4,
            regressed: false,
            previous_score: None,
            feedback: "Low score".to_string(),
            criteria_scores: std::collections::HashMap::new(),
        };
        
        let plan = vec![
            "step1".to_string(),
            "step2".to_string(),
        ];
        
        let suggestions = replanner.replan("test goal", &eval, plan).await;
        
        assert!(!suggestions.is_empty());
    }
}
