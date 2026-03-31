//! Orchestration API route handlers.
//!
//! Handles advanced multi-agent orchestration like delegation,
//! handoffs, evaluate loops, and permission links.
//! Extracted from routes/mod.rs.

use axum::{Json, extract::State};
use std::sync::Arc;

use crate::server::AppState;
use super::helpers::internal_error;
use super::safe_truncate;

/// Delegate a task from one agent to another.
/// POST /api/v1/orchestration/delegate
/// Body: {"from_agent": "a", "to_agent": "b", "task": "...", "mode": "sync|async"}
pub async fn orch_delegate(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let from = body["from_agent"].as_str().unwrap_or("");
    let to = body["to_agent"].as_str().unwrap_or("");
    let task = body["task"].as_str().unwrap_or("");

    if from.is_empty() || to.is_empty() || task.is_empty() {
        return Json(
            serde_json::json!({"ok": false, "error": "from_agent, to_agent, and task are required"}),
        );
    }

    let mode = match body["mode"].as_str().unwrap_or("sync") {
        "async" => bizclaw_core::types::DelegationMode::Async,
        _ => bizclaw_core::types::DelegationMode::Sync,
    };

    let mut orch = state.orchestrator.lock().await;
    match orch.delegate_with_mode(from, to, task, mode).await {
        Ok(response) => Json(serde_json::json!({
            "ok": true,
            "from": from,
            "to": to,
            "response": safe_truncate(&response, 5000),
        })),
        Err(e) => {
            tracing::error!("[orch_delegate] {e}");
            internal_error("delegation", e)
        }
    }
}

/// Handoff conversation from one agent to another.
/// POST /api/v1/orchestration/handoff
pub async fn orch_handoff(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let from = body["from_agent"].as_str().unwrap_or("");
    let to = body["to_agent"].as_str().unwrap_or("");
    let session = body["session_id"].as_str().unwrap_or("default");
    let reason = body["reason"].as_str();

    if from.is_empty() || to.is_empty() {
        return Json(serde_json::json!({"ok": false, "error": "from_agent and to_agent required"}));
    }

    let mut orch = state.orchestrator.lock().await;
    match orch.handoff(from, to, session, reason).await {
        Ok(()) => Json(serde_json::json!({"ok": true, "from": from, "to": to, "session": session})),
        Err(e) => {
            tracing::error!("[orch_handoff] {e}");
            internal_error("handoff", e)
        }
    }
}

/// Clear handoff for a session.
/// DELETE /api/v1/orchestration/handoff/{session_id}
pub async fn orch_clear_handoff(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(session_id): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    let orch = state.orchestrator.lock().await;
    match orch.clear_handoff(&session_id).await {
        Ok(()) => Json(serde_json::json!({"ok": true, "session": session_id})),
        Err(e) => internal_error("clear_handoff", e),
    }
}

/// Run evaluate loop between two agents.
/// POST /api/v1/orchestration/evaluate
pub async fn orch_evaluate(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let generator = body["generator"].as_str().unwrap_or("");
    let evaluator = body["evaluator"].as_str().unwrap_or("");
    let task = body["task"].as_str().unwrap_or("");
    let pass_criteria = body["pass_criteria"]
        .as_str()
        .unwrap_or("high quality output");
    let max_rounds = body["max_rounds"].as_u64().unwrap_or(3) as u32;

    if generator.is_empty() || evaluator.is_empty() || task.is_empty() {
        return Json(
            serde_json::json!({"ok": false, "error": "generator, evaluator, and task required"}),
        );
    }

    let config = bizclaw_core::types::EvaluateConfig {
        generator: generator.to_string(),
        evaluator: evaluator.to_string(),
        task: task.to_string(),
        pass_criteria: pass_criteria.to_string(),
        max_rounds,
    };

    let mut orch = state.orchestrator.lock().await;
    match orch.evaluate_loop(&config).await {
        Ok(result) => Json(serde_json::json!({
            "ok": true,
            "approved": result.approved,
            "output": result.output,
            "feedback": result.feedback,
            "rounds_used": result.rounds_used,
            "max_rounds": result.max_rounds,
        })),
        Err(e) => {
            tracing::error!("[orch_evaluate] {e}");
            internal_error("evaluate", e)
        }
    }
}

