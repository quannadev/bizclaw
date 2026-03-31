//! Telegram Bot ↔ Agent API route handlers.
//!
//! Connect/disconnect/status for Telegram bot polling loops
//! bound to specific agents.
//! Extracted from routes/mod.rs.

use axum::{Json, extract::State};
use std::sync::Arc;

use crate::server::AppState;
use super::safe_truncate;

/// Connect a Telegram bot to a specific agent.
/// Verifies the bot token, then spawns a polling loop.
pub async fn connect_telegram(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(agent_name): axum::extract::Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let bot_token = body["bot_token"].as_str().unwrap_or("").trim().to_string();
    if bot_token.is_empty() {
        return Json(serde_json::json!({"ok": false, "error": "bot_token is required"}));
    }

    // Check agent exists
    {
        let orch = state.orchestrator.lock().await;
        let agents = orch.list_agents();
        if !agents
            .iter()
            .any(|a| a["name"].as_str() == Some(&agent_name))
        {
            return Json(
                serde_json::json!({"ok": false, "error": format!("Agent '{}' not found", agent_name)}),
            );
        }
    }

    // Already connected? Disconnect first
    {
        let mut bots = state.telegram_bots.lock().await;
        if let Some(existing) = bots.remove(&agent_name) {
            existing.abort_handle.notify_one();
            tracing::info!(
                "[telegram] Disconnecting existing bot for agent '{}'",
                agent_name
            );
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }
    }

    // Verify bot token
    let tg = bizclaw_channels::telegram::TelegramChannel::new(
        bizclaw_channels::telegram::TelegramConfig {
            bot_token: bot_token.clone(),
            enabled: true,
            poll_interval: 1,
        },
    );
    let bot_info = match tg.get_me().await {
        Ok(me) => me,
        Err(e) => {
            tracing::warn!("[security] Telegram bot token verification failed: {e}");
            return Json(
                serde_json::json!({"ok": false, "error": "Invalid bot token — please check and try again"}),
            );
        }
    };
    let bot_username = bot_info.username.clone().unwrap_or_default();
    tracing::info!(
        "[telegram] Bot @{} verified for agent '{}'",
        bot_username,
        agent_name
    );

    // Spawn polling loop
    let stop = Arc::new(tokio::sync::Notify::new());
    let stop_rx = stop.clone();
    let state_clone = state.clone();
    let agent_name_clone = agent_name.clone();
    let bot_token_clone = bot_token.clone();

    tokio::spawn(async move {
        let mut channel = bizclaw_channels::telegram::TelegramChannel::new(
            bizclaw_channels::telegram::TelegramConfig {
                bot_token: bot_token_clone,
                enabled: true,
                poll_interval: 1,
            },
        );
        tracing::info!(
            "[telegram] Polling started for agent '{}'",
            agent_name_clone
        );

        loop {
            tokio::select! {
                _ = stop_rx.notified() => {
                    tracing::info!("[telegram] Polling stopped for agent '{}'", agent_name_clone);
                    break;
                }
                result = channel.get_updates() => {
                    match result {
                        Ok(updates) => {
                            for update in updates {
                                if let Some(msg) = update.to_incoming() {
                                    let parts: Vec<&str> = msg.thread_id.split(':').collect();
                                    let chat_id: i64 = parts[0].parse().unwrap_or(0);
                                    let message_thread_id: Option<i64> = parts.get(1).and_then(|id| id.parse().ok());

                                    let sender = msg.sender_name.clone().unwrap_or_default();
                                    let text = msg.content.clone();

                                    tracing::info!("[telegram] {} → agent '{}': {}", sender, agent_name_clone, safe_truncate(&text, 100));

                                    // Send typing indicator
                                    let _ = channel.send_typing(chat_id, message_thread_id).await;

                                    // Route to agent
                                    let response = {
                                        let mut orch = state_clone.orchestrator.lock().await;
                                        match orch.send_to(&agent_name_clone, &text).await {
                                            Ok(r) => r,
                                            Err(e) => format!("⚠️ Agent error: {e}"),
                                        }
                                    };

                                    // Reply via Telegram
                                    if let Err(e) = channel.send_message(chat_id, message_thread_id, &response).await {
                                        tracing::error!("[telegram] Reply failed: {e}");
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            tracing::error!("[telegram] Polling error for '{}': {e}", agent_name_clone);
                            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                        }
                    }
                }
            }
        }
    });

    // Save state
    {
        let mut bots = state.telegram_bots.lock().await;
        bots.insert(
            agent_name.clone(),
            crate::server::TelegramBotState {
                bot_token: bot_token.clone(),
                bot_username: bot_username.clone(),
                abort_handle: stop,
            },
        );
    }

    Json(serde_json::json!({
        "ok": true,
        "agent": agent_name,
        "bot_username": bot_username,
        "message": format!("@{} connected to agent '{}'", bot_username, agent_name),
    }))
}

/// Disconnect Telegram bot from an agent.
pub async fn disconnect_telegram(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(agent_name): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    let mut bots = state.telegram_bots.lock().await;
    if let Some(bot) = bots.remove(&agent_name) {
        bot.abort_handle.notify_one();
        tracing::info!(
            "[telegram] @{} disconnected from agent '{}'",
            bot.bot_username,
            agent_name
        );
        Json(serde_json::json!({
            "ok": true,
            "message": format!("@{} disconnected from agent '{}'", bot.bot_username, agent_name),
        }))
    } else {
        Json(
            serde_json::json!({"ok": false, "error": format!("No Telegram bot connected to agent '{}'", agent_name)}),
        )
    }
}

/// Get Telegram bot status for an agent.
pub async fn telegram_status(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(agent_name): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    let bots = state.telegram_bots.lock().await;
    if let Some(bot) = bots.get(&agent_name) {
        Json(serde_json::json!({
            "ok": true,
            "connected": true,
            "bot_username": bot.bot_username,
            "agent": agent_name,
        }))
    } else {
        Json(serde_json::json!({
            "ok": true,
            "connected": false,
            "agent": agent_name,
        }))
    }
}
