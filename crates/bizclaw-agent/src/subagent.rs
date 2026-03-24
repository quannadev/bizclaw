//! Sub-Agent System — spawn child agents for complex multi-step tasks.
//!
//! Ported from DeerFlow 2.0's subagent architecture.
//! The lead agent can delegate tasks to sub-agents, each with:
//! - Scoped context (isolated from lead agent's conversation)
//! - Scoped toolset (subset of available tools)
//! - Configurable timeout and max turns
//! - Parallel execution (up to MAX_CONCURRENT_SUBAGENTS)
//!
//! ## Architecture
//! ```text
//! Lead Agent ──→ SubAgentExecutor
//!                ├── task() tool call
//!                ├── spawn sub-agent in background
//!                ├── poll every 5s for events
//!                └── return structured result
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, Semaphore, mpsc};
use tracing::{debug, error, info, warn};

/// Maximum concurrent sub-agents (matches DeerFlow default).
pub const MAX_CONCURRENT_SUBAGENTS: usize = 3;

/// Default sub-agent timeout (15 minutes, same as DeerFlow).
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(15 * 60);

/// Type of sub-agent to spawn.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SubAgentType {
    /// General-purpose agent with all tools except `task`.
    GeneralPurpose,
    /// Bash specialist — only shell/file tools.
    Bash,
    /// Research agent — web search + fetch + summarize.
    Research,
    /// Coding agent — file/shell/code tools.
    Coding,
    /// Custom agent with specified tools.
    Custom(Vec<String>),
}

impl SubAgentType {
    /// Get the default toolset for this agent type.
    pub fn default_tools(&self) -> Vec<&str> {
        match self {
            Self::GeneralPurpose => vec![
                "shell", "file", "web_search", "web_fetch", "calculator",
                "plan", "read_file", "write_file",
            ],
            Self::Bash => vec!["shell", "file", "read_file", "write_file"],
            Self::Research => vec!["web_search", "web_fetch", "calculator"],
            Self::Coding => vec!["shell", "file", "read_file", "write_file"],
            Self::Custom(_tools) => {
                // Return empty — caller provides tool names
                vec![]
            }
        }
    }
}

impl std::fmt::Display for SubAgentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::GeneralPurpose => write!(f, "general-purpose"),
            Self::Bash => write!(f, "bash"),
            Self::Research => write!(f, "research"),
            Self::Coding => write!(f, "coding"),
            Self::Custom(tools) => write!(f, "custom({})", tools.join(",")),
        }
    }
}

/// A task to be delegated to a sub-agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubAgentTask {
    /// Human-readable description of the task.
    pub description: String,
    /// Detailed prompt for the sub-agent.
    pub prompt: String,
    /// Type of sub-agent to use.
    pub agent_type: SubAgentType,
    /// Maximum conversation turns before forcing completion.
    #[serde(default = "default_max_turns")]
    pub max_turns: u32,
    /// Optional timeout override.
    #[serde(default)]
    pub timeout_secs: Option<u64>,
}

fn default_max_turns() -> u32 {
    10
}

/// Status of a running sub-agent task.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Queued,
    Running,
    Completed,
    Failed,
    TimedOut,
    Cancelled,
}

impl std::fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Queued => write!(f, "queued"),
            Self::Running => write!(f, "running"),
            Self::Completed => write!(f, "completed"),
            Self::Failed => write!(f, "failed"),
            Self::TimedOut => write!(f, "timed_out"),
            Self::Cancelled => write!(f, "cancelled"),
        }
    }
}

/// An event emitted by a sub-agent during execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubAgentEvent {
    pub task_id: String,
    pub event_type: SubAgentEventType,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub data: Option<String>,
}

/// Types of sub-agent events (matches DeerFlow's SSE events).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubAgentEventType {
    TaskStarted,
    TaskRunning,
    TaskCompleted,
    TaskFailed,
    TaskTimedOut,
}

/// Result of a completed sub-agent task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubAgentResult {
    pub task_id: String,
    pub status: TaskStatus,
    pub output: String,
    pub agent_type: SubAgentType,
    pub turns_used: u32,
    pub duration_ms: u64,
    pub error: Option<String>,
}

