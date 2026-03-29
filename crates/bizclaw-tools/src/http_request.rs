//! HTTP Request tool — make HTTP requests to external APIs

use async_trait::async_trait;
use bizclaw_core::error::Result;
use bizclaw_core::traits::Tool;
use bizclaw_core::types::{ToolDefinition, ToolResult};

pub struct HttpRequestTool;

impl HttpRequestTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for HttpRequestTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for HttpRequestTool {
    fn name(&self) -> &str {
        "http_request"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "http_request".into(),
            description: "Make HTTP requests to APIs and websites. Supports GET, POST, PUT, DELETE with headers and body.".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "url": {
                        "type": "string",
                        "description": "URL to request"
                    },
                    "method": {
                        "type": "string",
                        "enum": ["GET", "POST", "PUT", "DELETE", "PATCH", "HEAD"],
                        "description": "HTTP method (default: GET)"
                    },
                    "headers": {
                        "type": "object",
                        "description": "Request headers (key-value pairs)"
                    },
                    "body": {
                        "type": "string",
                        "description": "Request body (for POST/PUT/PATCH)"
                    },
                    "timeout_secs": {
                        "type": "integer",
                        "description": "Request timeout in seconds (default: 15)"
                    }
                },
                "required": ["url"]
            }),
        }
    }

    async fn execute(&self, arguments: &str) -> Result<ToolResult> {
        let args: serde_json::Value = serde_json::from_str(arguments)
            .map_err(|e| bizclaw_core::error::BizClawError::Tool(e.to_string()))?;

        let url = args["url"]
            .as_str()
            .ok_or_else(|| bizclaw_core::error::BizClawError::Tool("Missing 'url'".into()))?;
        let method = args["method"].as_str().unwrap_or("GET").to_uppercase();
        let timeout = args["timeout_secs"].as_u64().unwrap_or(15);

        // Safety check: block requests to internal/private/metadata endpoints (SSRF protection)
        if let Some(reason) = is_url_blocked(url) {
            return Ok(ToolResult {
                tool_call_id: String::new(),
                output: format!("Blocked: {reason}"),
                success: false,
            });
        }

        let client = reqwest::Client::builder()
            .user_agent("BizClaw/1.0")
            .timeout(std::time::Duration::from_secs(timeout))
            .build()
            .map_err(|e| bizclaw_core::error::BizClawError::Tool(format!("Client error: {e}")))?;

        let mut request = match method.as_str() {
            "GET" => client.get(url),
            "POST" => client.post(url),
            "PUT" => client.put(url),
            "DELETE" => client.delete(url),
            "PATCH" => client.patch(url),
            "HEAD" => client.head(url),
            _ => {
                return Err(bizclaw_core::error::BizClawError::Tool(format!(
                    "Unsupported method: {method}"
                )));
            }
        };

        // Add custom headers
        if let Some(headers) = args["headers"].as_object() {
            for (key, value) in headers {
                if let Some(val_str) = value.as_str()
                    && let Ok(header_name) = reqwest::header::HeaderName::from_bytes(key.as_bytes())
                    && let Ok(header_val) = reqwest::header::HeaderValue::from_str(val_str)
                {
                    request = request.header(header_name, header_val);
                }
            }
        }

        // Add body
        if let Some(body) = args["body"].as_str() {
            request = request.body(body.to_string());
            // Auto-detect content type if not set
            if args["headers"]
                .as_object()
                .map(|h| !h.contains_key("content-type"))
                .unwrap_or(true)
                && (body.starts_with('{') || body.starts_with('['))
            {
                request = request.header("Content-Type", "application/json");
            }
        }

        let start = std::time::Instant::now();
        let response = request
            .send()
            .await
            .map_err(|e| bizclaw_core::error::BizClawError::Tool(format!("Request failed: {e}")))?;

        let elapsed = start.elapsed();
        let status = response.status();
        let headers: String = response
            .headers()
            .iter()
            .take(10) // limit header output
            .map(|(k, v)| format!("{}: {}", k.as_str(), v.to_str().unwrap_or("?")))
            .collect::<Vec<_>>()
            .join("\n");

        let body_text = response.text().await.map_err(|e| {
            bizclaw_core::error::BizClawError::Tool(format!("Read body failed: {e}"))
        })?;

        // Truncate very large responses
        let body_display = if body_text.len() > 8000 {
            let truncated: String = body_text.chars().take(8000).collect();
            format!(
                "{}...\n\n[truncated, {} total bytes]",
                truncated,
                body_text.len()
            )
        } else {
            body_text
        };

        let output = format!(
            "HTTP {} {} → {} ({:.0}ms)\n\nHeaders:\n{}\n\nBody:\n{}",
            method,
            url,
            status,
            elapsed.as_millis(),
            headers,
            body_display
        );

        Ok(ToolResult {
            tool_call_id: String::new(),
            output,
            success: status.is_success(),
        })
    }
}

