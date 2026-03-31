//! Channel webhooks and inbound message handlers.
//! Handles generic webhooks, Discord, Zalo OA, WhatsApp, Messenger, Xiaozhi.
//! Provider management extracted to routes/providers.rs.

use axum::{Json, extract::State};
use std::sync::Arc;
use crate::server::AppState;
use super::{
    load_channel_instances, safe_truncate, save_channel_instances,
};

// Re-export provider functions for backward compat
pub use super::providers::{
    list_providers, create_provider, delete_provider,
    update_provider, fetch_provider_models, list_channels,
    ollama_models, brain_scan_models,
};

/// Webhook inbound — receives external messages, routes to bound agent, replies.
/// POST /api/v1/webhook/inbound
/// Body: {"content": "message", "sender_id": "user1", "thread_id": "optional"}
/// Header: X-Webhook-Signature (optional HMAC-SHA256)
pub async fn webhook_inbound(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    body: String,
) -> Json<serde_json::Value> {
    // Find webhook channel instance bound to an agent
    let instances = load_channel_instances(&state);
    let webhook_instance = instances.iter().find(|i| {
        i["channel_type"].as_str() == Some("webhook")
            && i["enabled"].as_bool() == Some(true)
            && !i["agent_name"].as_str().unwrap_or("").is_empty()
    });

    let (agent_name, outbound_url, secret) = match webhook_instance {
        Some(inst) => {
            let agent = inst["agent_name"].as_str().unwrap_or("").to_string();
            let outbound = inst["config"]["webhook_url"]
                .as_str()
                .unwrap_or("")
                .to_string();
            let sec = inst["config"]["webhook_secret"]
                .as_str()
                .unwrap_or("")
                .to_string();
            (agent, outbound, sec)
        }
        None => {
            return Json(serde_json::json!({
                "ok": false,
                "error": "No webhook channel bound to an agent. Create one in Dashboard → Channels."
            }));
        }
    };

    // Verify signature if secret configured (HMAC-SHA256)
    if !secret.is_empty() {
        let sig = headers
            .get("x-webhook-signature")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        use hmac::{Hmac, Mac};
        use sha2::Sha256;
        type HmacSha256 = Hmac<Sha256>;
        let mut mac =
            HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC can take key of any size");
        mac.update(body.as_bytes());
        let expected = format!("{:x}", mac.finalize().into_bytes());
        if expected != sig {
            return Json(serde_json::json!({"ok": false, "error": "Invalid webhook signature"}));
        }
    }

    // Parse message
    let json: serde_json::Value = match serde_json::from_str(&body) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!("[security] Invalid JSON from client: {e}");
            return Json(serde_json::json!({"ok": false, "error": "Invalid JSON format"}));
        }
    };
    let content = json["content"].as_str().unwrap_or("").to_string();
    let sender = json["sender_id"]
        .as_str()
        .unwrap_or("webhook-user")
        .to_string();
    if content.is_empty() {
        return Json(serde_json::json!({"ok": false, "error": "'content' field required"}));
    }

    tracing::info!(
        "[webhook] {} → agent '{}': {}",
        sender,
        agent_name,
        safe_truncate(&content, 100)
    );

    // Route to agent
    let response = {
        let mut orch = state.orchestrator.lock().await;
        match orch.send_to(&agent_name, &content).await {
            Ok(r) => r,
            Err(e) => format!("⚠️ Agent error: {e}"),
        }
    };

    // Also forward reply to outbound URL if configured
    if !outbound_url.is_empty() {
        let reply_body = serde_json::json!({
            "content": response,
            "sender_id": agent_name,
            "thread_id": json["thread_id"].as_str().unwrap_or("webhook"),
            "in_reply_to": content,
        });
        let client = reqwest::Client::new();
        if let Err(e) = client.post(&outbound_url).json(&reply_body).send().await {
            tracing::error!("[webhook] Outbound forward failed: {e}");
        }
    }

    Json(serde_json::json!({
        "ok": true,
        "response": response,
        "agent": agent_name,
    }))
}

