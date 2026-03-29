//! WebAuth HTTP Proxy — OpenAI-compatible API server.
//!
//! Exposes:
//! - `POST /v1/chat/completions` — chat completion (with text-based tool call support)
//! - `GET  /v1/models` — list available WebAuth models
//! - `GET  /health` — health check
//! - `GET  /status` — provider status
//!
//! BizClaw connects to this proxy as a `custom` provider:
//! `custom:http://127.0.0.1:{PORT}/v1`

use axum::{
    Router,
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use serde_json::{Value, json};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing;

use crate::cdp::CdpClient;
use crate::providers::{WebProvider, find_provider_for_model};
use crate::types::{OpenAIChatRequest, OpenAIChatResponse, ProviderStatus};

use bizclaw_core::types::ToolDefinition;
use bizclaw_providers::text_tool_calls;

const LOG_TAG: &str = "[WebAuth-Proxy]";

/// Shared state for the proxy server.
pub struct ProxyState {
    /// CDP client for browser communication
    pub cdp: Option<CdpClient>,
    /// Available providers
    pub providers: Vec<Box<dyn WebProvider>>,
    /// Provider status cache
    pub status_cache: std::collections::HashMap<String, ProviderStatus>,
    /// CDP debugging port
    pub cdp_port: u16,
}

/// The WebAuth proxy server.
pub struct WebAuthProxy {
    state: Arc<RwLock<ProxyState>>,
}

impl WebAuthProxy {
    /// Create a new proxy with the given providers.
    pub fn new(providers: Vec<Box<dyn WebProvider>>, cdp_port: u16) -> Self {
        let mut status_cache = std::collections::HashMap::new();
        for p in &providers {
            status_cache.insert(p.id().to_string(), ProviderStatus::NotConfigured);
        }

        Self {
            state: Arc::new(RwLock::new(ProxyState {
                cdp: None,
                providers,
                status_cache,
                cdp_port,
            })),
        }
    }

    /// Connect to Chrome CDP.
    pub async fn connect_cdp(&self) -> Result<(), String> {
        let state = self.state.read().await;
        let ws_url = crate::cdp::find_chrome_ws_url(state.cdp_port).await?;
        drop(state);

        let cdp = CdpClient::connect(&ws_url).await?;
        let mut state = self.state.write().await;
        state.cdp = Some(cdp);
        tracing::info!("{} CDP connected", LOG_TAG);
        Ok(())
    }

    /// Build the axum router.
    pub fn router(&self) -> Router {
        let state = self.state.clone();
        Router::new()
            .route("/v1/chat/completions", post(handle_chat))
            .route("/v1/models", get(handle_models))
            .route("/health", get(handle_health))
            .route("/status", get(handle_status))
            .with_state(state)
    }

    /// Start the proxy server on the given port (0 = auto).
    pub async fn start(self, port: u16) -> Result<u16, String> {
        let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{}", port))
            .await
            .map_err(|e| format!("{} Bind failed: {}", LOG_TAG, e))?;

        let actual_port = listener
            .local_addr()
            .map_err(|e| format!("{} Could not get local addr: {}", LOG_TAG, e))?
            .port();

        tracing::info!(
            "{} Listening on http://127.0.0.1:{}/v1",
            LOG_TAG,
            actual_port
        );

        let router = self.router();

        tokio::spawn(async move {
            if let Err(e) = axum::serve(listener, router).await {
                tracing::error!("{} Server error: {}", LOG_TAG, e);
            }
        });

        Ok(actual_port)
    }
}

// ─── Route Handlers ────────────────────────────────────────────────────────

/// POST /v1/chat/completions
async fn handle_chat(
    State(state): State<Arc<RwLock<ProxyState>>>,
    axum::Json(request): axum::Json<OpenAIChatRequest>,
) -> impl IntoResponse {
    let model_id = &request.model;
    tracing::info!(
        "{} Chat request for model: {}, messages: {}",
        LOG_TAG,
        model_id,
        request.messages.len()
    );

    let state = state.read().await;

    // Find provider for model
    let provider = match find_provider_for_model(&state.providers, model_id) {
        Some(p) => p,
        None => {
            return (
                StatusCode::NOT_FOUND,
                axum::Json(json!({
                    "error": {
                        "message": format!("Model '{}' not found", model_id),
                        "type": "invalid_request_error"
                    }
                })),
            );
        }
    };

    // Check CDP connection
    let cdp = match &state.cdp {
        Some(c) => c,
        None => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                axum::Json(json!({
                    "error": {
                        "message": "CDP not connected. Start Chrome with --remote-debugging-port",
                        "type": "service_unavailable"
                    }
                })),
            );
        }
    };

    // Convert messages to consolidated prompt
    let core_messages: Vec<bizclaw_core::types::Message> = request
        .messages
        .iter()
        .map(|m| {
            let role = match m.role.as_str() {
                "system" => bizclaw_core::types::Role::System,
                "assistant" => bizclaw_core::types::Role::Assistant,
                "tool" => bizclaw_core::types::Role::Tool,
                _ => bizclaw_core::types::Role::User,
            };
            bizclaw_core::types::Message {
                role,
                content: m.content.clone(),
                name: m.name.clone(),
                tool_call_id: m.tool_call_id.clone(),
                tool_calls: None,
            }
        })
        .collect();

    // Extract tool definitions
    let tool_defs: Vec<ToolDefinition> = request
        .tools
        .as_ref()
        .map(|tools| {
            tools
                .iter()
                .filter_map(|t| {
                    let func = t.get("function")?;
                    Some(ToolDefinition {
                        name: func.get("name")?.as_str()?.to_string(),
                        description: func
                            .get("description")
                            .and_then(|d| d.as_str())
                            .unwrap_or("")
                            .to_string(),
                        parameters: func.get("parameters").cloned().unwrap_or_else(|| json!({})),
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    // Determine provider type for prompt strategy
    let provider_type = match provider.id() {
        "chatgpt" => text_tool_calls::ProviderType::ChatGPT,
        "gemini" => text_tool_calls::ProviderType::Gemini,
        "claude" => text_tool_calls::ProviderType::Claude,
        "deepseek" => text_tool_calls::ProviderType::DeepSeek,
        "grok" => text_tool_calls::ProviderType::Grok,
        _ => text_tool_calls::ProviderType::Generic,
    };

    // Consolidate messages into single prompt
    let prompt = text_tool_calls::consolidate_messages(&core_messages, &tool_defs, provider_type);

    // Send to provider
    match provider.chat(cdp, &prompt).await {
        Ok(response_text) => {
            // Parse text-based tool calls if tools were provided
            if !tool_defs.is_empty() {
                let tool_calls = text_tool_calls::parse_text_tool_calls(&response_text);
                if !tool_calls.is_empty() {
                    tracing::info!(
                        "{} Extracted {} tool call(s) from response",
                        LOG_TAG,
                        tool_calls.len()
                    );

                    let clean_text =
                        text_tool_calls::strip_tool_call_text(&response_text, &tool_calls);

                    // Convert to OpenAI format
                    let tc_values: Vec<Value> = tool_calls
                        .iter()
                        .map(|tc| {
                            json!({
                                "id": tc.id,
                                "type": "function",
                                "function": {
                                    "name": tc.function.name,
                                    "arguments": tc.function.arguments,
                                }
                            })
                        })
                        .collect();

                    let resp = OpenAIChatResponse::with_tool_calls(
                        model_id,
                        if clean_text.is_empty() {
                            None
                        } else {
                            Some(clean_text)
                        },
                        tc_values,
                    );

                    return (
                        StatusCode::OK,
                        axum::Json(serde_json::to_value(resp).unwrap()),
                    );
                }
            }

            // No tool calls — text-only response
            let resp = OpenAIChatResponse::text(model_id, &response_text);
            (
                StatusCode::OK,
                axum::Json(serde_json::to_value(resp).unwrap()),
            )
        }
        Err(e) => {
            tracing::error!("{} Provider error: {}", LOG_TAG, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(json!({
                    "error": {
                        "message": e,
                        "type": "provider_error"
                    }
                })),
            )
        }
    }
}

/// GET /v1/models
async fn handle_models(State(state): State<Arc<RwLock<ProxyState>>>) -> impl IntoResponse {
    let state = state.read().await;
    let models: Vec<Value> = state
        .providers
        .iter()
        .flat_map(|p| {
            p.models().iter().map(|m| {
                json!({
                    "id": m.id,
                    "object": "model",
                    "created": 0,
                    "owned_by": format!("webauth-{}", p.id()),
                })
            })
        })
        .collect();

    axum::Json(json!({
        "object": "list",
        "data": models,
    }))
}

/// GET /health
async fn handle_health(State(state): State<Arc<RwLock<ProxyState>>>) -> impl IntoResponse {
    let state = state.read().await;
    let cdp_connected = state.cdp.is_some();
    let total_providers = state.providers.len();
    let authenticated = state
        .status_cache
        .values()
        .filter(|s| **s == ProviderStatus::Authenticated)
        .count();

    axum::Json(json!({
        "status": if cdp_connected { "ok" } else { "degraded" },
        "cdp_connected": cdp_connected,
        "cdp_port": state.cdp_port,
        "providers": {
            "total": total_providers,
            "authenticated": authenticated,
        }
    }))
}

/// GET /status
async fn handle_status(State(state): State<Arc<RwLock<ProxyState>>>) -> impl IntoResponse {
    let state = state.read().await;
    let providers: Vec<Value> = state
        .providers
        .iter()
        .map(|p| {
            let status = state
                .status_cache
                .get(p.id())
                .copied()
                .unwrap_or(ProviderStatus::NotConfigured);
            json!({
                "id": p.id(),
                "name": p.name(),
                "status": status.to_string(),
                "login_url": p.login_url(),
                "models": p.models().iter().map(|m| &m.id).collect::<Vec<_>>(),
            })
        })
        .collect();

    axum::Json(json!({
        "providers": providers,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proxy_creation() {
        let providers = crate::providers::create_all_providers();
        let proxy = WebAuthProxy::new(providers, 9222);
        // Just verify it creates without panic
        let _ = proxy.router();
    }
}
