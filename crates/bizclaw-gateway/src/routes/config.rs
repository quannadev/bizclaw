//! Configuration API route handlers.
//!
//! Extracted from mod.rs to reduce God File size.
//! Handles: get_config, get_full_config, update_config.

use axum::{Json, extract::State};
use std::sync::Arc;

use super::internal_error;
use super::mask_secret;
use crate::server::AppState;

/// Get current configuration (sanitized — no API keys).
pub async fn get_config(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let cfg = state.full_config.lock().unwrap_or_else(|p| p.into_inner());
    Json(serde_json::json!({
        "default_provider": cfg.default_provider,
        "default_model": cfg.default_model,
        "default_temperature": cfg.default_temperature,
        "api_key_set": !cfg.api_key.is_empty(),
        "api_base_url": cfg.api_base_url,
        "identity": {
            "name": cfg.identity.name,
            "persona": cfg.identity.persona,
            "system_prompt": cfg.identity.system_prompt,
        },
        "gateway": {
            "host": cfg.gateway.host,
            "port": cfg.gateway.port,
            "require_pairing": cfg.gateway.require_pairing,
        },
        "memory": {
            "backend": cfg.memory.backend,
            "auto_save": cfg.memory.auto_save,
            "embedding_provider": cfg.memory.embedding_provider,
            "vector_weight": cfg.memory.vector_weight,
            "keyword_weight": cfg.memory.keyword_weight,
        },
        "autonomy": {
            "level": cfg.autonomy.level,
            "workspace_only": cfg.autonomy.workspace_only,
            "allowed_commands": cfg.autonomy.allowed_commands,
            "forbidden_paths": cfg.autonomy.forbidden_paths,
        },
        "brain": {
            "enabled": cfg.brain.enabled,
            "model_path": cfg.brain.model_path,
            "threads": cfg.brain.threads,
            "max_tokens": cfg.brain.max_tokens,
            "context_length": cfg.brain.context_length,
            "temperature": cfg.brain.temperature,
            "json_mode": cfg.brain.json_mode,
        },
        "runtime": {
            "kind": cfg.runtime.kind,
        },
        "tunnel": {
            "provider": cfg.tunnel.provider,
        },
        "secrets": {
            "encrypt": cfg.secrets.encrypt,
        },
        "mcp_servers": cfg.mcp_servers.iter().map(|s| {
            let mut masked_env = std::collections::HashMap::new();
            for (k, v) in &s.env {
                masked_env.insert(k.clone(), mask_secret(v));
            }
            serde_json::json!({
                "name": s.name, "command": s.command,
                "args": s.args, "env": masked_env, "enabled": s.enabled,
            })
        }).collect::<Vec<_>>(),
        "channels": {
            "telegram": cfg.channel.telegram.iter().map(|t| serde_json::json!({
                "name": t.name,
                "enabled": t.enabled,
                "bot_token": mask_secret(&t.bot_token),
                "bot_token_set": !t.bot_token.is_empty(),
                "allowed_chat_ids": t.allowed_chat_ids.iter().map(|id| id.to_string()).collect::<Vec<_>>().join(", "),
            })).collect::<Vec<_>>(),
            "zalo": cfg.channel.zalo.iter().map(|z| serde_json::json!({
                "name": z.name,
                "enabled": z.enabled,
                "mode": z.mode,
                "cookie_path": z.personal.cookie_path,
                "cookie": if z.personal.cookie_path.is_empty() { "".to_string() } else { "•••• (saved to file)".to_string() },
                "imei": z.personal.imei,
                "self_listen": z.personal.self_listen,
                "auto_reconnect": z.personal.auto_reconnect,
            })).collect::<Vec<_>>(),
            "discord": cfg.channel.discord.iter().map(|d| serde_json::json!({
                "name": d.name,
                "enabled": d.enabled,
                "bot_token": mask_secret(&d.bot_token),
                "bot_token_set": !d.bot_token.is_empty(),
                "allowed_channel_ids": d.allowed_channel_ids.iter().map(|id| id.to_string()).collect::<Vec<_>>().join(", "),
            })).collect::<Vec<_>>(),
            "email": cfg.channel.email.iter().map(|e| serde_json::json!({
                "enabled": e.enabled,
                "smtp_host": e.smtp_host,
                "smtp_port": e.smtp_port,
                "smtp_user": e.email,
                "smtp_pass": mask_secret(&e.password),
                "imap_host": e.imap_host,
                "imap_port": e.imap_port,
            })).collect::<Vec<_>>(),
            "whatsapp": cfg.channel.whatsapp.iter().map(|w| serde_json::json!({
                "enabled": w.enabled,
                "phone_number_id": w.phone_number_id,
                "access_token": mask_secret(&w.access_token),
                "business_id": w.business_id,
            })).collect::<Vec<_>>(),
            "webhook": cfg.channel.webhook.iter().map(|wh| serde_json::json!({
                "enabled": wh.enabled,
                "secret": mask_secret(&wh.secret),
                "secret_set": !wh.secret.is_empty(),
                "outbound_url": wh.outbound_url,
            })).collect::<Vec<_>>(),
        },
    }))
}