/// Create a permission link between agents.
/// POST /api/v1/orchestration/links
pub async fn orch_create_link(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let source = body["source"].as_str().unwrap_or("");
    let target = body["target"].as_str().unwrap_or("");
    let direction = match body["direction"].as_str().unwrap_or("outbound") {
        "inbound" => bizclaw_core::types::LinkDirection::Inbound,
        "bidirectional" => bizclaw_core::types::LinkDirection::Bidirectional,
        _ => bizclaw_core::types::LinkDirection::Outbound,
    };

    if source.is_empty() || target.is_empty() {
        return Json(serde_json::json!({"ok": false, "error": "source and target required"}));
    }

    let orch = state.orchestrator.lock().await;
    match orch.create_link(source, target, direction).await {
        Ok(link) => Json(serde_json::json!({"ok": true, "id": link.id})),
        Err(e) => internal_error("create_link", e),
    }
}

/// List all agent permission links.
/// GET /api/v1/orchestration/links
pub async fn orch_list_links(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let orch = state.orchestrator.lock().await;
    match orch.list_links().await {
        Ok(links) => {
            let items: Vec<serde_json::Value> = links
                .iter()
                .map(|l| {
                    serde_json::json!({
                        "id": l.id,
                        "source": l.source_agent,
                        "target": l.target_agent,
                        "direction": l.direction.to_string(),
                        "max_concurrent": l.max_concurrent,
                    })
                })
                .collect();
            Json(serde_json::json!({"ok": true, "links": items, "count": items.len()}))
        }
        Err(e) => internal_error("list_links", e),
    }
}

/// Delete a permission link.
/// DELETE /api/v1/orchestration/links/{id}
pub async fn orch_delete_link(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    let orch = state.orchestrator.lock().await;
    match orch.delete_link(&id).await {
        Ok(()) => Json(serde_json::json!({"ok": true})),
        Err(e) => internal_error("delete_link", e),
    }
}

/// List delegation history.
/// GET /api/v1/orchestration/delegations?agent=name&limit=20
pub async fn orch_list_delegations(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<serde_json::Value> {
    let agent = params.get("agent").map(|s| s.as_str()).unwrap_or("*");
    let limit = params
        .get("limit")
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(20);

    let store = &state.orch_store;
    let delegations = if agent == "*" {
        // Get all — use traces as proxy (list_delegations requires agent)
        store.list_delegations("", limit).await.unwrap_or_default()
    } else {
        store
            .list_delegations(agent, limit)
            .await
            .unwrap_or_default()
    };

    let items: Vec<serde_json::Value> = delegations
        .iter()
        .map(|d| {
            serde_json::json!({
                "id": d.id,
                "from": d.from_agent,
                "to": d.to_agent,
                "task": safe_truncate(&d.task, 200),
                "status": format!("{:?}", d.status),
                "mode": format!("{:?}", d.mode),
                "created_at": d.created_at.to_rfc3339(),
            })
        })
        .collect();

    Json(serde_json::json!({"ok": true, "delegations": items, "count": items.len()}))
}

/// List LLM traces (observability).
/// GET /api/v1/orchestration/traces?limit=50
pub async fn orch_list_traces(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<serde_json::Value> {
    let limit = params
        .get("limit")
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(50);

    let orch = state.orchestrator.lock().await;
    let traces = orch.list_traces(limit).await.unwrap_or_default();

    let items: Vec<serde_json::Value> = traces
        .iter()
        .map(|t| {
            serde_json::json!({
                "id": t.id,
                "agent": t.agent_name,
                "provider": t.provider,
                "model": t.model,
                "tokens": t.total_tokens,
                "latency_ms": t.latency_ms,
                "cache_hit": t.cache_hit,
                "status": t.status,
                "created_at": t.created_at.to_rfc3339(),
            })
        })
        .collect();

    Json(serde_json::json!({"ok": true, "traces": items, "count": items.len()}))
}
