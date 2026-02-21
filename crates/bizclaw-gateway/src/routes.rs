//! API route handlers for the gateway.

use axum::{extract::State, Json};
use std::sync::Arc;

use super::server::AppState;

/// Health check endpoint.
pub async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "service": "bizclaw-gateway",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

/// System information endpoint.
pub async fn system_info(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let uptime = state.start_time.elapsed();
    Json(serde_json::json!({
        "name": "BizClaw",
        "version": env!("CARGO_PKG_VERSION"),
        "platform": format!("{}/{}", std::env::consts::OS, std::env::consts::ARCH),
        "uptime_secs": uptime.as_secs(),
        "gateway": {
            "host": state.config.host,
            "port": state.config.port,
            "require_pairing": state.config.require_pairing,
        }
    }))
}

/// Get current configuration (sanitized — no secrets).
pub async fn get_config(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "gateway": {
            "host": state.config.host,
            "port": state.config.port,
            "require_pairing": state.config.require_pairing,
        }
    }))
}

/// List available providers.
pub async fn list_providers() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "providers": [
            {"name": "openai", "type": "cloud", "status": "available"},
            {"name": "anthropic", "type": "cloud", "status": "available"},
            {"name": "ollama", "type": "local", "status": "available"},
            {"name": "llamacpp", "type": "local", "status": "available"},
            {"name": "brain", "type": "local", "status": "available"},
        ]
    }))
}

/// List available channels.
pub async fn list_channels() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "channels": [
            {"name": "cli", "type": "interactive", "status": "available"},
            {"name": "zalo", "type": "messaging", "status": "available"},
            {"name": "telegram", "type": "messaging", "status": "available"},
            {"name": "discord", "type": "messaging", "status": "available"},
            {"name": "webhook", "type": "api", "status": "available"},
        ]
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::AppState;

    fn test_state() -> State<Arc<AppState>> {
        State(Arc::new(AppState {
            config: bizclaw_core::config::GatewayConfig::default(),
            start_time: std::time::Instant::now(),
        }))
    }

    #[tokio::test]
    async fn test_health_check() {
        let result = health_check().await;
        let json = result.0;
        assert_eq!(json["status"], "ok");
    }

    #[tokio::test]
    async fn test_system_info() {
        let result = system_info(test_state()).await;
        let json = result.0;
        assert_eq!(json["name"], "BizClaw");
        assert!(json["version"].is_string());
    }

    #[tokio::test]
    async fn test_list_providers() {
        let result = list_providers().await;
        let json = result.0;
        assert!(json["providers"].is_array());
        assert!(json["providers"].as_array().unwrap().len() >= 5);
    }

    #[tokio::test]
    async fn test_list_channels() {
        let result = list_channels().await;
        let json = result.0;
        assert!(json["channels"].is_array());
        let channels = json["channels"].as_array().unwrap();
        assert!(channels.iter().all(|c| c["status"] == "available"));
    }
}
