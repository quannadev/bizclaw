//! Scheduler and Workflow validation APIs.
use crate::server::AppState;
use axum::{Json, extract::State};
use std::sync::Arc;

// ---- Scheduler API ----

/// List all scheduled tasks.
pub async fn scheduler_list_tasks(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let engine = state.scheduler.lock().await;
    let tasks: Vec<_> = engine
        .list_tasks()
        .iter()
        .map(|t| {
            let action_type = match &t.action {
                bizclaw_scheduler::tasks::TaskAction::AgentPrompt(_) => "agent_prompt",
                bizclaw_scheduler::tasks::TaskAction::Notify(_) => "notify",
                bizclaw_scheduler::tasks::TaskAction::Webhook { .. } => "webhook",
            };
            serde_json::json!({
                "id": t.id,
                "name": t.name,
                "action": t.action,
                "task_type": t.task_type,
                "status": t.status,
                "enabled": t.enabled,
                "run_count": t.run_count,
                "next_run": t.next_run.map(|d| d.to_rfc3339()),
                "last_run": t.last_run.map(|d| d.to_rfc3339()),
                "action_type": action_type,
                "agent_name": t.agent_name,
                "deliver_to": t.deliver_to,
                // Retry fields
                "fail_count": t.fail_count,
                "last_error": t.last_error,
                "retry": {
                    "max_retries": t.retry.max_retries,
                    "base_delay_secs": t.retry.base_delay_secs,
                    "backoff_multiplier": t.retry.backoff_multiplier,
                    "max_delay_secs": t.retry.max_delay_secs,
                },
                "retry_status": t.retry_status(),
            })
        })
        .collect();
    let stats = engine.retry_stats();
    Json(serde_json::json!({
        "ok": true,
        "tasks": tasks,
        "count": tasks.len(),
        "stats": {
            "retrying": stats.retrying,
            "permanently_failed": stats.permanently_failed,
            "total_retries": stats.total_retries,
        }
    }))
}

/// Add a new scheduled task.
pub async fn scheduler_add_task(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let name = body["name"].as_str().unwrap_or("unnamed");
    let prompt = body["prompt"].as_str().unwrap_or("");
    let action_str = body["action"].as_str().unwrap_or("");
    let agent_name = body["agent_name"]
        .as_str()
        .filter(|s| !s.is_empty())
        .map(String::from);
    let deliver_to = body["deliver_to"]
        .as_str()
        .filter(|s| !s.is_empty())
        .map(String::from);

    // If prompt is provided, use AgentPrompt; otherwise Notify
    let action = if !prompt.is_empty() {
        bizclaw_scheduler::tasks::TaskAction::AgentPrompt(prompt.to_string())
    } else if !action_str.is_empty() {
        bizclaw_scheduler::tasks::TaskAction::Notify(action_str.to_string())
    } else {
        return Json(
            serde_json::json!({"ok": false, "error": "Either 'prompt' or 'action' is required"}),
        );
    };

    let task_type = body["task_type"]
        .as_str()
        .or_else(|| body["type"].as_str())
        .unwrap_or("cron");

    let mut task = match task_type {
        "cron" => {
            let expr = body["cron"]
                .as_str()
                .or_else(|| body["expression"].as_str())
                .unwrap_or("0 * * * *");
            bizclaw_scheduler::Task::cron(name, expr, action)
        }
        "once" => {
            let at = chrono::Utc::now()
                + chrono::Duration::seconds(body["delay_secs"].as_i64().unwrap_or(60));
            bizclaw_scheduler::Task::once(name, at, action)
        }
        _ => {
            let secs = body["interval_secs"].as_u64().unwrap_or(300);
            bizclaw_scheduler::Task::interval(name, secs, action)
        }
    };

    // Set optional fields
    task.agent_name = agent_name;
    task.deliver_to = deliver_to.clone();
    task.notify_via = deliver_to;

    let id = task.id.clone();
    state.scheduler.lock().await.add_task(task);
    Json(serde_json::json!({"ok": true, "id": id}))
}

