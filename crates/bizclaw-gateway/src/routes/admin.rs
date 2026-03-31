//! Admin, PaaS, and System Management API route handlers.
//!
//! Includes: API key management, usage/quotas, system metrics,
//! Prometheus endpoints, audit log, backup/restore.
//! Extracted from routes/mod.rs.

use axum::{Json, extract::State};
use std::sync::Arc;

use crate::server::AppState;

/// Clear all traces
pub async fn clear_traces(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let mut traces = state.traces.lock().unwrap_or_else(|e| e.into_inner());
    let count = traces.len();
    traces.clear();
    tracing::info!("🗑️ Cleared {} LLM traces", count);
    Json(serde_json::json!({"ok": true, "cleared": count}))
}

/// Clear activity
pub async fn clear_activity(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let mut events = state.activity_log.lock().unwrap_or_else(|e| e.into_inner());
    let count = events.len();
    events.clear();
    tracing::info!("🗑️ Cleared {} activity events", count);
    Json(serde_json::json!({"ok": true, "cleared": count}))
}

// ═══ PaaS: API Key Management ═══

/// POST /api/v1/api-keys — Create a new API key
pub async fn create_api_key(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let name = body["name"].as_str().unwrap_or("Unnamed Key");
    let scopes = body["scopes"].as_str().unwrap_or("read,write");
    let expires_days = body["expires_days"].as_i64();

    match state.db.create_api_key(name, scopes, expires_days) {
        Ok((id, raw_key)) => {
            tracing::info!(
                "🔑 API key created: {} ({})",
                name,
                raw_key.chars().take(10).collect::<String>()
            );
            // Track usage
            let _ = state.db.track_usage("api_keys_created", 1.0);
            Json(serde_json::json!({
                "ok": true,
                "id": id,
                "key": raw_key,
                "message": "API key created. Save this key — it won't be shown again!"
            }))
        }
        Err(e) => Json(serde_json::json!({"ok": false, "error": e})),
    }
}

/// GET /api/v1/api-keys — List all API keys
pub async fn list_api_keys(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    match state.db.list_api_keys() {
        Ok(keys) => Json(serde_json::json!({"ok": true, "keys": keys, "count": keys.len()})),
        Err(e) => Json(serde_json::json!({"ok": false, "error": e})),
    }
}

/// DELETE /api/v1/api-keys/:id — Revoke an API key
pub async fn revoke_api_key(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    match state.db.revoke_api_key(&id) {
        Ok(true) => {
            tracing::info!("🗑️ API key revoked: {}", id);
            Json(serde_json::json!({"ok": true, "message": "Key revoked"}))
        }
        Ok(false) => Json(serde_json::json!({"ok": false, "error": "Key not found"})),
        Err(e) => Json(serde_json::json!({"ok": false, "error": e})),
    }
}

// ═══ PaaS: Usage & Quotas ═══

/// GET /api/v1/usage — Current month usage summary
pub async fn get_usage(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let usage = state.db.get_monthly_usage().unwrap_or_default();
    let limits = state.db.get_plan_limits().unwrap_or_default();
    // Also include real-time stats
    let traces_count = state.traces.lock().map(|t| t.len()).unwrap_or(0);
    let agents_count = {
        let orch = state.orchestrator.lock().await;
        orch.list_agents().len()
    };
    Json(serde_json::json!({
        "ok": true,
        "usage": usage,
        "limits": limits,
        "realtime": {
            "active_agents": agents_count,
            "traces_in_memory": traces_count,
        }
    }))
}

