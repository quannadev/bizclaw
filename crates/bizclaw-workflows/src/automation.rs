//! Workflow Automation Engine
//! 
//! Inspired by OpenHarness patterns:
//! - Auto tool retry với exponential backoff
//! - Workflow definitions với hooks
//! - Parallel tool execution
//! - Workflow registry

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// Workflow step definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    pub name: String,
    pub tool: String,
    pub params: serde_json::Value,
    pub retry: RetryConfig,
    pub hooks: StepHooks,
    pub timeout_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RetryConfig {
    pub max_attempts: u32,
    pub initial_delay_ms: u64,
    pub max_delay_ms: u64,
    pub backoff_multiplier: f64,
}

impl Default for WorkflowStep {
    fn default() -> Self {
        Self {
            name: "step".to_string(),
            tool: String::new(),
            params: serde_json::json!({}),
            retry: RetryConfig::default(),
            hooks: StepHooks::default(),
            timeout_secs: 30,
        }
    }
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay_ms: 1000,
            max_delay_ms: 30000,
            backoff_multiplier: 2.0,
        }
    }
}

/// Pre/Post tool hooks
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StepHooks {
    pub pre_tool: Vec<Hook>,
    pub post_tool: Vec<Hook>,
    pub on_error: Vec<Hook>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hook {
    pub name: String,
    pub hook_type: HookType,
    pub condition: Option<HookCondition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HookType {
    PreTool,
    PostTool,
    OnError,
    OnSuccess,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookCondition {
    pub field: String,
    pub operator: String,
    pub value: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HookType {
    PreTool,
    PostTool,
    OnError,
    OnSuccess,
}

/// Workflow definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    pub id: String,
    pub name: String,
    pub description: String,
    pub steps: Vec<WorkflowStep>,
    pub parallel: bool,
    pub parallel_limit: usize,
}

impl Workflow {
    pub fn new(id: &str, name: &str) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            description: String::new(),
            steps: Vec::new(),
            parallel: false,
            parallel_limit: 4,
        }
    }

    pub fn add_step(mut self, step: WorkflowStep) -> Self {
        self.steps.push(step);
        self
    }
}

/// Workflow executor
pub struct WorkflowExecutor {
    registry: Arc<RwLock<WorkflowRegistry>>,
    hooks: Arc<WorkflowHooks>,
}

#[derive(Default)]
pub struct WorkflowRegistry {
    workflows: HashMap<String, Workflow>,
}

#[derive(Clone)]
pub struct WorkflowHooks {
    pub pre_tool: Vec<Arc<dyn ToolHook + Send + Sync>,
    pub post_tool: Vec<Arc<dyn ToolHook + Send + Sync>>,
}

pub trait ToolHook: Send + Sync {
    fn execute(&self, context: &HookContext) -> HookResult;
}

pub struct HookContext {
    pub tool_name: String,
    pub params: serde_json::Value,
    pub workflow_id: String,
    pub step_name: String,
}

pub struct HookResult {
    pub approved: bool,
    pub modified_params: Option<serde_json::Value>,
    pub message: Option<String>,
}

impl WorkflowExecutor {
    pub fn new() -> Self {
        Self {
            registry: Arc::new(RwLock::new(WorkflowRegistry::default())),
            hooks: Arc::new(WorkflowHooks::default()),
        }
    }

    pub async fn register_workflow(&self, workflow: Workflow) {
        let mut registry = self.registry.write().await;
        registry.workflows.insert(workflow.id.clone(), workflow);
        info!("Registered workflow: {}", workflow.id);
    }

    pub async fn list_workflows(&self) -> Vec<Workflow> {
        let registry = self.registry.read().await;
        registry.workflows.values().cloned().collect()
    }

    pub async fn run_workflow(&self, workflow_id: &str) -> WorkflowResult {
        let workflow = {
            let registry = self.registry.read().await;
            registry.workflows.get(workflow_id).cloned()
        };

        match workflow {
            Some(wf) => self.execute_workflow(&wf).await,
            None => WorkflowResult {
                workflow_id: workflow_id.to_string(),
                success: false,
                output: None,
                error: Some("Workflow not found".to_string()),
                steps_completed: 0,
                total_steps: 0,
            },
        }
    }

    async fn execute_workflow(&self, workflow: &Workflow) -> WorkflowResult {
        info!("Executing workflow: {}", workflow.id);
        let mut steps_completed = 0;
        let mut last_error = None;

        for step in &workflow.steps {
            match self.execute_step(step).await {
                Ok(output) => {
                    steps_completed += 1;
                    debug!("Step {} completed: {}", step.name, output);
                }
                Err(e) => {
                    last_error = Some(e.clone());
                    error!("Step {} failed: {}", step.name, e);
                    if step.retry.max_attempts == 0 {
                        break;
                    }
                }
            }
        }

        WorkflowResult {
            workflow_id: workflow.id.clone(),
            success: last_error.is_none(),
            output: None,
            error: last_error,
            steps_completed,
            total_steps: workflow.steps.len(),
        }
    }

    async fn execute_step(&self, step: &WorkflowStep) -> Result<String, String> {
        // Pre hooks
        for hook in &self.hooks.pre_tool {
            let ctx = HookContext {
                tool_name: step.tool.clone(),
                params: step.params.clone(),
                workflow_id: String::new(),
                step_name: step.name.clone(),
            };
            let result = hook.execute(&ctx);
            if !result.approved {
                return Err(format!("Pre-hook rejected: {:?}", result.message));
            }
        }

        // Execute with retry
        let mut delay_ms = step.retry.initial_delay_ms;
        for attempt in 0..step.retry.max_attempts {
            if attempt > 0 {
                tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
                delay_ms = ((delay_ms as f64 * step.retry.backoff_multiplier) as u64).min(step.retry.max_delay_ms);
            debug!("Attempt {} for step {}", attempt + 1, step.name);
            
            let result = self.execute_tool(&step.tool, &step.params).await;
            match result {
                Ok(output) => {
                    // Post hooks
                    return Ok(output);
                }
                Err(e) => {
                    if attempt + 1 >= step.retry.max_attempts {
                        return Err(format!("Step {} failed after {} attempts: {}", step.name, step.retry.max_attempts, e);
                    }
                    warn!("Retry {} failed: {}", step.name, e);
                }
            }
        }
        Err("Max retries exceeded".to_string())
    }

    async fn execute_tool(&self, tool: &str, _params: &serde_json::Value) -> Result<String, String> {
        // Placeholder - integrate with bizclaw-tools registry
        Ok(format!("Tool {} executed", tool))
    }
}

pub struct WorkflowResult {
    pub workflow_id: String,
    pub success: bool,
    pub output: Option<String>,
    pub error: Option<String>,
    pub steps_completed: usize,
    pub total_steps: usize,
}