/// Spawn a Telegram polling loop that routes messages to a specific agent.
/// Reused by both save_channel_instance (manual) and auto_connect_channels (startup).
pub async fn spawn_telegram_polling(
    state: Arc<AppState>,
    agent_name: String,
    bot_token: String,
    instance_id: String,
) {
    // Disconnect existing bot for this agent if any
    {
        let mut bots = state.telegram_bots.lock().await;
        if let Some(existing) = bots.remove(&agent_name) {
            existing.abort_handle.notify_one();
            tracing::info!(
                "[telegram] Disconnecting existing bot for agent '{}'",
                agent_name
            );
            tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
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
    let bot_username = match tg.get_me().await {
        Ok(me) => me.username.unwrap_or_default(),
        Err(e) => {
            tracing::error!(
                "[telegram] Bot token invalid for instance '{}': {}",
                instance_id,
                e
            );
            return;
        }
    };
    tracing::info!(
        "[telegram] @{} connected → agent '{}' (instance: {})",
        bot_username,
        agent_name,
        instance_id
    );

    // Spawn polling loop
    let stop = Arc::new(tokio::sync::Notify::new());
    let stop_rx = stop.clone();
    let state_clone = state.clone();
    let agent_name_clone = agent_name.clone();
    let bot_token_for_state = bot_token.clone();

    tokio::spawn(async move {
        let mut channel = bizclaw_channels::telegram::TelegramChannel::new(
            bizclaw_channels::telegram::TelegramConfig {
                bot_token: bot_token.clone(),
                enabled: true,
                poll_interval: 1,
            },
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
                                    let _ = channel.send_typing(chat_id, message_thread_id).await;

                                    // Route to agent
                                    let response = {
                                        let mut orch = state_clone.orchestrator.lock().await;
                                        match orch.send_to(&agent_name_clone, &text).await {
                                            Ok(r) => r,
                                            Err(e) => format!("⚠️ Agent error: {e}"),
                                        }
                                    };

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
                bot_token: bot_token_for_state,
                bot_username: bot_username.clone(),
                abort_handle: stop,
            },
        );
    }
}

/// Spawn a Discord Gateway listener that routes messages to a specific agent.
pub async fn spawn_discord_gateway(
    state: Arc<AppState>,
    agent_name: String,
    bot_token: String,
    instance_id: String,
) {
    use futures::StreamExt;

    let discord =
        bizclaw_channels::discord::DiscordChannel::new(bizclaw_channels::discord::DiscordConfig {
            bot_token: bot_token.clone(),
            enabled: true,
            intents: 33281, // GUILDS | GUILD_MESSAGES | MESSAGE_CONTENT
        });

    // Verify bot token
    match discord.get_me().await {
        Ok(me) => {
            tracing::info!(
                "[discord] Bot {} connected → agent '{}' (instance: {})",
                me.username,
                agent_name,
                instance_id
            );
        }
        Err(e) => {
            tracing::error!(
                "[discord] Bot token invalid for instance '{}': {}",
                instance_id,
                e
            );
            return;
        }
    }

    let gateway = discord.start_gateway();
    let state_clone = state.clone();
    let agent_name_clone = agent_name.clone();

    tokio::spawn(async move {
        let mut stream = gateway;
        let reply_client = bizclaw_channels::discord::DiscordChannel::new(
            bizclaw_channels::discord::DiscordConfig {
                bot_token: bot_token.clone(),
                enabled: true,
                intents: 33281,
            },
        );

        while let Some(msg) = stream.next().await {
            let channel_id = msg.thread_id.clone();
            let text = msg.content.clone();
            let sender = msg.sender_name.clone().unwrap_or_default();

            tracing::info!(
                "[discord] {} → agent '{}': {}",
                sender,
                agent_name_clone,
                safe_truncate(&text, 100)
            );

            // Send typing indicator
            let _ = reply_client.send_typing_indicator(&channel_id).await;

            // Route to agent
            let response = {
                let mut orch = state_clone.orchestrator.lock().await;
                match orch.send_to(&agent_name_clone, &text).await {
                    Ok(r) => r,
                    Err(e) => format!("⚠️ Agent error: {e}"),
                }
            };

            // Reply via Discord
            if let Err(e) = reply_client.send_message(&channel_id, &response).await {
                tracing::error!("[discord] Reply failed: {e}");
            }
        }
        tracing::warn!(
            "[discord] Gateway stream ended for agent '{}'",
            agent_name_clone
        );
    });
}

/// Auto-connect all enabled channel instances on startup.
/// Called from server::start() after AppState is built.
pub async fn resolve_agent_for_channel(state: &AppState, channel: &str) -> Option<String> {
    let agent_channels_path = state
        .config_path
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .join("agent-channels.json");
    if agent_channels_path.exists()
        && let Ok(content) = std::fs::read_to_string(&agent_channels_path)
        && let Ok(bindings) = serde_json::from_str::<serde_json::Value>(&content)
        && let Some(obj) = bindings.as_object()
    {
        for (agent, channels) in obj {
            if let Some(arr) = channels.as_array()
                && arr.iter().any(|c| c.as_str() == Some(channel))
            {
                return Some(agent.clone());
            }
        }
    }
    None
}

pub async fn dispatch_to_channel_agent(state: &AppState, channel: &str, content: &str) -> String {
    let target = resolve_agent_for_channel(state, channel).await;
    let mut orch = state.orchestrator.lock().await;

    if let Some(agent_name) = target {
        match orch.send_to(&agent_name, content).await {
            Ok(r) => return r,
            Err(e) => {
                tracing::warn!("Failed to route to mapped agent '{}': {}. Falling back to default.", agent_name, e);
            }
        }
    }

    match orch.send(content).await {
        Ok(r) => r,
        Err(e) => format!("⚠️ Agent error: {e}"),
    }
}

// ── Fallback: if no telegram instances, check config.toml for bot_token ──
pub async fn auto_connect_channels(state: Arc<AppState>) {
    let mut instances = load_channel_instances(&state);
    let mut connected = 0;

    // ── Fallback: if no telegram instances, check config.toml for bot_token ──
    let has_telegram_instance = instances
        .iter()
        .any(|i| i["channel_type"].as_str() == Some("telegram"));
    if !has_telegram_instance {
        // Extract telegram config data (owned) to avoid holding MutexGuard across await
        let tg_data = {
            let cfg = state.full_config.lock().unwrap_or_else(|p| p.into_inner());
            cfg.channel.telegram.first().and_then(|tg| {
                if tg.enabled && !tg.bot_token.is_empty() {
                    Some((tg.bot_token.clone(), tg.allowed_chat_ids.clone()))
                } else {
                    None
                }
            })
        }; // MutexGuard dropped here

        if let Some((bot_token, chat_ids)) = tg_data {
            // Find which agent to bind to — check agent-channels.json first
            let mut target_agent = resolve_agent_for_channel(&state, "telegram").await.unwrap_or_default();
            // Fallback: bind to first agent available
            if target_agent.is_empty() {
                let orch = state.orchestrator.lock().await;
                let agents = orch.list_agents();
                if let Some(first) = agents.first() {
                    target_agent = first["name"].as_str().unwrap_or("").to_string();
                }
            }

            if !target_agent.is_empty() {
                let instance_id = format!("telegram_config_{}", chrono::Utc::now().timestamp());
                let inst = serde_json::json!({
                    "id": instance_id,
                    "name": "Telegram Bot (auto)",
                    "channel_type": "telegram",
                    "enabled": true,
                    "agent_name": target_agent,
                    "config": {
                        "bot_token": bot_token,
                        "allowed_chat_ids": chat_ids.iter().map(|id| id.to_string()).collect::<Vec<_>>().join(", "),
                    },
                    "updated_at": chrono::Utc::now().to_rfc3339(),
                });
                instances.push(inst);
                save_channel_instances(&state, &instances);
                tracing::info!(
                    "[auto-connect] Migrated config.toml telegram → channel instance bound to '{}'",
                    target_agent
                );
            }
        }
    }

    // ── Connect all enabled instances ──
    for inst in &instances {
        let enabled = inst["enabled"].as_bool().unwrap_or(false);
        if !enabled {
            continue;
        }
        let channel_type = inst["channel_type"].as_str().unwrap_or("");
        let agent_name = inst["agent_name"].as_str().unwrap_or("");
        let instance_id = inst["id"].as_str().unwrap_or("");
        let cfg = &inst["config"];

        match channel_type {
            "telegram" if !agent_name.is_empty() => {
                let bot_token = cfg["bot_token"].as_str().unwrap_or("").to_string();
                if !bot_token.is_empty() {
                    spawn_telegram_polling(
                        state.clone(),
                        agent_name.to_string(),
                        bot_token,
                        instance_id.to_string(),
                    )
                    .await;
                    connected += 1;
                }
            }
            "discord" if !agent_name.is_empty() => {
                let bot_token = cfg["bot_token"].as_str().unwrap_or("").to_string();
                if !bot_token.is_empty() {
                    let s = state.clone();
                    let an = agent_name.to_string();
                    let iid = instance_id.to_string();
                    tokio::spawn(async move {
                        spawn_discord_gateway(s, an, bot_token, iid).await;
                    });
                    connected += 1;
                }
            }
            "webhook" if !agent_name.is_empty() => {
                // Webhook is passive — inbound via /api/v1/webhook/inbound
                // No polling needed, just log that it's ready
                tracing::info!(
                    "[webhook] Instance '{}' bound to agent '{}' — ready for inbound at /api/v1/webhook/inbound",
                    inst["name"].as_str().unwrap_or(instance_id),
                    agent_name
                );
                connected += 1;
            }
            _ => {}
        }
    }
    if connected > 0 {
        tracing::info!("📱 Auto-connected {} channel instance(s)", connected);
    }
}

/// Generate Zalo QR code for login.
pub async fn zalo_qr_code(State(_state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    use bizclaw_channels::zalo::client::auth::{ZaloAuth, ZaloCredentials};

    let creds = ZaloCredentials::default();
    let mut auth = ZaloAuth::new(creds);

    match auth.get_qr_code().await {
        Ok(qr) => Json(serde_json::json!({
            "ok": true,
            "qr_code": qr.image,
            "qr_id": qr.code,
            "imei": auth.credentials().imei,
            "instructions": [
                "1. Mở ứng dụng Zalo trên điện thoại",
                "2. Nhấn biểu tượng QR ở thanh tìm kiếm",
                "3. Quét mã QR này để đăng nhập",
                "4. Xác nhận đăng nhập trên điện thoại"
            ],
            "message": "Quét mã QR bằng Zalo trên điện thoại"
        })),
        Err(e) => {
            tracing::error!("[zalo_qr] {e}");
            Json(serde_json::json!({
                "ok": false,
                "error": "Không thể tạo mã QR Zalo",
                "fallback": "Vui lòng vào chat.zalo.me → F12 → Application → Cookies → Copy toàn bộ và paste vào ô Cookie bên dưới"
            }))
        }
    }
}

/// WhatsApp webhook verification (GET) — Meta sends this to verify endpoint.
pub async fn whatsapp_webhook_verify(
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
    State(state): State<Arc<AppState>>,
) -> axum::response::Response {
    let mode = params.get("hub.mode").map(|s| s.as_str()).unwrap_or("");
    let token = params
        .get("hub.verify_token")
        .map(|s| s.as_str())
        .unwrap_or("");
    let challenge = params
        .get("hub.challenge")
        .map(|s| s.as_str())
        .unwrap_or("");

    let expected_token = {
        let cfg = state.full_config.lock().unwrap_or_else(|p| p.into_inner());
        cfg.channel
            .whatsapp
            .first()
            .map(|w| w.webhook_verify_token.clone())
            .unwrap_or_default()
    };

    if mode == "subscribe" && token == expected_token {
        tracing::info!("WhatsApp webhook verified");
        axum::response::Response::builder()
            .status(200)
            .body(axum::body::Body::from(challenge.to_string()))
            .unwrap()
    } else {
        axum::response::Response::builder()
            .status(403)
            .body(axum::body::Body::from("Forbidden"))
            .unwrap()
    }
}

/// WhatsApp webhook handler (POST) — receives incoming messages from Meta.
pub async fn whatsapp_webhook(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let entry = &body["entry"];
    if let Some(entries) = entry.as_array() {
        for entry in entries {
            if let Some(changes) = entry["changes"].as_array() {
                for change in changes {
                    let value = &change["value"];
                    if let Some(messages) = value["messages"].as_array() {
                        for msg in messages {
                            let msg_type = msg["type"].as_str().unwrap_or("");
                            if msg_type != "text" {
                                continue;
                            }

                            let from = msg["from"].as_str().unwrap_or("").to_string();
                            let text = msg["text"]["body"].as_str().unwrap_or("").to_string();
                            let msg_id = msg["id"].as_str().unwrap_or("").to_string();

                            if text.is_empty() {
                                continue;
                            }

                            tracing::info!("[whatsapp] Message from {from}: {text}");

                            let wa_config = {
                                let cfg =
                                    state.full_config.lock().unwrap_or_else(|p| p.into_inner());
                                cfg.channel.whatsapp.first().cloned()
                            };

                            let agent_lock = state.agent.clone();
                            tokio::spawn(async move {
                                let response = {
                                    let mut agent = agent_lock.lock().await;
                                    if let Some(agent) = agent.as_mut() {
                                        match agent.process(&text).await {
                                            Ok(r) => r,
                                            Err(e) => format!("Error: {e}"),
                                        }
                                    } else {
                                        "Agent not available".to_string()
                                    }
                                };

                                if let Some(wa_cfg) = wa_config {
                                    let url = format!(
                                        "https://graph.facebook.com/v21.0/{}/messages",
                                        wa_cfg.phone_number_id
                                    );
                                    let reply = serde_json::json!({
                                        "messaging_product": "whatsapp",
                                        "to": from,
                                        "type": "text",
                                        "text": { "body": response },
                                        "context": { "message_id": msg_id },
                                    });
                                    let client = reqwest::Client::new();
                                    if let Err(e) = client
                                        .post(&url)
                                        .header(
                                            "Authorization",
                                            format!("Bearer {}", wa_cfg.access_token),
                                        )
                                        .json(&reply)
                                        .send()
                                        .await
                                    {
                                        tracing::error!("[whatsapp] Reply failed: {e}");
                                    }
                                }
                            });
                        }
                    }
                }
            }
        }
    }

    Json(serde_json::json!({"status": "ok"}))
}

// ═══ Xiaozhi Webhook Bridge ═══

/// Xiaozhi webhook inbound — receives voice commands from Xiaozhi Server.
/// POST /api/v1/xiaozhi/webhook
pub async fn xiaozhi_webhook(
    State(state): State<Arc<AppState>>,
    _headers: axum::http::HeaderMap,
    body: String,
) -> Json<serde_json::Value> {
    let start = std::time::Instant::now();

    let req: bizclaw_channels::xiaozhi::XiaozhiRequest = match serde_json::from_str(&body) {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!("[security] Invalid Xiaozhi request format: {e}");
            return Json(serde_json::json!({"ok": false, "error": "Invalid request format"}));
        }
    };

    if req.content.is_empty() {
        return Json(serde_json::json!({"ok": false, "error": "'content' field required"}));
    }

    tracing::info!(
        "[xiaozhi] Device {} → '{}' ({})",
        req.device_mac,
        safe_truncate(&req.content, 80),
        req.lang
    );

    let response = dispatch_to_channel_agent(&state, "xiaozhi", &req.content).await;
    let processing_ms = start.elapsed().as_millis() as u64;

    Json(serde_json::json!({
        "ok": true,
        "text": response,
        "agent": "default",
        "session_id": req.session_id,
        "device_mac": req.device_mac,
        "has_audio": false,
        "processing_ms": processing_ms,
        "timestamp": chrono::Utc::now().to_rfc3339(),
    }))
}


// ═══════════════════════════════════════════════════════════════════════
// ZALO OA WEBHOOK — Server-side Zalo without Android bridge
// ═══════════════════════════════════════════════════════════════════════

/// POST /api/v1/webhook/zalo-oa — Receive messages from Zalo Official Account.
pub async fn zalo_oa_webhook(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    body: axum::body::Bytes,
) -> Json<serde_json::Value> {
    let body_str = String::from_utf8_lossy(&body);

    tracing::info!("[zalo-oa] Webhook received: {} bytes", body.len());

    // ── 1. Validate MAC signature (optional but recommended) ──
    let cfg = state
        .full_config
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone();
    let zalo_configs = &cfg.channel.zalo;
    let oa_config = zalo_configs.iter().find(|z| z.mode == "official");

    if let Some(mac_header) = headers.get("X-ZaloOA-Signature").or(headers.get("mac"))
        && let (Some(config), Ok(mac_value)) = (oa_config, mac_header.to_str())
    {
        let app_secret = &config.official.app_secret;
        if !app_secret.is_empty() {
            use hmac::Mac;
            type HmacSha256 = hmac::Hmac<sha2::Sha256>;
            if let Ok(mut mac) = HmacSha256::new_from_slice(app_secret.as_bytes()) {
                mac.update(body_str.as_bytes());
                let expected = hex::encode(mac.finalize().into_bytes());
                if mac_value != expected {
                    tracing::warn!("[zalo-oa] Invalid MAC signature");
                    return Json(serde_json::json!({"error": "Invalid signature"}));
                }
                tracing::debug!("[zalo-oa] MAC signature validated ✓");
            }
        }
    }

    // ── 2. Parse the webhook event ──
    let event: serde_json::Value = match serde_json::from_str(&body_str) {
        Ok(v) => v,
        Err(e) => {
            tracing::error!("[zalo-oa] Failed to parse webhook body: {e}");
            return Json(serde_json::json!({"ok": false, "error": "Invalid JSON"}));
        }
    };

    let event_name = event["event_name"].as_str().unwrap_or("");
    let timestamp = event["timestamp"].as_str().unwrap_or("");

    tracing::info!("[zalo-oa] Event: {} at {}", event_name, timestamp);

    match event_name {
        "user_send_text" => {
            let sender_id = event["sender"]["id"].as_str().unwrap_or("");
            let message_text = event["message"]["text"].as_str().unwrap_or("");
            let msg_id = event["message"]["msg_id"].as_str().unwrap_or("");

            if sender_id.is_empty() || message_text.is_empty() {
                return Json(serde_json::json!({"ok": false, "error": "Missing sender or text"}));
            }

            tracing::info!(
                "[zalo-oa] Text from {}: '{}' (msg_id={})",
                sender_id,
                if message_text.len() > 50 {
                    &message_text[..50]
                } else {
                    message_text
                },
                msg_id
            );

            let agent_response = dispatch_to_channel_agent(&state, "zalo", message_text).await;

            if let Some(config) = oa_config {
                let access_token = &config.official.access_token;
                if !access_token.is_empty() {
                    let reply_payload = serde_json::json!({
                        "recipient": { "user_id": sender_id },
                        "message": { "text": agent_response }
                    });

                    let client = reqwest::Client::new();
                    match client
                        .post("https://openapi.zalo.me/v3.0/oa/message/cs")
                        .header("access_token", access_token.as_str())
                        .json(&reply_payload)
                        .send()
                        .await
                    {
                        Ok(resp) => {
                            let status = resp.status();
                            let reply_body =
                                resp.json::<serde_json::Value>().await.unwrap_or_default();
                            if status.is_success()
                                && reply_body["error"].as_i64().unwrap_or(-1) == 0
                            {
                                tracing::info!(
                                    "[zalo-oa] ✅ Replied to {} successfully",
                                    sender_id
                                );
                            } else {
                                let err_msg = reply_body["message"].as_str().unwrap_or("Unknown");
                                tracing::error!(
                                    "[zalo-oa] Reply failed: {} (code: {})",
                                    err_msg,
                                    reply_body["error"]
                                );
                            }
                        }
                        Err(e) => {
                            tracing::error!("[zalo-oa] Reply request failed: {e}");
                        }
                    }
                } else {
                    tracing::warn!("[zalo-oa] No access_token configured — cannot reply");
                }
            }

            Json(serde_json::json!({
                "ok": true,
                "event": "user_send_text",
                "sender": sender_id,
                "replied": true
            }))
        }

        "user_send_image" => {
            let sender_id = event["sender"]["id"].as_str().unwrap_or("");
            let image_url = event["message"]["attachments"]
                .as_array()
                .and_then(|a| a.first())
                .and_then(|a| a["payload"]["url"].as_str())
                .unwrap_or("");

            tracing::info!("[zalo-oa] Image from {}: {}", sender_id, image_url);

            Json(serde_json::json!({
                "ok": true,
                "event": "user_send_image",
                "sender": sender_id
            }))
        }

        "follow" => {
            let follower_id = event["follower"]["id"].as_str().unwrap_or("");
            tracing::info!("[zalo-oa] 🎉 New follower: {}", follower_id);

            if let Some(config) = oa_config {
                let access_token = &config.official.access_token;
                if !access_token.is_empty() {
                    let welcome = serde_json::json!({
                        "recipient": { "user_id": follower_id },
                        "message": { "text": "Xin chào! 👋 Cảm ơn bạn đã quan tâm. Tôi là trợ lý AI, hãy gửi tin nhắn để bắt đầu trò chuyện!" }
                    });
                    let client = reqwest::Client::new();
                    let _ = client
                        .post("https://openapi.zalo.me/v3.0/oa/message/cs")
                        .header("access_token", access_token.as_str())
                        .json(&welcome)
                        .send()
                        .await;
                    tracing::info!("[zalo-oa] Welcome message sent to {}", follower_id);
                }
            }

            Json(serde_json::json!({"ok": true, "event": "follow", "follower": follower_id}))
        }

        "unfollow" => {
            let follower_id = event["follower"]["id"].as_str().unwrap_or("");
            tracing::info!("[zalo-oa] 👋 Unfollowed by: {}", follower_id);
            Json(serde_json::json!({"ok": true, "event": "unfollow"}))
        }

        _ => {
            tracing::debug!("[zalo-oa] Unhandled event: {}", event_name);
            Json(serde_json::json!({"ok": true, "event": event_name, "handled": false}))
        }
    }
}