/// Get full config as TOML string for export/display.
pub async fn get_full_config(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let cfg = state.full_config.lock().unwrap_or_else(|p| p.into_inner());
    let toml_str = toml::to_string_pretty(&*cfg).unwrap_or_default();
    Json(serde_json::json!({
        "ok": true,
        "toml": toml_str,
        "config_path": state.config_path.display().to_string(),
    }))
}

/// Update config fields via JSON body.
pub async fn update_config(
    State(state): State<Arc<AppState>>,
    Json(req): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let mut cfg = state.full_config.lock().unwrap_or_else(|p| p.into_inner());

    // Update top-level fields + sync to LLM section
    // CRITICAL: create_provider() reads llm.* FIRST, so both must be in sync
    if let Some(v) = req.get("default_provider").and_then(|v| v.as_str()) {
        cfg.default_provider = v.to_string();
        cfg.llm.provider = v.to_string(); // sync
    }
    if let Some(v) = req.get("default_model").and_then(|v| v.as_str()) {
        cfg.default_model = v.to_string();
        cfg.llm.model = v.to_string(); // sync
    }
    if let Some(v) = req.get("default_temperature").and_then(|v| v.as_f64()) {
        cfg.default_temperature = v as f32;
        cfg.llm.temperature = v as f32; // sync
    }
    if let Some(v) = req.get("api_key").and_then(|v| v.as_str()) {
        cfg.api_key = v.to_string();
        cfg.llm.api_key = v.to_string(); // sync
    }
    if let Some(v) = req.get("api_base_url").and_then(|v| v.as_str()) {
        cfg.api_base_url = v.to_string();
        cfg.llm.endpoint = v.to_string(); // sync
    }

    // Update identity
    if let Some(id) = req.get("identity") {
        if let Some(v) = id.get("name").and_then(|v| v.as_str()) {
            cfg.identity.name = v.to_string();
        }
        if let Some(v) = id.get("persona").and_then(|v| v.as_str()) {
            cfg.identity.persona = v.to_string();
        }
        if let Some(v) = id.get("system_prompt").and_then(|v| v.as_str()) {
            cfg.identity.system_prompt = v.to_string();
        }
    }

    // Update memory
    if let Some(mem) = req.get("memory") {
        if let Some(v) = mem.get("backend").and_then(|v| v.as_str()) {
            cfg.memory.backend = v.to_string();
        }
        if let Some(v) = mem.get("auto_save").and_then(|v| v.as_bool()) {
            cfg.memory.auto_save = v;
        }
    }

    // Update autonomy
    if let Some(auto) = req.get("autonomy") {
        if let Some(v) = auto.get("level").and_then(|v| v.as_str()) {
            cfg.autonomy.level = v.to_string();
        }
        if let Some(v) = auto.get("workspace_only").and_then(|v| v.as_bool()) {
            cfg.autonomy.workspace_only = v;
        }
    }

    // Update brain
    if let Some(brain) = req.get("brain") {
        if let Some(v) = brain.get("enabled").and_then(|v| v.as_bool()) {
            cfg.brain.enabled = v;
        }
        if let Some(v) = brain.get("model_path").and_then(|v| v.as_str()) {
            cfg.brain.model_path = v.to_string();
        }
        if let Some(v) = brain.get("threads").and_then(|v| v.as_u64()) {
            cfg.brain.threads = v as u32;
        }
        if let Some(v) = brain.get("max_tokens").and_then(|v| v.as_u64()) {
            cfg.brain.max_tokens = v as u32;
        }
        if let Some(v) = brain.get("context_length").and_then(|v| v.as_u64()) {
            cfg.brain.context_length = v as u32;
        }
        if let Some(v) = brain.get("temperature").and_then(|v| v.as_f64()) {
            cfg.brain.temperature = v as f32;
        }
    }

    // Update MCP servers
    if let Some(mcp) = req.get("mcp_servers")
        && let Ok(servers) =
            serde_json::from_value::<Vec<bizclaw_core::config::McpServerEntry>>(mcp.clone())
    {
        cfg.mcp_servers = servers;
    }

    // Save to disk
    let content = toml::to_string_pretty(&*cfg).unwrap_or_default();
    let new_cfg = cfg.clone();

    // Build sync data for platform DB import
    let sync_data = serde_json::json!({
        "default_provider": new_cfg.default_provider,
        "default_model": new_cfg.default_model,
        "api_key": new_cfg.api_key,
        "api_base_url": new_cfg.api_base_url,
        "identity.name": new_cfg.identity.name,
        "identity.persona": new_cfg.identity.persona,
        "identity.system_prompt": new_cfg.identity.system_prompt,
        "brain.enabled": new_cfg.brain.enabled,
        "brain.model_path": new_cfg.brain.model_path,
        "brain.threads": new_cfg.brain.threads,
        "brain.max_tokens": new_cfg.brain.max_tokens,
        "brain.context_length": new_cfg.brain.context_length,
        "brain.temperature": new_cfg.brain.temperature,
        "updated_at": chrono::Utc::now().to_rfc3339(),
    });

    drop(cfg); // Release lock before file write + agent reinit

    match std::fs::write(&state.config_path, &content) {
        Ok(_) => {
            tracing::info!("✅ Config saved to {}", state.config_path.display());

            // Write config_sync.json for platform DB import
            if let Some(parent) = state.config_path.parent() {
                let sync_path = parent.join("config_sync.json");
                if let Ok(json) = serde_json::to_string_pretty(&sync_data) {
                    if let Err(e) = std::fs::write(&sync_path, json) {
                        tracing::warn!(
                            "Failed to write config sync file to {}: {e}",
                            sync_path.display()
                        );
                    }
                    // SECURITY: Set 0600 — file contains api_key
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        let _ = std::fs::set_permissions(
                            &sync_path,
                            std::fs::Permissions::from_mode(0o600),
                        );
                    }
                    tracing::info!("📋 Config sync file written to {}", sync_path.display());
                }
            }

            // Re-initialize Agent with new config (async, don't block response)
            let agent_lock = state.agent.clone();
            tokio::spawn(async move {
                match bizclaw_agent::Agent::new_with_mcp(new_cfg).await {
                    Ok(new_agent) => {
                        let mut guard = agent_lock.lock().await;
                        tracing::info!(
                            "🔄 Agent re-initialized: provider={}, tools={}",
                            new_agent.provider_name(),
                            new_agent.tool_count()
                        );
                        *guard = Some(new_agent);
                    }
                    Err(e) => tracing::warn!("⚠️ Agent re-init failed: {e}"),
                }
            });

            Json(serde_json::json!({"ok": true, "message": "Config saved — agent reloading"}))
        }
        Err(e) => internal_error("gateway", e),
    }
}
