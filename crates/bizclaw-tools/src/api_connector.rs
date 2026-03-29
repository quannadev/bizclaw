//! API Connector tool — Pre-configured API endpoints for safe data updates
//!
//! This is the WRITE path complement to `db_query` (READ path).
//! Agents use this to update data through authenticated REST APIs,
//! which have server-side validation — safer than direct DB writes.
//!
//! # Config format (`data/api-endpoints.json`):
//! ```json
//! {
//!   "endpoints": [
//!     {
//!       "id": "update_order_status",
//!       "url": "https://api.myapp.com/orders/{id}/status",
//!       "method": "PATCH",
//!       "auth_header": "Authorization",
//!       "auth_value": "vault://api_myapp_token",
//!       "description": "Cập nhật trạng thái đơn hàng",
//!       "allowed_fields": ["status", "note"],
//!       "dangerous": false
//!     }
//!   ]
//! }
//! ```

use async_trait::async_trait;
use bizclaw_core::error::Result;
use bizclaw_core::traits::Tool;
use bizclaw_core::types::{ToolDefinition, ToolResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A pre-configured API endpoint profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiEndpointProfile {
    /// Unique identifier (e.g., "update_order_status")
    pub id: String,

    /// Base URL (can contain `{param}` placeholders)
    pub url: String,

    /// HTTP method: GET, POST, PUT, PATCH, DELETE
    pub method: String,

    /// Header name for authentication (e.g., "Authorization")
    #[serde(default = "default_auth_header")]
    pub auth_header: String,

    /// Auth value or vault:// URI (e.g., "vault://api_token" or "Bearer sk-xxx")
    #[serde(default)]
    pub auth_value: String,

    /// Human-readable description
    #[serde(default)]
    pub description: String,

    /// Fields that are allowed in the request body (empty = allow all)
    #[serde(default)]
    pub allowed_fields: Vec<String>,

    /// If true, requires human approval before executing
    #[serde(default)]
    pub dangerous: bool,

    /// Whether this endpoint is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Extra default headers
    #[serde(default)]
    pub headers: HashMap<String, String>,
}

fn default_auth_header() -> String {
    "Authorization".into()
}
fn default_true() -> bool {
    true
}

/// Root config for `api-endpoints.json`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ApiEndpointConfig {
    pub endpoints: Vec<ApiEndpointProfile>,
}

/// API Connector tool — safe data updates via pre-configured endpoints.
pub struct ApiConnectorTool {
    endpoints: HashMap<String, ApiEndpointProfile>,
}

impl ApiConnectorTool {
    pub fn new() -> Self {
        Self::load_from(std::path::Path::new("data/api-endpoints.json"))
    }

    pub fn load_from(path: &std::path::Path) -> Self {
        let endpoints = match std::fs::read_to_string(path) {
            Ok(content) => match serde_json::from_str::<ApiEndpointConfig>(&content) {
                Ok(config) => {
                    let mut map = HashMap::new();
                    for ep in config.endpoints {
                        if ep.enabled {
                            map.insert(ep.id.clone(), ep);
                        }
                    }
                    tracing::info!(
                        "🔗 Loaded {} API endpoint(s) from {}",
                        map.len(),
                        path.display()
                    );
                    map
                }
                Err(e) => {
                    tracing::warn!("⚠️ Failed to parse API endpoints config: {e}");
                    HashMap::new()
                }
            },
            Err(_) => {
                tracing::debug!("No API endpoints config at {}", path.display());
                HashMap::new()
            }
        };
        Self { endpoints }
    }
}

impl Default for ApiConnectorTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for ApiConnectorTool {
    fn name(&self) -> &str {
        "api_connector"
    }

