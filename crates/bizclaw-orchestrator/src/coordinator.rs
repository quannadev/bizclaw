//! Coordinator Agent logic (The "Mama Tổng Quản").
//!
//! Exposes `Coordinator` which receives a goal, generates a Task DAG, 
//! and automatically dispatches workloads in parallel across Tokio tasks.

use crate::team::AgentTeam;
use crate::task_dag::{TaskDag, Task, TaskStatus};
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct Coordinator {
    /// The team of agents available to dispatch tasks
    team: Arc<RwLock<AgentTeam>>,
}

impl Coordinator {
    pub fn new(team: AgentTeam) -> Self {
        Self {
            team: Arc::new(RwLock::new(team)),
        }
    }

    /// Run the team to satisfy the given Goal using true multithreading!
    pub async fn run_team(&self, goal: &str) -> Result<String, String> {
        tracing::info!("Coordinator started for goal: {}", goal);

        // In a real implementation, we would call an LLM (Gemma4/Claude) here
        // passing `goal` and `available_agents` tools to generate the JSON.
        // For the sake of the engine, we mock the decomposition phase.

        let mut dag = self.decompose_goal_into_tasks(goal).await?;

        // Shared graph for asynchronous execution
        let shared_dag = Arc::new(RwLock::new(dag));

        loop {
             // Find unblocked tasks in Pending state
             let ready_task_ids = {
                 let mut graph = shared_dag.write().await;
                 let ready = graph.get_ready_tasks();
                 // Mark them as running immediately to avoid double-processing
                 for task_id in &ready {
                     graph.mark_running(task_id);
                 }
                 ready
             };

             if ready_task_ids.is_empty() {
                 let is_done = shared_dag.read().await.is_all_completed();
                 if is_done {
                     break; // Goal completed successfully
                 }
                 // If no tasks are ready but not all completed, we must wait for
                 // running tasks to finish. In a real reactor design, we would await a channel signal here instead of sleeping.
                 tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
                 continue;
             }

             // SPawn parallel executions -- THIS IS WHERE RUST SHINES vs NODE.JS
             // Each task executes on a truly independent tokio green-thread context.
             for task_id in ready_task_ids {
                 let dag_clone = Arc::clone(&shared_dag);
                 let team_clone = Arc::clone(&self.team);
                 let task_id_clone = task_id.clone();

                 tokio::spawn(async move {
                      // Lookup task
                      let task_info = {
                          let graph = dag_clone.read().await;
                          graph.get_task(&task_id_clone).cloned().unwrap()
                      };

                      tracing::info!("Starting Task: {} assigned to Agent: {}", task_info.name, task_info.assigned_agent);

                      // Dispatch to LLM provider (MOCKED)
                      // In real life, `team.agent_for_channel(...)` or similar is used here to get instructions
                      tokio::time::sleep(tokio::time::Duration::from_millis(500)).await; // simulate LLM thinking

                      let resultStr = format!("Result for {}", task_info.name);

                      tracing::info!("Finished Task: {}", task_info.name);

                      // Update Graph
                      let mut graph = dag_clone.write().await;
                      graph.mark_completed(&task_id_clone, resultStr);

                      // A channel could also be used here to signal the loop to wake up immediately.
                 });
             }
        }

        tracing::info!("Coordinator successfully orchestrated team to completion!");

        Ok("All subtasks completed successfully. Result aggregated.".to_string())
    }

    /// LLM powered Auto Task Decomposition (mocked for demo structure)
    async fn decompose_goal_into_tasks(&self, _goal: &str) -> Result<TaskDag, String> {
        let mut dag = TaskDag::new();

        // Task 1: Research (Architect)
        dag.add_task(Task {
            id: "task_1".to_string(),
            name: "Research & Design API".to_string(),
            description: "Analyze requirement".to_string(),
            assigned_agent: "architect".to_string(),
            status: TaskStatus::Pending,
            result: None,
        });

        // Task 2: Implement (Developer)
        dag.add_task(Task {
            id: "task_2".to_string(),
            name: "Write Code".to_string(),
            description: "Implement API based on design".to_string(),
            assigned_agent: "developer".to_string(),
            status: TaskStatus::Pending,
            result: None,
        });

        // Task 3: Review (Reviewer)
        dag.add_task(Task {
            id: "task_3".to_string(),
            name: "Review Code".to_string(),
            description: "Check for security bugs".to_string(),
            assigned_agent: "reviewer".to_string(),
            status: TaskStatus::Pending,
            result: None,
        });

        // Enforce Workflow: Developer CANNOT start until Architect finishes
        dag.add_dependency("task_2", "task_1")?; 
        
        // Enforce Workflow: Reviewer CANNOT start until Developer finishes
        dag.add_dependency("task_3", "task_2")?;

        Ok(dag)
    }
}
