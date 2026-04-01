//! Multi-Agent Orchestrator API route handlers.
//!
//! Agent CRUD (list, create, update, delete), agent chat,
//! and broadcast messaging.
//! Extracted from routes/mod.rs.

use axum::{Json, extract::State};
use std::sync::Arc;

use super::apply_provider_config_from_db;
use crate::server::AppState;

/// List all agents in the orchestrator.
pub async fn list_agents(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let orch = state.orchestrator.lock().await;
    let mut agents = orch.list_agents();

    // Load channel bindings and attach to each agent
    let bindings_path = state
        .config_path
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .join("agent-channels.json");
    let bindings: serde_json::Value = if bindings_path.exists() {
        std::fs::read_to_string(&bindings_path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or(serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    for agent in agents.iter_mut() {
        if let Some(name) = agent["name"].as_str() {
            let ch_data = bindings.get(name).cloned().unwrap_or(serde_json::json!([]));
            if ch_data.is_array() {
                agent
                    .as_object_mut()
                    .map(|o| o.insert("channels".into(), ch_data));
            } else if let Some(obj) = ch_data.as_object() {
                agent.as_object_mut().map(|o| {
                    o.insert(
                        "channels".into(),
                        obj.get("channels")
                            .cloned()
                            .unwrap_or(serde_json::json!([])),
                    );
                    if let Some(rag) = obj.get("rag_collection") {
                        o.insert("rag_collection".into(), rag.clone());
                    }
                    if let Some(sql) = obj.get("sql_connection") {
                        o.insert("sql_connection".into(), sql.clone());
                    }
                });
            }
        }
    }

    Json(serde_json::json!({
        "ok": true,
        "agents": agents,
        "total": orch.agent_count(),
        "default": orch.default_agent_name(),
        "recent_messages": orch.recent_messages(10),
    }))
}

/// Create a new named agent.
pub async fn create_agent(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let name = body["name"].as_str().unwrap_or("agent");
    let role = body["role"].as_str().unwrap_or("assistant");
    let description = body["description"].as_str().unwrap_or("A helpful AI agent");

    // Use current config as base, optionally override provider/model
    let mut agent_config = state
        .full_config
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .clone();
    if let Some(provider) = body["provider"].as_str()
        && !provider.is_empty()
    {
        agent_config.default_provider = provider.to_string();
        agent_config.llm.provider = provider.to_string(); // sync
    }
    if let Some(model) = body["model"].as_str()
        && !model.is_empty()
    {
        agent_config.default_model = model.to_string();
        agent_config.llm.model = model.to_string(); // sync
    }
    if let Some(persona) = body["persona"].as_str() {
        agent_config.identity.persona = persona.to_string();
    }
    if let Some(sys_prompt) = body["system_prompt"].as_str() {
        agent_config.identity.system_prompt = sys_prompt.to_string();
    }
    agent_config.identity.name = name.to_string();

    // Critical: inject per-provider API key and base_url from DB
    // This enables agents to use different providers (e.g. Ollama, DeepSeek)
    // without needing the global config to match.
    apply_provider_config_from_db(&state.db, &mut agent_config);

    // Use sync Agent::new() — MCP tools are shared at orchestrator level
    match bizclaw_agent::Agent::new(agent_config) {
        Ok(agent) => {
            let provider = agent.provider_name().to_string();
            let model = agent.model_name().to_string();
            let system_prompt = agent.system_prompt().to_string();
            let mut orch = state.orchestrator.lock().await;
            orch.add_agent(name, role, description, agent);
            // Persist to SQLite DB
            if let Err(e) =
                state
                    .db
                    .upsert_agent(name, role, description, &provider, &model, &system_prompt)
            {
                tracing::warn!("DB persist failed for agent '{}': {}", name, e);
            }
            // Also save to legacy agents.json for backward compatibility
            let agents_path = state
                .config_path
                .parent()
                .unwrap_or(std::path::Path::new("."))
                .join("agents.json");
            orch.save_agents_metadata(&agents_path);
            tracing::info!("🤖 Agent '{}' created (role={})", name, role);
            Json(serde_json::json!({
                "ok": true,
                "name": name,
                "role": role,
                "total_agents": orch.agent_count(),
            }))
        }
        Err(e) => Json(serde_json::json!({
            "ok": false,
            "error": format!("Failed to create agent: {e}"),
        })),
    }
}

/// Delete a named agent.
pub async fn delete_agent(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(name): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    let mut orch = state.orchestrator.lock().await;
    let removed = orch.remove_agent(&name);
    if removed {
        // Delete from SQLite DB
        if let Err(e) = state.db.delete_agent(&name) {
            tracing::warn!("DB delete failed for agent '{}': {}", name, e);
        }
        // Also update legacy agents.json
        let agents_path = state
            .config_path
            .parent()
            .unwrap_or(std::path::Path::new("."))
            .join("agents.json");
        orch.save_agents_metadata(&agents_path);
    }
    Json(serde_json::json!({
        "ok": removed,
        "message": if removed { format!("Agent '{}' removed", name) } else { format!("Agent '{}' not found", name) },
    }))
}

/// Update an existing agent's metadata.
pub async fn update_agent(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(name): axum::extract::Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let role = body["role"].as_str();
    let description = body["description"].as_str();
    let provider = body["provider"].as_str();
    let model = body["model"].as_str();
    let system_prompt = body["system_prompt"].as_str();

    // Phase 1: Update basic metadata + check if re-creation needed
    let mut needs_recreate = false;
    {
        let mut orch = state.orchestrator.lock().await;
        let updated = orch.update_agent(&name, role, description);
        if !updated {
            return Json(
                serde_json::json!({"ok": false, "message": format!("Agent '{}' not found", name)}),
            );
        }
        // Only re-create if provider or model ACTUALLY CHANGED (not just present)
        if let Some(agent) = orch.get_agent_mut(&name) {
            let cur_provider = agent.provider_name().to_string();
            let cur_model = agent.model_name().to_string();
            if let Some(p) = provider
                && !p.is_empty()
                && p != cur_provider
            {
                needs_recreate = true;
            }
            if let Some(m) = model
                && !m.is_empty()
                && m != cur_model
            {
                needs_recreate = true;
            }
            // Update system prompt directly on live agent (no re-creation needed)
            if !needs_recreate
                && let Some(sp) = system_prompt
                && !sp.is_empty()
                && sp != agent.system_prompt()
            {
                agent.set_system_prompt(sp);
                tracing::info!(
                    "📝 update_agent '{}' — system_prompt updated in-place",
                    name
                );
            }
        }
    } // lock released here

    // Phase 2: Re-create agent ONLY if provider/model actually changed
    if needs_recreate {
        let mut agent_config = state
            .full_config
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .clone();
        {
            let mut orch = state.orchestrator.lock().await;
            if let Some(agent) = orch.get_agent_mut(&name) {
                agent_config.default_provider = agent.provider_name().to_string();
                agent_config.default_model = agent.model_name().to_string();
                agent_config.identity.system_prompt = agent.system_prompt().to_string();
            }
        } // lock released before potentially slow await

        if let Some(p) = provider
            && !p.is_empty()
        {
            agent_config.default_provider = p.to_string();
            agent_config.llm.provider = p.to_string(); // sync
        }
        if let Some(m) = model
            && !m.is_empty()
        {
            agent_config.default_model = m.to_string();
            agent_config.llm.model = m.to_string(); // sync
        }
        if let Some(sp) = system_prompt {
            agent_config.identity.system_prompt = sp.to_string();
        }
        agent_config.identity.name = name.clone();

        // Critical: inject per-provider API key from DB
        apply_provider_config_from_db(&state.db, &mut agent_config);

        // Re-create agent with sync Agent::new() — fast, no MCP hang
        match bizclaw_agent::Agent::new(agent_config) {
            Ok(new_agent) => {
                let mut orch = state.orchestrator.lock().await;
                let role_str = role.unwrap_or("assistant").to_string();
                let desc_str = description.unwrap_or("").to_string();
                let agents_list = orch.list_agents();
                let current = agents_list
                    .iter()
                    .find(|a| a["name"].as_str() == Some(&name));
                let final_role = if role.is_some() {
                    role_str.clone()
                } else {
                    current
                        .and_then(|a| a["role"].as_str())
                        .unwrap_or("assistant")
                        .to_string()
                };
                let final_desc = if description.is_some() {
                    desc_str.clone()
                } else {
                    current
                        .and_then(|a| a["description"].as_str())
                        .unwrap_or("")
                        .to_string()
                };
                orch.remove_agent(&name);
                orch.add_agent(&name, &final_role, &final_desc, new_agent);
                tracing::info!("🔄 Agent '{}' re-created with new provider/model", name);
            }
            Err(e) => {
                tracing::warn!("⚠️ Agent '{}' re-create failed: {}", name, e);
            }
        }
    }

    // Phase 3: Persist to DB — always save metadata/prompt even without re-creation
    // Use DB record as fallback (NOT hardcoded "openai") to preserve user's provider choice
    {
        let db_agent = state.db.get_agent(&name).ok();
        let orch = state.orchestrator.lock().await;
        let agents_list = orch.list_agents();
        let current = agents_list
            .iter()
            .find(|a| a["name"].as_str() == Some(&name));
        let final_role = current
            .and_then(|a| a["role"].as_str())
            .unwrap_or("assistant");
        let final_desc = current
            .and_then(|a| a["description"].as_str())
            .unwrap_or("");
        // Provider fallback chain: explicit request → DB record → orchestrator live state → ""
        let final_provider = provider.unwrap_or_else(|| {
            current
                .and_then(|a| a["provider"].as_str())
                .filter(|p| !p.is_empty())
                .or_else(|| {
                    db_agent
                        .as_ref()
                        .map(|a| a.provider.as_str())
                        .filter(|p| !p.is_empty())
                })
                .unwrap_or("")
        });
        let final_model = model.unwrap_or_else(|| {
            current
                .and_then(|a| a["model"].as_str())
                .filter(|m| !m.is_empty())
                .or_else(|| {
                    db_agent
                        .as_ref()
                        .map(|a| a.model.as_str())
                        .filter(|m| !m.is_empty())
                })
                .unwrap_or("")
        });
        let final_prompt = system_prompt.unwrap_or_else(|| {
            current
                .and_then(|a| a["system_prompt"].as_str())
                .or_else(|| db_agent.as_ref().map(|a| a.system_prompt.as_str()))
                .unwrap_or("")
        });
        if let Err(e) = state.db.upsert_agent(
            &name,
            final_role,
            final_desc,
            final_provider,
            final_model,
            final_prompt,
        ) {
            tracing::warn!("DB persist failed for agent '{}': {}", name, e);
        }
    }

    // Persist to legacy agents.json
    {
        let orch = state.orchestrator.lock().await;
        let agents_path = state
            .config_path
            .parent()
            .unwrap_or(std::path::Path::new("."))
            .join("agents.json");
        orch.save_agents_metadata(&agents_path);
    }

    Json(serde_json::json!({
        "ok": true,
        "message": format!("Agent '{}' updated", name),
    }))
}

/// Chat with a specific agent.
pub async fn agent_chat(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(name): axum::extract::Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let message = body["message"].as_str().unwrap_or("");
    if message.is_empty() {
        return Json(serde_json::json!({"ok": false, "error": "Empty message"}));
    }

    let mut orch = state.orchestrator.lock().await;
    match orch.send_to(&name, message).await {
        Ok(response) => Json(serde_json::json!({
            "ok": true,
            "agent": name,
            "response": response,
        })),
        Err(e) => {
            tracing::error!("[agent_chat:{name}] {e}");
            Json(serde_json::json!({
                "ok": false,
                "error": "Agent processing failed",
            }))
        }
    }
}

/// Broadcast message to all agents.
pub async fn agent_broadcast(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let message = body["message"].as_str().unwrap_or("");
    if message.is_empty() {
        return Json(serde_json::json!({"ok": false, "error": "Empty message"}));
    }

    let mut orch = state.orchestrator.lock().await;
    let results = orch.broadcast(message).await;
    let responses: Vec<serde_json::Value> = results
        .into_iter()
        .map(|(name, result)| match result {
            Ok(response) => serde_json::json!({
                "agent": name,
                "ok": true,
                "response": response,
            }),
            Err(e) => {
                tracing::error!("[broadcast:{name}] {e}");
                serde_json::json!({
                    "agent": name,
                    "ok": false,
                    "error": "Agent processing failed",
                })
            }
        })
        .collect();

    Json(serde_json::json!({
        "ok": true,
        "responses": responses,
    }))
}