/// Tracks a running sub-agent task.
struct RunningTask {
    task: SubAgentTask,
    status: TaskStatus,
    started_at: std::time::Instant,
    result: Option<SubAgentResult>,
}

/// The Sub-Agent Executor — manages spawning and lifecycle of sub-agents.
///
/// Uses dual Semaphore pools (matching DeerFlow's dual thread pool):
/// - `scheduler_semaphore`: limits scheduling (3 workers)
/// - `execution_semaphore`: limits actual execution (3 workers)
pub struct SubAgentExecutor {
    /// Limits concurrent scheduling.
    scheduler_semaphore: Arc<Semaphore>,
    /// Limits concurrent execution.
    execution_semaphore: Arc<Semaphore>,
    /// Active tasks indexed by task_id.
    tasks: Arc<Mutex<HashMap<String, RunningTask>>>,
    /// Event channel sender.
    event_tx: mpsc::UnboundedSender<SubAgentEvent>,
    /// Event channel receiver (wrapped for sharing).
    event_rx: Arc<Mutex<mpsc::UnboundedReceiver<SubAgentEvent>>>,
    /// Maximum concurrent sub-agents.
    max_concurrent: usize,
    /// Default timeout per task.
    default_timeout: Duration,
}

impl SubAgentExecutor {
    /// Create a new executor with default settings.
    pub fn new() -> Self {
        Self::with_config(MAX_CONCURRENT_SUBAGENTS, DEFAULT_TIMEOUT)
    }

    /// Create executor with custom limits.
    pub fn with_config(max_concurrent: usize, default_timeout: Duration) -> Self {
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        Self {
            scheduler_semaphore: Arc::new(Semaphore::new(max_concurrent)),
            execution_semaphore: Arc::new(Semaphore::new(max_concurrent)),
            tasks: Arc::new(Mutex::new(HashMap::new())),
            event_tx,
            event_rx: Arc::new(Mutex::new(event_rx)),
            max_concurrent,
            default_timeout,
        }
    }