    fn definition(&self) -> ToolDefinition {
        let ep_list = if self.endpoints.is_empty() {
            "No API endpoints configured. Add them in data/api-endpoints.json".to_string()
        } else {
            self.endpoints
                .values()
                .map(|ep| {
                    let danger = if ep.dangerous { "⚠️ " } else { "" };
                    format!(
                        "  - '{}' {} {} — {}{}",
                        ep.id, ep.method, ep.url, danger, ep.description
                    )
                })
                .collect::<Vec<_>>()
                .join("\n")
        };

        ToolDefinition {
            name: "api_connector".into(),
            description: format!(
                "Call pre-configured API endpoints to safely update data. Each endpoint has authentication, \
                field validation, and optional approval gates. Use this for WRITE operations \
                (create, update) — use db_query for READ operations.\n\n\
                Available endpoints:\n{}",
                ep_list
            ),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "endpoint_id": {
                        "type": "string",
                        "description": "ID of a pre-configured API endpoint"
                    },
                    "path_params": {
                        "type": "object",
                        "description": "URL path parameters to replace {placeholders} in the endpoint URL"
                    },
                    "body": {
                        "type": "object",
                        "description": "Request body (JSON object)"
                    },
                    "query_params": {
                        "type": "object",
                        "description": "URL query parameters"
                    }
                },
                "required": ["endpoint_id"]
            }),
        }
    }

    async fn execute(&self, arguments: &str) -> Result<ToolResult> {
        let args: serde_json::Value = serde_json::from_str(arguments)
            .map_err(|e| bizclaw_core::error::BizClawError::Tool(format!("Invalid JSON: {}", e)))?;

        let ep_id = args["endpoint_id"].as_str().ok_or_else(|| {
            bizclaw_core::error::BizClawError::Tool("Missing 'endpoint_id'".into())
        })?;

        // Look up endpoint
        let endpoint = match self.endpoints.get(ep_id) {
            Some(ep) => ep,
            None => {
                let available = self
                    .endpoints
                    .keys()
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(", ");
                return Ok(ToolResult {
                    tool_call_id: String::new(),
                    output: format!(
                        "❌ Unknown endpoint '{}'. Available: [{}]",
                        ep_id, available
                    ),
                    success: false,
                });
            }
        };

        // Check dangerous flag
        if endpoint.dangerous {
            return Ok(ToolResult {
                tool_call_id: String::new(),
                output: format!(
                    "⚠️ Endpoint '{}' is marked as dangerous and requires human approval. \
                    Please ask the operator to approve this action first.",
                    ep_id
                ),
                success: false,
            });
        }

        // Build URL with path params
        let mut url = endpoint.url.clone();
        if let Some(params) = args["path_params"].as_object() {
            for (key, val) in params {
                if let Some(v) = val.as_str() {
                    url = url.replace(&format!("{{{}}}", key), v);
                }
            }
        }

        // Add query params
        if let Some(qp) = args["query_params"].as_object() {
            let query_string: Vec<String> = qp
                .iter()
                .filter_map(|(k, v)| {
                    v.as_str()
                        .map(|s| format!("{}={}", k, urlencoding::encode(s)))
                })
                .collect();
            if !query_string.is_empty() {
                let separator = if url.contains('?') { "&" } else { "?" };
                url = format!("{}{}{}", url, separator, query_string.join("&"));
            }
        }

        // Validate body fields
        if let Some(body) = args["body"].as_object()
            && !endpoint.allowed_fields.is_empty()
        {
            for key in body.keys() {
                if !endpoint.allowed_fields.contains(key) {
                    return Ok(ToolResult {
                        tool_call_id: String::new(),
                        output: format!(
                            "🛡️ Field '{}' is not allowed for endpoint '{}'. Allowed: {:?}",
                            key, ep_id, endpoint.allowed_fields
                        ),
                        success: false,
                    });
                }
            }
        }

        // Resolve auth via Vault
        let vault = bizclaw_security::vault::Vault::new();
        let auth_value = if endpoint.auth_value.is_empty() {
            None
        } else {
            Some(vault.resolve_or_passthrough(&endpoint.auth_value))
        };

        // Make the request
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| {
                bizclaw_core::error::BizClawError::Tool(format!("HTTP client error: {}", e))
            })?;

        let method = endpoint.method.to_uppercase();
        let mut request = match method.as_str() {
            "GET" => client.get(&url),
            "POST" => client.post(&url),
            "PUT" => client.put(&url),
            "PATCH" => client.patch(&url),
            "DELETE" => client.delete(&url),
            _ => {
                return Ok(ToolResult {
                    tool_call_id: String::new(),
                    output: format!("❌ Unsupported method: {}", method),
                    success: false,
                });
            }
        };

        // Set auth header
        if let Some(ref auth) = auth_value {
            request = request.header(&endpoint.auth_header, auth);
        }

        // Set extra headers
        for (k, v) in &endpoint.headers {
            request = request.header(k.as_str(), v.as_str());
        }

        // Set body
        if let Some(body) = args.get("body")
            && !body.is_null()
        {
            request = request
                .header("Content-Type", "application/json")
                .json(body);
        }

        let start = std::time::Instant::now();
        match request.send().await {
            Ok(response) => {
                let status = response.status();
                let elapsed = start.elapsed();
                let body_text = response
                    .text()
                    .await
                    .unwrap_or_else(|_| "[empty]".to_string());

                // Truncate response
                let max_len = 3000;
                let body_display = if body_text.len() > max_len {
                    format!(
                        "{}... [truncated, total {} bytes]",
                        &body_text[..max_len],
                        body_text.len()
                    )
                } else {
                    body_text
                };

                let success = status.is_success();
                let icon = if success { "✅" } else { "❌" };

                Ok(ToolResult {
                    tool_call_id: String::new(),
                    output: format!(
                        "{} {} {} → {} ({}ms)\n\nResponse:\n{}",
                        icon,
                        method,
                        url,
                        status,
                        elapsed.as_millis(),
                        body_display
                    ),
                    success,
                })
            }
            Err(e) => Ok(ToolResult {
                tool_call_id: String::new(),
                output: format!("❌ Request failed: {}", e),
                success: false,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_config() {
        let json = r#"{
            "endpoints": [
                {
                    "id": "update_order",
                    "url": "https://api.example.com/orders/{id}",
                    "method": "PATCH",
                    "auth_value": "Bearer test-token",
                    "description": "Update order status",
                    "allowed_fields": ["status", "note"],
                    "dangerous": false
                },
                {
                    "id": "delete_user",
                    "url": "https://api.example.com/users/{id}",
                    "method": "DELETE",
                    "description": "Delete user — requires approval",
                    "dangerous": true
                }
            ]
        }"#;

        let config: ApiEndpointConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.endpoints.len(), 2);
        assert_eq!(config.endpoints[0].id, "update_order");
        assert_eq!(config.endpoints[0].allowed_fields, vec!["status", "note"]);
        assert!(!config.endpoints[0].dangerous);
        assert!(config.endpoints[1].dangerous);
    }

    #[test]
    fn test_load_nonexistent() {
        let tool = ApiConnectorTool::load_from(std::path::Path::new("/tmp/nonexistent_api.json"));
        assert_eq!(tool.endpoints.len(), 0);
    }

    #[test]
    fn test_default_config() {
        let config = ApiEndpointConfig::default();
        assert!(config.endpoints.is_empty());
    }

    #[test]
    fn test_field_validation_config() {
        let json = r#"{"endpoints": [{
            "id": "test_ep",
            "url": "https://api.example.com/test",
            "method": "POST",
            "allowed_fields": ["name", "email"]
        }]}"#;

        let config: ApiEndpointConfig = serde_json::from_str(json).unwrap();
        let ep = &config.endpoints[0];
        assert!(ep.allowed_fields.contains(&"name".to_string()));
        assert!(ep.allowed_fields.contains(&"email".to_string()));
        assert!(!ep.allowed_fields.contains(&"password".to_string()));
    }
}
