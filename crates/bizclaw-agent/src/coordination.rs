//! Multi-Agent Coordination
//!
//! Inspired by OpenHarness subagent patterns:
//! - Team coordination
//! - Background task lifecycle
//! - Task delegation

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Agent team configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Team {
    pub id: String,
    pub name: String,
    pub agents: Vec<TeamAgent>,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamAgent {
    pub id: String,
    pub name: String,
    pub role: AgentRole,
    pub status: AgentStatus,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum AgentStatus {
    Idle,
    Working,
    Waiting,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum AgentRole {
    Coordinator,
    Coder,
    Researcher,
    Reviewer,
    Executor,
}

impl Team {
    pub fn new(id: &str, name: str) -> Self {
        Self {
            id: id.to_string(),
            name,
            agents: Vec::new(),
            created_at: chrono::Utc::now().timestamp(),
        }
    }

    pub fn add_agent(mut self, agent: TeamAgent) -> Self {
        self.agents.push(agent);
        self
    }
}

pub struct TeamRegistry {
    teams: Arc<RwLock<HashMap<String, Team>>,
    agents: Arc<RwLock<HashMap<String, AgentStatus>>,
}

impl TeamRegistry {
    pub fn new() -> Self {
        Self {
            teams: Arc::new(RwLock::new(HashMap::new()),
            agents: Arc::new(RwLock::new(HashMap::new()),
        }
    }

    pub async fn create_team(&self, team: Team) {
        let mut teams = self.teams.write().await;
        teams.insert(team.id.clone(), team.clone());
        for agent in &team.agents {
            let mut agents = self.agents.write().await;
            agents.insert(agent.id.clone(), AgentStatus::Idle);
        }
        info!("Team {} created with {} agents", team.id, team.agents.len());
    }

    pub async fn get_team(&self, team_id: &str) -> Option<Team> {
        let teams = self.teams.read().await;
        teams.get(team_id).cloned()
    }

    pub async fn update_agent_status(&self, agent_id: &str, status: AgentStatus) {
        let mut agents = self.agents.write().await;
        if let Some(current) = agents.get_mut(agent_id) {
            *current = status;
        }
    }

    pub async fn list_agents(&self) -> Vec<(String, AgentStatus)> {
        let agents = self.agents.read().await;
        agents.iter().map(|(k, v)| (k.clone(), *v)).collect()
    }
}

/// Task delegation with priority queue
pub struct TaskDelegator {
    tasks: Arc<RwLock<Vec<Task>>,
}

#[derive(Debug, Clone)]
pub struct Task {
    pub id: String,
    pub team_id: String,
    pub assigned_to: Option<String>,
    pub status: TaskStatus,
    pub priority: Priority,
    pub created_at: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum Priority {
    Low,
    Normal,
    High,
    Critical,
}

#[derive(Debug, Clone, Copy)]
pub enum TaskStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Cancelled,
}

impl TaskDelegator {
    pub fn new() -> Self {
        Self {
            tasks: Arc::new(RwLock::new(Vec::new()),
        }
    }

    pub async fn delegate(&self, task: Task) -> String {
        let task_id = task.id.clone();
        let mut tasks = self.tasks.write().await;
        tasks.push(task);
        tasks.sort_by(|a, b| b.priority.cmp(&a.priority));
        debug!("Task {} delegated", task_id);
        task_id
    }

    pub async fn claim_task(&self, agent_id: &str) -> Option<Task> {
        let mut tasks = self.tasks.write().await;
        if let Some(pos) = tasks.iter().position(|t| t.assigned_to.is_none()) {
            let task = &mut tasks[pos];
            task.assigned_to = Some(agent_id.to_string());
            return tasks.get(pos).cloned();
        }
        None
    }

    pub async fn complete_task(&self, task_id: &str, success: bool) {
        let mut tasks = self.tasks.write().await;
        if let Some(task) = tasks.iter_mut().find(|t| t.id == task_id) {
            task.status = if success { TaskStatus::Completed } else { TaskStatus::Failed };
        }
    }
}