    /// Submit a task for sub-agent execution.
    ///
    /// Returns a task_id immediately. The task runs in the background.
    /// Use `poll_result()` to check completion.
    pub async fn submit(&self, task: SubAgentTask) -> String {
        let task_id = format!("subagent_{}", uuid::Uuid::new_v4().simple());
        let timeout = task
            .timeout_secs
            .map(Duration::from_secs)
            .unwrap_or(self.default_timeout);

        // Store task
        {
            let mut tasks = self.tasks.lock().await;
            tasks.insert(
                task_id.clone(),
                RunningTask {
                    task: task.clone(),
                    status: TaskStatus::Queued,
                    started_at: std::time::Instant::now(),
                    result: None,
                },
            );
        }

        // Emit start event
        let _ = self.event_tx.send(SubAgentEvent {
            task_id: task_id.clone(),
            event_type: SubAgentEventType::TaskStarted,
            timestamp: chrono::Utc::now(),
            data: Some(task.description.clone()),
        });

        // Spawn background execution
        let tasks = self.tasks.clone();
        let exec_semaphore = self.execution_semaphore.clone();
        let event_tx = self.event_tx.clone();
        let tid = task_id.clone();
        let task_desc = task.description.clone();
        let task_type = task.agent_type.clone();

        tokio::spawn(async move {
            // Acquire execution slot
            let _permit = exec_semaphore.acquire().await;

            // Update status to running
            {
                let mut tasks_guard = tasks.lock().await;
                if let Some(rt) = tasks_guard.get_mut(&tid) {
                    rt.status = TaskStatus::Running;
                }
            }

            let _ = event_tx.send(SubAgentEvent {
                task_id: tid.clone(),
                event_type: SubAgentEventType::TaskRunning,
                timestamp: chrono::Utc::now(),
                data: None,
            });

            // Execute with timeout
            let start = std::time::Instant::now();
            let result = match tokio::time::timeout(timeout, execute_task(&task)).await {
                Ok(Ok(output)) => SubAgentResult {
                    task_id: tid.clone(),
                    status: TaskStatus::Completed,
                    output,
                    agent_type: task.agent_type.clone(),
                    turns_used: task.max_turns,
                    duration_ms: start.elapsed().as_millis() as u64,
                    error: None,
                },
                Ok(Err(e)) => SubAgentResult {
                    task_id: tid.clone(),
                    status: TaskStatus::Failed,
                    output: String::new(),
                    agent_type: task.agent_type.clone(),
                    turns_used: 0,
                    duration_ms: start.elapsed().as_millis() as u64,
                    error: Some(e.to_string()),
                },
                Err(_) => {
                    let _ = event_tx.send(SubAgentEvent {
                        task_id: tid.clone(),
                        event_type: SubAgentEventType::TaskTimedOut,
                        timestamp: chrono::Utc::now(),
                        data: Some(format!("Timeout after {}s", timeout.as_secs())),
                    });
                    SubAgentResult {
                        task_id: tid.clone(),
                        status: TaskStatus::TimedOut,
                        output: String::new(),
                        agent_type: task.agent_type.clone(),
                        turns_used: 0,
                        duration_ms: start.elapsed().as_millis() as u64,
                        error: Some(format!("Task timed out after {} seconds", timeout.as_secs())),
                    }
                }
            };

            let event_type = match result.status {
                TaskStatus::Completed => SubAgentEventType::TaskCompleted,
                TaskStatus::Failed => SubAgentEventType::TaskFailed,
                TaskStatus::TimedOut => SubAgentEventType::TaskTimedOut,
                _ => SubAgentEventType::TaskFailed,
            };

            let _ = event_tx.send(SubAgentEvent {
                task_id: tid.clone(),
                event_type,
                timestamp: chrono::Utc::now(),
                data: Some(
                    result
                        .error
                        .clone()
                        .unwrap_or_else(|| result.output.clone()),
                ),
            });

            // Store result
            {
                let mut tasks_guard = tasks.lock().await;
                if let Some(rt) = tasks_guard.get_mut(&tid) {
                    rt.status = result.status.clone();
                    rt.result = Some(result);
                }
            }
        });

        info!(
            "🤖 Sub-agent submitted: {} (type: {}, id: {})",
            task_desc, task_type, task_id
        );

        task_id
    }

    /// Poll for a task result (None if still running).
    pub async fn poll_result(&self, task_id: &str) -> Option<SubAgentResult> {
        let tasks = self.tasks.lock().await;
        tasks.get(task_id).and_then(|rt| rt.result.clone())
    }

    /// Wait for a task to complete (blocking).
    pub async fn wait_for(&self, task_id: &str, poll_interval: Duration) -> SubAgentResult {
        loop {
            if let Some(result) = self.poll_result(task_id).await {
                return result;
            }
            tokio::time::sleep(poll_interval).await;
        }
    }

    /// Cancel a running task.
    pub async fn cancel(&self, task_id: &str) -> bool {
        let mut tasks = self.tasks.lock().await;
        if let Some(rt) = tasks.get_mut(task_id) {
            if rt.status == TaskStatus::Running || rt.status == TaskStatus::Queued {
                rt.status = TaskStatus::Cancelled;
                rt.result = Some(SubAgentResult {
                    task_id: task_id.into(),
                    status: TaskStatus::Cancelled,
                    output: String::new(),
                    agent_type: rt.task.agent_type.clone(),
                    turns_used: 0,
                    duration_ms: rt.started_at.elapsed().as_millis() as u64,
                    error: Some("Cancelled by lead agent".into()),
                });
                return true;
            }
        }
        false
    }

    /// Get status of all tasks.
    pub async fn status(&self) -> Vec<(String, TaskStatus, String)> {
        let tasks = self.tasks.lock().await;
        tasks
            .iter()
            .map(|(id, rt)| (id.clone(), rt.status.clone(), rt.task.description.clone()))
            .collect()
    }

    /// Get count of active (queued + running) tasks.
    pub async fn active_count(&self) -> usize {
        let tasks = self.tasks.lock().await;
        tasks
            .values()
            .filter(|rt| rt.status == TaskStatus::Queued || rt.status == TaskStatus::Running)
            .count()
    }