/// Remove a scheduled task.
pub async fn scheduler_remove_task(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    let removed = state.scheduler.lock().await.remove_task(&id);
    Json(serde_json::json!({"ok": removed}))
}

/// Toggle a scheduled task (enable/disable).
pub async fn scheduler_toggle_task(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let enabled = body["enabled"].as_bool().unwrap_or(true);
    state.scheduler.lock().await.set_enabled(&id, enabled);
    Json(serde_json::json!({"ok": true, "enabled": enabled}))
}

/// Get notification history.
pub async fn scheduler_notifications(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let engine = state.scheduler.lock().await;
    let history: Vec<_> = engine
        .router
        .history()
        .iter()
        .map(|n| {
            serde_json::json!({
                "title": n.title,
                "body": n.body,
                "source": n.source,
                "priority": format!("{:?}", n.priority),
                "timestamp": n.timestamp.to_rfc3339(),
            })
        })
        .collect();
    Json(serde_json::json!({"ok": true, "notifications": history}))
}

// ---- Workflow Rules API ----

/// List all workflow rules.
pub async fn workflow_rules_list(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let sched_dir = state
        .config_path
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .join("scheduler");
    let db = match bizclaw_scheduler::SchedulerDb::open(&sched_dir.join("scheduler.db")) {
        Ok(db) => db,
        Err(e) => return Json(serde_json::json!({"ok": false, "error": e})),
    };
    let rules: Vec<_> = db
        .load_workflow_rules()
        .iter()
        .map(|r| {
            serde_json::json!({
                "id": r.id,
                "name": r.name,
                "trigger_type": r.trigger_type,
                "trigger_config": r.trigger_config,
                "action_type": r.action_type,
                "action_config": r.action_config,
                "enabled": r.enabled,
                "priority": r.priority,
                "cooldown_secs": r.cooldown_secs,
                "run_count": r.run_count,
                "last_triggered": r.last_triggered.map(|t| t.to_rfc3339()),
            })
        })
        .collect();
    Json(serde_json::json!({"ok": true, "rules": rules}))
}

/// Add a workflow rule.
pub async fn workflow_rules_add(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let name = body["name"].as_str().unwrap_or("untitled");
    let trigger_type = body["trigger_type"].as_str().unwrap_or("message_keyword");
    let trigger_config = body
        .get("trigger_config")
        .cloned()
        .unwrap_or(serde_json::json!({}));
    let action_type = body["action_type"].as_str().unwrap_or("notify");
    let action_config = body
        .get("action_config")
        .cloned()
        .unwrap_or(serde_json::json!({}));
    let priority = body["priority"].as_i64().unwrap_or(10) as i32;
    let cooldown = body["cooldown_secs"].as_u64().unwrap_or(60);

    let mut rule = bizclaw_scheduler::persistence::WorkflowRule::new(
        name,
        trigger_type,
        trigger_config,
        action_type,
        action_config,
    );
    rule.priority = priority;
    rule.cooldown_secs = cooldown;

    let sched_dir = state
        .config_path
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .join("scheduler");
    let db = match bizclaw_scheduler::SchedulerDb::open(&sched_dir.join("scheduler.db")) {
        Ok(db) => db,
        Err(e) => return Json(serde_json::json!({"ok": false, "error": e})),
    };
    let _ = db.save_workflow_rule(&rule);

    Json(serde_json::json!({"ok": true, "id": rule.id}))
}

/// Delete a workflow rule.
pub async fn workflow_rules_delete(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    let sched_dir = state
        .config_path
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .join("scheduler");
    let db = match bizclaw_scheduler::SchedulerDb::open(&sched_dir.join("scheduler.db")) {
        Ok(db) => db,
        Err(e) => return Json(serde_json::json!({"ok": false, "error": e})),
    };
    let _ = db.delete_workflow_rule(&id);
    Json(serde_json::json!({"ok": true}))
}