/// GET /api/v1/usage/daily?days=30 — Daily usage breakdown
pub async fn get_usage_daily(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<serde_json::Value> {
    let days = params
        .get("days")
        .and_then(|d| d.parse::<i64>().ok())
        .unwrap_or(30);
    match state.db.get_daily_usage(days) {
        Ok(data) => Json(serde_json::json!({"ok": true, "data": data, "days": days})),
        Err(e) => Json(serde_json::json!({"ok": false, "error": e})),
    }
}

/// GET /api/v1/usage/limits — Current plan limits
pub async fn get_plan_limits(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    match state.db.get_plan_limits() {
        Ok(limits) => Json(serde_json::json!({"ok": true, "limits": limits})),
        Err(e) => Json(serde_json::json!({"ok": false, "error": e})),
    }
}

/// PUT /api/v1/usage/limits — Update plan limits
pub async fn update_plan_limits(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    if let Some(obj) = body.as_object() {
        for (key, val) in obj {
            if let Some(v) = val.as_i64() {
                let _ = state.db.set_plan_limit(key, v);
            }
        }
        tracing::info!("📊 Plan limits updated");
        Json(serde_json::json!({"ok": true, "message": "Limits updated"}))
    } else {
        Json(serde_json::json!({"ok": false, "error": "Expected JSON object"}))
    }
}

// ═══ PaaS: System Metrics ═══

/// GET /api/v1/metrics — System metrics for dashboard
pub async fn get_system_metrics(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    // Agent count
    let agents_count = {
        let orch = state.orchestrator.lock().await;
        orch.list_agents().len()
    };
    // Provider count
    let providers = state.db.list_providers("").map(|p| p.len()).unwrap_or(0);
    let active_providers = state
        .db
        .list_providers("")
        .map(|p| p.iter().filter(|x| x.is_active).count())
        .unwrap_or(0);
    // API keys
    let api_keys = state.db.list_api_keys().map(|k| k.len()).unwrap_or(0);
    // Traces stats
    let (traces_count, total_tokens, total_cost) = {
        let traces = state.traces.lock().unwrap_or_else(|e| e.into_inner());
        let count = traces.len();
        let tokens: i64 = traces.iter().map(|t| t.total_tokens as i64).sum();
        let cost: f64 = traces.iter().map(|t| t.cost_usd).sum();
        (count, tokens, cost)
    };
    // Usage this month
    let monthly = state.db.get_monthly_usage().unwrap_or_default();
    let limits = state.db.get_plan_limits().unwrap_or_default();
    // Uptime
    let uptime_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    Json(serde_json::json!({
        "ok": true,
        "metrics": {
            "agents": agents_count,
            "providers": providers,
            "active_providers": active_providers,
            "api_keys": api_keys,
            "traces_session": traces_count,
            "tokens_session": total_tokens,
            "cost_session": total_cost,
            "usage_month": monthly,
            "limits": limits,
            "uptime_seconds": uptime_secs,
        }
    }))
}

// ═══ Prometheus Metrics (text/plain, OpenMetrics format) ═══

/// GET /metrics — Prometheus-compatible metrics endpoint.
/// Returns text/plain in OpenMetrics exposition format for Prometheus scraping.
pub async fn prometheus_metrics(State(state): State<Arc<AppState>>) -> axum::response::Response {
    let agents_count = {
        let orch = state.orchestrator.lock().await;
        orch.list_agents().len()
    };
    let providers = state.db.list_providers("").map(|p| p.len()).unwrap_or(0);
    let active_providers = state
        .db
        .list_providers("")
        .map(|p| p.iter().filter(|x| x.is_active).count())
        .unwrap_or(0);
    let api_keys = state.db.list_api_keys().map(|k| k.len()).unwrap_or(0);
    let (traces_count, total_tokens, total_cost) = {
        let traces = state.traces.lock().unwrap_or_else(|e| e.into_inner());
        let count = traces.len();
        let tokens: i64 = traces.iter().map(|t| t.total_tokens as i64).sum();
        let cost: f64 = traces.iter().map(|t| t.cost_usd).sum();
        (count, tokens, cost)
    };
    let uptime_secs = state.start_time.elapsed().as_secs();

    let mut output = String::with_capacity(2048);
    output.push_str("# HELP bizclaw_agents_total Number of configured agents\n");
    output.push_str("# TYPE bizclaw_agents_total gauge\n");
    output.push_str(&format!("bizclaw_agents_total {agents_count}\n"));
    output.push_str("# HELP bizclaw_providers_total Number of configured providers\n");
    output.push_str("# TYPE bizclaw_providers_total gauge\n");
    output.push_str(&format!("bizclaw_providers_total {providers}\n"));
    output.push_str("# HELP bizclaw_providers_active Number of active providers\n");
    output.push_str("# TYPE bizclaw_providers_active gauge\n");
    output.push_str(&format!("bizclaw_providers_active {active_providers}\n"));
    output.push_str("# HELP bizclaw_api_keys_total Number of API keys\n");
    output.push_str("# TYPE bizclaw_api_keys_total gauge\n");
    output.push_str(&format!("bizclaw_api_keys_total {api_keys}\n"));
    output
        .push_str("# HELP bizclaw_traces_session_total Number of LLM traces in current session\n");
    output.push_str("# TYPE bizclaw_traces_session_total counter\n");
    output.push_str(&format!("bizclaw_traces_session_total {traces_count}\n"));
    output
        .push_str("# HELP bizclaw_tokens_session_total Total tokens consumed in current session\n");
    output.push_str("# TYPE bizclaw_tokens_session_total counter\n");
    output.push_str(&format!("bizclaw_tokens_session_total {total_tokens}\n"));
    output.push_str("# HELP bizclaw_cost_session_usd Total cost in USD for current session\n");
    output.push_str("# TYPE bizclaw_cost_session_usd counter\n");
    output.push_str(&format!("bizclaw_cost_session_usd {total_cost:.6}\n"));
    output.push_str("# HELP bizclaw_uptime_seconds Uptime in seconds\n");
    output.push_str("# TYPE bizclaw_uptime_seconds counter\n");
    output.push_str(&format!("bizclaw_uptime_seconds {uptime_secs}\n"));

    axum::response::Response::builder()
        .status(200)
        .header("Content-Type", "text/plain; version=0.0.4; charset=utf-8")
        .body(axum::body::Body::from(output))
        .unwrap()
}

/// GET /api/v1/audit — List audit log entries.
pub async fn list_audit_log(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<serde_json::Value> {
    let limit = params
        .get("limit")
        .and_then(|v| v.parse().ok())
        .unwrap_or(50i64);
    let offset = params
        .get("offset")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0i64);
    match state.db.get_audit_log(limit, offset) {
        Ok(entries) => {
            Json(serde_json::json!({ "ok": true, "entries": entries, "count": entries.len() }))
        }
        Err(e) => Json(serde_json::json!({ "ok": false, "error": e })),
    }
}

// ═══ Backup & Restore API ═══

/// GET /api/v1/backup — Export system configuration as JSON snapshot.
/// Admin-only. Excludes sensitive data (API keys are masked).
pub async fn export_backup(State(state): State<Arc<AppState>>) -> axum::response::Response {
    let mut backup = serde_json::json!({
        "version": "1.0",
        "exported_at": chrono::Utc::now().to_rfc3339(),
        "type": "bizclaw-backup"
    });

    // Providers (mask API keys)
    if let Ok(providers) = state.db.list_providers("") {
        let masked: Vec<_> = providers
            .iter()
            .map(|p| {
                let mut v = serde_json::to_value(p).unwrap_or_default();
                if let Some(obj) = v.as_object_mut() {
                    obj.insert("api_key".into(), serde_json::json!("***MASKED***"));
                }
                v
            })
            .collect();
        backup["providers"] = serde_json::json!(masked);
    }

    // Agents
    if let Ok(agents) = state.db.list_agents() {
        backup["agents"] = serde_json::to_value(&agents).unwrap_or_default();
    }

    // Agent-Channel bindings
    if let Ok(bindings) = state.db.all_agent_channels() {
        backup["agent_channels"] = serde_json::to_value(&bindings).unwrap_or_default();
    }

    // Plan limits
    if let Ok(limits) = state.db.get_plan_limits() {
        backup["plan_limits"] = limits;
    }

    // Config file
    if let Ok(content) = std::fs::read_to_string(&state.config_path) {
        backup["config_file"] = serde_json::json!(content);
    }

    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
    let filename = format!("bizclaw_backup_{timestamp}.json");
    let body = serde_json::to_string_pretty(&backup).unwrap_or_default();

    axum::response::Response::builder()
        .status(200)
        .header("Content-Type", "application/json")
        .header(
            "Content-Disposition",
            format!("attachment; filename=\"{filename}\""),
        )
        .body(axum::body::Body::from(body))
        .unwrap()
}

/// POST /api/v1/restore — Import system configuration from JSON backup.
/// Admin-only. Restores agents and agent-channel bindings. Does NOT restore API keys.
pub async fn import_restore(
    State(state): State<Arc<AppState>>,
    Json(backup): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    // Validate backup format
    if backup.get("type").and_then(|v| v.as_str()) != Some("bizclaw-backup") {
        return Json(
            serde_json::json!({ "ok": false, "error": "Invalid backup format — missing type: bizclaw-backup" }),
        );
    }

    let mut restored = serde_json::Map::new();

    // Restore agents
    if let Some(agents) = backup.get("agents").and_then(|v| v.as_array()) {
        let mut count = 0;
        for agent in agents {
            let name = agent.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let role = agent
                .get("role")
                .and_then(|v| v.as_str())
                .unwrap_or("assistant");
            let description = agent
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let provider = agent
                .get("provider")
                .and_then(|v| v.as_str())
                .unwrap_or("openai");
            let model = agent
                .get("model")
                .and_then(|v| v.as_str())
                .unwrap_or("gpt-4o-mini");
            let system_prompt = agent
                .get("system_prompt")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if !name.is_empty()
                && state
                    .db
                    .upsert_agent(name, role, description, provider, model, system_prompt)
                    .is_ok()
            {
                count += 1;
            }
        }
        restored.insert("agents_restored".into(), serde_json::json!(count));
    }

    // Restore agent-channel bindings
    if let Some(bindings) = backup.get("agent_channels").and_then(|v| v.as_object()) {
        let mut count = 0;
        for (agent_name, channels) in bindings {
            if let Some(ch_arr) = channels.as_array() {
                let channels: Vec<String> = ch_arr
                    .iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect();
                if state.db.set_agent_channels(agent_name, &channels).is_ok() {
                    count += 1;
                }
            }
        }
        restored.insert("bindings_restored".into(), serde_json::json!(count));
    }

    Json(serde_json::json!({
        "ok": true,
        "restored": restored,
        "note": "API keys are not restored for security — please reconfigure them manually."
    }))
}