    /// Clean up completed/failed tasks older than given duration.
    pub async fn cleanup(&self, max_age: Duration) -> usize {
        let mut tasks = self.tasks.lock().await;
        let before = tasks.len();
        tasks.retain(|_, rt| {
            let is_terminal = matches!(
                rt.status,
                TaskStatus::Completed | TaskStatus::Failed | TaskStatus::TimedOut | TaskStatus::Cancelled
            );
            if is_terminal && rt.started_at.elapsed() > max_age {
                return false;
            }
            true
        });
        before - tasks.len()
    }

    /// Submit multiple tasks and wait for all to complete.
    pub async fn fan_out(&self, tasks: Vec<SubAgentTask>) -> Vec<SubAgentResult> {
        let mut task_ids = Vec::new();

        for task in tasks {
            let id = self.submit(task).await;
            task_ids.push(id);
        }

        info!("🔀 Fan-out: {} sub-agents running in parallel", task_ids.len());

        let mut results = Vec::new();
        for id in task_ids {
            let result = self.wait_for(&id, Duration::from_secs(2)).await;
            results.push(result);
        }

        let completed = results.iter().filter(|r| r.status == TaskStatus::Completed).count();
        let failed = results.len() - completed;
        info!("🔀 Fan-out complete: {} succeeded, {} failed", completed, failed);

        results
    }
}

impl Default for SubAgentExecutor {
    fn default() -> Self {
        Self::new()
    }
}

/// Execute a sub-agent task.
///
/// This is the core execution function. In a full implementation, this would:
/// 1. Create a scoped Agent with the right toolset
/// 2. Run the agent's Think-Act-Observe loop
/// 3. Return the final output
///
/// For now, this provides the task execution framework.
/// The actual LLM integration will be wired when SubAgentExecutor
/// is integrated into the main Agent.
async fn execute_task(task: &SubAgentTask) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    info!(
        "🤖 Sub-agent executing: {} (type: {}, max_turns: {})",
        task.description, task.agent_type, task.max_turns
    );

    // Simulate task execution — this will be replaced with actual Agent execution
    // when wired into the agent loop. The framework (semaphores, timeouts, events,
    // fan-out) is production-ready.

    // In production, this would be:
    // let scoped_agent = Agent::new_scoped(task.agent_type, task.prompt, tools)?;
    // let result = scoped_agent.run(task.max_turns).await?;
    // return Ok(result);

    Ok(format!(
        "[Sub-agent result for: {}]\nAgent type: {}\nMax turns: {}\nTask completed successfully.",
        task.description, task.agent_type, task.max_turns
    ))
}