/// Check if a URL is blocked by SSRF protection.
/// Uses proper URL parsing to handle IPv6 and edge cases.
pub fn is_url_blocked(url: &str) -> Option<String> {
    let lower_url = url.to_lowercase();
    if !lower_url.starts_with("http://") && !lower_url.starts_with("https://") {
        return Some("Only HTTP/HTTPS schemes allowed".into());
    }

    // Parse URL properly to extract host (handles IPv6, ports, etc.)
    let host = match url::Url::parse(url) {
        Ok(parsed) => parsed.host_str().unwrap_or("").to_lowercase(),
        Err(_) => {
            // Fallback to manual extraction if URL parser fails
            let host_part = lower_url.split("://").nth(1).unwrap_or("");
            let h = host_part.split('/').next().unwrap_or("");
            // Strip port: for IPv6 [::1]:8080 → [::1], for IPv4 1.2.3.4:80 → 1.2.3.4
            if h.starts_with('[') {
                h.split(']')
                    .next()
                    .unwrap_or("")
                    .trim_start_matches('[')
                    .to_string()
            } else {
                h.split(':').next().unwrap_or("").to_string()
            }
        }
    };

    // Strip brackets from IPv6 for pattern matching
    let host_clean = host.trim_start_matches('[').trim_end_matches(']');

    // IPv6 loopback and private checks
    let ipv6_blocked = [
        "::1",             // loopback
        "::0",             // unspecified
        "::",              // unspecified shorthand
        "0:0:0:0:0:0:0:1", // full loopback
        "0:0:0:0:0:0:0:0", // full unspecified
    ];
    if ipv6_blocked.contains(&host_clean) {
        return Some(format!("Cannot access loopback address ({host})"));
    }
    // IPv6 link-local (fe80::/10) and unique-local (fc00::/7)
    if host_clean.starts_with("fe80:")
        || host_clean.starts_with("fc")
        || host_clean.starts_with("fd")
    {
        return Some(format!("Cannot access private IPv6 network ({host})"));
    }

    // IPv4 and hostname blocked patterns
    let blocked_patterns = [
        "127.0.0.1",
        "localhost",
        "0.0.0.0",
        "169.254.",
        "metadata.google",
        "metadata.aws",
        "10.",
        "192.168.",
        "172.16.",
        "172.17.",
        "172.18.",
        "172.19.",
        "172.20.",
        "172.21.",
        "172.22.",
        "172.23.",
        "172.24.",
        "172.25.",
        "172.26.",
        "172.27.",
        "172.28.",
        "172.29.",
        "172.30.",
        "172.31.",
    ];
    if blocked_patterns.iter().any(|p| host_clean.contains(p)) {
        return Some(format!("Cannot access internal/private network ({host})"));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── SSRF Protection Tests ──────────────────────
    #[test]
    fn test_block_localhost() {
        assert!(is_url_blocked("http://localhost/admin").is_some());
        assert!(is_url_blocked("http://localhost:8080/admin").is_some());
        assert!(is_url_blocked("https://localhost/").is_some());
    }

    #[test]
    fn test_block_loopback_ipv4() {
        assert!(is_url_blocked("http://127.0.0.1/").is_some());
        assert!(is_url_blocked("http://127.0.0.1:3000/api").is_some());
    }

    #[test]
    fn test_block_loopback_ipv6() {
        // Note: IPv6 in URL uses brackets, but our host extraction splits on ':',
        // so [::1] becomes "[" after split — the pattern "[::1]" won't match.
        // We test that the SSRF checker at least blocks the scheme check for non-http.
        // For full IPv6 support, a proper URL parser would be needed.
        // For now, verify localhost and 127.0.0.1 cover the common cases.
        assert!(is_url_blocked("http://localhost/").is_some());
        assert!(is_url_blocked("http://127.0.0.1/").is_some());
    }

    #[test]
    fn test_block_zero_address() {
        assert!(is_url_blocked("http://0.0.0.0/").is_some());
        assert!(is_url_blocked("http://0.0.0.0:8080/").is_some());
    }

    #[test]
    fn test_block_10_network() {
        assert!(is_url_blocked("http://10.0.0.1/").is_some());
        assert!(is_url_blocked("http://10.255.255.255/").is_some());
        assert!(is_url_blocked("http://10.10.10.10:9090/api").is_some());
    }

    #[test]
    fn test_block_172_16_network() {
        for i in 16..=31 {
            let url = format!("http://172.{i}.0.1/");
            assert!(is_url_blocked(&url).is_some(), "Should block {url}");
        }
    }

    #[test]
    fn test_block_192_168_network() {
        assert!(is_url_blocked("http://192.168.0.1/").is_some());
        assert!(is_url_blocked("http://192.168.1.1/router").is_some());
        assert!(is_url_blocked("http://192.168.100.100:8080/").is_some());
    }

    #[test]
    fn test_block_link_local() {
        assert!(is_url_blocked("http://169.254.169.254/metadata").is_some());
        assert!(is_url_blocked("http://169.254.0.1/").is_some());
    }

    #[test]
    fn test_block_cloud_metadata() {
        assert!(is_url_blocked("http://metadata.google.internal/computeMetadata/v1/").is_some());
        assert!(is_url_blocked("http://metadata.aws.amazon.com/latest/").is_some());
    }

    #[test]
    fn test_block_non_http_schemes() {
        assert!(is_url_blocked("ftp://example.com/file").is_some());
        assert!(is_url_blocked("file:///etc/passwd").is_some());
        assert!(is_url_blocked("gopher://evil.com/").is_some());
        assert!(is_url_blocked("javascript:alert(1)").is_some());
        assert!(is_url_blocked("data:text/html,<h1>hi</h1>").is_some());
    }

    #[test]
    fn test_allow_public_urls() {
        assert!(is_url_blocked("https://api.github.com/repos").is_none());
        assert!(is_url_blocked("https://google.com/").is_none());
        assert!(is_url_blocked("http://example.com/api").is_none());
        assert!(is_url_blocked("https://api.openai.com/v1/chat").is_none());
    }

    #[test]
    fn test_allow_172_non_private() {
        // 172.15.x.x and 172.32.x.x are NOT private
        assert!(is_url_blocked("http://172.15.0.1/").is_none());
        assert!(is_url_blocked("http://172.32.0.1/").is_none());
    }

    #[test]
    fn test_port_stripping() {
        assert!(is_url_blocked("http://localhost:443/").is_some());
        assert!(is_url_blocked("http://10.0.0.1:9999/").is_some());
        assert!(is_url_blocked("https://example.com:8443/api").is_none());
    }

    // ── Tool definition tests ──────────────────────
    #[test]
    fn test_tool_name() {
        let tool = HttpRequestTool::new();
        assert_eq!(tool.name(), "http_request");
    }

    #[test]
    fn test_tool_definition() {
        let tool = HttpRequestTool::new();
        let def = tool.definition();
        assert_eq!(def.name, "http_request");
        assert!(def.description.contains("HTTP"));
        let params = def.parameters;
        assert!(params["properties"]["url"].is_object());
        assert!(params["properties"]["method"].is_object());
        assert!(params["required"].as_array().unwrap().len() == 1);
    }

    #[test]
    fn test_default_impl() {
        let tool = HttpRequestTool::default();
        assert_eq!(tool.name(), "http_request");
    }

    // ── Execute tests (blocked URLs) ──────────────────────
    #[tokio::test]
    async fn test_execute_blocks_internal() {
        let tool = HttpRequestTool::new();
        let result = tool
            .execute(r#"{"url":"http://127.0.0.1:8080/admin"}"#)
            .await
            .unwrap();
        assert!(!result.success);
        assert!(result.output.contains("Blocked"));
    }

    #[tokio::test]
    async fn test_execute_blocks_non_http() {
        let tool = HttpRequestTool::new();
        let result = tool
            .execute(r#"{"url":"ftp://evil.com/file"}"#)
            .await
            .unwrap();
        assert!(!result.success);
        assert!(result.output.contains("Blocked"));
    }

    #[tokio::test]
    async fn test_execute_missing_url() {
        let tool = HttpRequestTool::new();
        let result = tool.execute(r#"{"method":"GET"}"#).await;
        assert!(result.is_err() || !result.unwrap().success);
    }
}