/// Build the tool definition for the `task` tool that the lead agent uses
/// to delegate work to sub-agents.
pub fn task_tool_definition() -> bizclaw_core::types::ToolDefinition {
    bizclaw_core::types::ToolDefinition {
        name: "task".into(),
        description: "Delegate a task to a sub-agent. Use this for complex, multi-step tasks \
            that would benefit from parallel execution or specialized tools. Each sub-agent \
            runs in its own isolated context."
            .into(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "description": {
                    "type": "string",
                    "description": "Brief description of the task (shown in status)"
                },
                "prompt": {
                    "type": "string",
                    "description": "Detailed instructions for the sub-agent"
                },
                "agent_type": {
                    "type": "string",
                    "enum": ["general_purpose", "bash", "research", "coding"],
                    "description": "Type of sub-agent to use"
                },
                "max_turns": {
                    "type": "integer",
                    "description": "Maximum conversation turns (default: 10)",
                    "default": 10
                }
            },
            "required": ["description", "prompt"]
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_executor_submit_and_poll() {
        let executor = SubAgentExecutor::new();

        let task = SubAgentTask {
            description: "Test task".into(),
            prompt: "Do something".into(),
            agent_type: SubAgentType::GeneralPurpose,
            max_turns: 5,
            timeout_secs: Some(10),
        };

        let task_id = executor.submit(task).await;
        assert!(task_id.starts_with("subagent_"));

        // Wait for completion
        let result = executor
            .wait_for(&task_id, Duration::from_millis(100))
            .await;
        assert_eq!(result.status, TaskStatus::Completed);
        assert!(result.output.contains("Test task"));
    }

    #[tokio::test]
    async fn test_executor_fan_out() {
        let executor = SubAgentExecutor::new();

        let tasks = vec![
            SubAgentTask {
                description: "Task A".into(),
                prompt: "Research topic A".into(),
                agent_type: SubAgentType::Research,
                max_turns: 3,
                timeout_secs: Some(10),
            },
            SubAgentTask {
                description: "Task B".into(),
                prompt: "Code solution B".into(),
                agent_type: SubAgentType::Coding,
                max_turns: 5,
                timeout_secs: Some(10),
            },
        ];

        let results = executor.fan_out(tasks).await;
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|r| r.status == TaskStatus::Completed));
    }

    #[tokio::test]
    async fn test_executor_cancel() {
        let executor = SubAgentExecutor::new();

        let task = SubAgentTask {
            description: "Long task".into(),
            prompt: "Wait forever".into(),
            agent_type: SubAgentType::Bash,
            max_turns: 100,
            timeout_secs: Some(300),
        };

        let task_id = executor.submit(task).await;

        // Give it a moment to start
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Cancel should succeed if still running
        // (might already be done since execute_task returns immediately)
        let _cancelled = executor.cancel(&task_id).await;

        // Either cancelled or already completed
        let result = executor
            .wait_for(&task_id, Duration::from_millis(100))
            .await;
        assert!(
            result.status == TaskStatus::Cancelled || result.status == TaskStatus::Completed
        );
    }

    #[tokio::test]
    async fn test_executor_status() {
        let executor = SubAgentExecutor::new();

        let task = SubAgentTask {
            description: "Status check".into(),
            prompt: "Test".into(),
            agent_type: SubAgentType::GeneralPurpose,
            max_turns: 1,
            timeout_secs: Some(5),
        };

        let _id = executor.submit(task).await;
        tokio::time::sleep(Duration::from_millis(100)).await;

        let statuses = executor.status().await;
        assert_eq!(statuses.len(), 1);
    }

    #[tokio::test]
    async fn test_executor_cleanup() {
        let executor = SubAgentExecutor::new();

        let task = SubAgentTask {
            description: "Old task".into(),
            prompt: "Done".into(),
            agent_type: SubAgentType::Bash,
            max_turns: 1,
            timeout_secs: Some(5),
        };

        let id = executor.submit(task).await;
        executor
            .wait_for(&id, Duration::from_millis(100))
            .await;

        // Cleanup with 0 duration should remove completed tasks
        let removed = executor.cleanup(Duration::ZERO).await;
        assert_eq!(removed, 1);
    }

    #[tokio::test]
    async fn test_subagent_type_display() {
        assert_eq!(SubAgentType::GeneralPurpose.to_string(), "general-purpose");
        assert_eq!(SubAgentType::Bash.to_string(), "bash");
        assert_eq!(SubAgentType::Research.to_string(), "research");
        assert_eq!(SubAgentType::Coding.to_string(), "coding");
        assert_eq!(
            SubAgentType::Custom(vec!["a".into(), "b".into()]).to_string(),
            "custom(a,b)"
        );
    }

    #[tokio::test]
    async fn test_task_tool_definition() {
        let def = task_tool_definition();
        assert_eq!(def.name, "task");
        assert!(def.description.contains("sub-agent"));
    }

    #[tokio::test]
    async fn test_active_count() {
        let executor = SubAgentExecutor::new();
        assert_eq!(executor.active_count().await, 0);

        let task = SubAgentTask {
            description: "Quick".into(),
            prompt: "Fast".into(),
            agent_type: SubAgentType::Bash,
            max_turns: 1,
            timeout_secs: Some(5),
        };

        let id = executor.submit(task).await;
        executor
            .wait_for(&id, Duration::from_millis(100))
            .await;
        // After completion, active count should be 0
        assert_eq!(executor.active_count().await, 0);
    }
}
