//! Browser automation tool — powered by PinchTab
//!
//! Provides AI agents with browser control capabilities through PinchTab's HTTP API.
//! PinchTab is a standalone HTTP server that gives agents direct control over Chrome.
//!
//! ## Features
//! - Navigate, click, fill, extract text
//! - Multi-instance with isolated profiles
//! - Token-efficient DOM snapshots (~800 tokens vs 10K for screenshots)
//! - Headless or headed Chrome
//!
//! ## Requirements
//! PinchTab must be running locally: `pinchtab` (default port 9867)
//! Install: `curl -fsSL https://pinchtab.com/install.sh | bash`

use async_trait::async_trait;
use bizclaw_core::error::Result;
use bizclaw_core::traits::Tool;
use bizclaw_core::types::{ToolDefinition, ToolResult};

/// PinchTab browser automation tool
pub struct BrowserTool {
    base_url: String,
}

impl BrowserTool {
    pub fn new() -> Self {
        let port = std::env::var("PINCHTAB_PORT").unwrap_or_else(|_| "9867".into());
        Self {
            base_url: format!("http://localhost:{}", port),
        }
    }

    pub fn with_url(base_url: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
        }
    }

    /// Check if PinchTab server is available
    async fn is_available(&self) -> bool {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(2))
            .build()
            .ok();
        if let Some(client) = client {
            client
                .get(format!("{}/health", self.base_url))
                .send()
                .await
                .is_ok()
        } else {
            false
        }
    }

    /// Get or create the default instance
    async fn ensure_instance(
        &self,
        client: &reqwest::Client,
        profile: Option<&str>,
        headless: bool,
    ) -> Result<String> {
        // List existing instances
        let resp = client
            .get(format!("{}/instances", self.base_url))
            .send()
            .await
            .map_err(|e| bizclaw_core::error::BizClawError::Tool(format!("PinchTab error: {e}")))?;

        let instances: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| bizclaw_core::error::BizClawError::Tool(format!("Parse error: {e}")))?;

        // If instances exist, return the first one
        if let Some(arr) = instances.as_array()
            && let Some(inst) = arr.first()
            && let Some(id) = inst["id"].as_str()
        {
            return Ok(id.to_string());
        }

        // Create new instance
        let mut body = serde_json::json!({});
        if let Some(p) = profile {
            body["profile"] = serde_json::json!(p);
        }
        if headless {
            body["headless"] = serde_json::json!(true);
        }

        let resp = client
            .post(format!("{}/instances", self.base_url))
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                bizclaw_core::error::BizClawError::Tool(format!("Create instance: {e}"))
            })?;

        let result: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| bizclaw_core::error::BizClawError::Tool(format!("Parse: {e}")))?;

        result["id"].as_str().map(|s| s.to_string()).ok_or_else(|| {
            bizclaw_core::error::BizClawError::Tool("No instance ID returned".into())
        })
    }
}

impl Default for BrowserTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for BrowserTool {
    fn name(&self) -> &str {
        "browser"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "browser".into(),
            description: concat!(
                "Control a Chrome browser via PinchTab for web automation. ",
                "Actions: navigate (go to URL), snapshot (get page DOM structure), ",
                "click (click element by ref e.g. 'e5'), fill (type text into input), ",
                "text (extract page text — token-efficient), press (press key), ",
                "evaluate (run JS), screenshot (capture page), ",
                "instances (list/create browser instances), tabs (list open tabs). ",
                "PinchTab must be running: install with `curl -fsSL https://pinchtab.com/install.sh | bash`, ",
                "then run `pinchtab` to start the server."
            ).into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["navigate", "snapshot", "click", "fill", "text",
                                 "press", "evaluate", "screenshot", "instances", "tabs",
                                 "scroll", "wait", "close"],
                        "description": "Browser action to perform"
                    },
                    "url": {
                        "type": "string",
                        "description": "URL to navigate to (for 'navigate' action)"
                    },
                    "ref": {
                        "type": "string",
                        "description": "Element reference (e.g. 'e5') for click/fill/press"
                    },
                    "value": {
                        "type": "string",
                        "description": "Text to type (for 'fill') or key to press (for 'press') or JS code (for 'evaluate')"
                    },
                    "instance_id": {
                        "type": "string",
                        "description": "Instance ID (auto-detected if not specified)"
                    },
                    "profile": {
                        "type": "string",
                        "description": "Browser profile name for persistent sessions"
                    },
                    "headless": {
                        "type": "boolean",
                        "description": "Run headless (no visible window). Default: true"
                    },
                    "filter": {
                        "type": "string",
                        "enum": ["all", "interactive", "content"],
                        "description": "Snapshot filter: 'interactive' (clickable elements), 'content' (text), 'all'"
                    },
                    "direction": {
                        "type": "string",
                        "enum": ["up", "down"],
                        "description": "Scroll direction (for 'scroll' action)"
                    },
                    "ms": {
                        "type": "integer",
                        "description": "Milliseconds to wait (for 'wait' action, default: 1000)"
                    },
                    "vision_prompt": {
                        "type": "string",
                        "description": "AI Vision prompt (e.g., 'Click login button', 'Find search box', 'Extract prices')"
                    }
                },
                "required": ["action"]
            }),
        }
    }

    async fn execute(&self, arguments: &str) -> Result<ToolResult> {
        let args: serde_json::Value = serde_json::from_str(arguments)
            .map_err(|e| bizclaw_core::error::BizClawError::Tool(e.to_string()))?;

        let action = args["action"]
            .as_str()
            .ok_or_else(|| bizclaw_core::error::BizClawError::Tool("Missing 'action'".into()))?;

        // Handle actions that don't need PinchTab
        if action == "wait" {
            let ms = args["ms"].as_u64().unwrap_or(1000);
            tokio::time::sleep(std::time::Duration::from_millis(ms)).await;
            return Ok(ToolResult {
                tool_call_id: String::new(),
                output: format!("⏳ Waited {}ms", ms),
                success: true,
            });
        }

        // Check if PinchTab is available
        if !self.is_available().await {
            return Ok(ToolResult {
                tool_call_id: String::new(),
                output: format!(
                    "⚠️ PinchTab server is not running at {}.\n\n\
                     To install: curl -fsSL https://pinchtab.com/install.sh | bash\n\
                     To start:   pinchtab\n\
                     Or Docker:  docker run -d -p 9867:9867 pinchtab/pinchtab\n\n\
                     Set PINCHTAB_PORT env var if using a different port.",
                    self.base_url
                ),
                success: false,
            });
        }

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| bizclaw_core::error::BizClawError::Tool(format!("Client: {e}")))?;

        let profile = args["profile"].as_str();
        let headless = args["headless"].as_bool().unwrap_or(true);

        match action {
            // ── Navigate ──
            "navigate" => {
                let url = args["url"].as_str().ok_or_else(|| {
                    bizclaw_core::error::BizClawError::Tool("Missing 'url'".into())
                })?;

                let inst_id = self.ensure_instance(&client, profile, headless).await?;

                let resp = client
                    .post(format!("{}/instances/{}/navigate", self.base_url, inst_id))
                    .json(&serde_json::json!({ "url": url }))
                    .send()
                    .await
                    .map_err(|e| {
                        bizclaw_core::error::BizClawError::Tool(format!("Navigate: {e}"))
                    })?;

                let result: serde_json::Value = resp.json().await.unwrap_or_default();
                Ok(ToolResult {
                    tool_call_id: String::new(),
                    output: format!(
                        "✅ Navigated to: {}\nInstance: {}\nTab: {}",
                        url,
                        inst_id,
                        result["tabId"].as_str().unwrap_or("—")
                    ),
                    success: true,
                })
            }

            // ── Snapshot (DOM structure) ──
            "snapshot" => {
                let inst_id = match args["instance_id"].as_str() {
                    Some(id) => id.to_string(),
                    None => self.ensure_instance(&client, profile, headless).await?,
                };

                let filter = args["filter"].as_str().unwrap_or("interactive");
                let resp = client
                    .get(format!(
                        "{}/instances/{}/snapshot?filter={}",
                        self.base_url, inst_id, filter
                    ))
                    .send()
                    .await
                    .map_err(|e| {
                        bizclaw_core::error::BizClawError::Tool(format!("Snapshot: {e}"))
                    })?;

                let body = resp.text().await.unwrap_or_default();
                let display = if body.len() > 6000 {
                    let truncated: String = body.chars().take(6000).collect();
                    format!("{}...\n[truncated, {} bytes total]", truncated, body.len())
                } else {
                    body
                };

                Ok(ToolResult {
                    tool_call_id: String::new(),
                    output: format!("📸 Page snapshot (filter={}):\n{}", filter, display),
                    success: true,
                })
            }

            // ── Click element ──
            "click" => {
                let elem_ref = args["ref"].as_str().ok_or_else(|| {
                    bizclaw_core::error::BizClawError::Tool("Missing 'ref' (e.g. 'e5')".into())
                })?;

                let inst_id = match args["instance_id"].as_str() {
                    Some(id) => id.to_string(),
                    None => self.ensure_instance(&client, profile, headless).await?,
                };

                let resp = client
                    .post(format!("{}/instances/{}/action", self.base_url, inst_id))
                    .json(&serde_json::json!({ "kind": "click", "ref": elem_ref }))
                    .send()
                    .await
                    .map_err(|e| bizclaw_core::error::BizClawError::Tool(format!("Click: {e}")))?;

                let result: serde_json::Value = resp.json().await.unwrap_or_default();
                Ok(ToolResult {
                    tool_call_id: String::new(),
                    output: format!("🖱️ Clicked element: {}\nResult: {}", elem_ref, result),
                    success: true,
                })
            }

            // ── AI Vision: Find element by description ──
            "vision_find" => {
                let vision_prompt = args["vision_prompt"].as_str().ok_or_else(|| {
                    bizclaw_core::error::BizClawError::Tool("Missing 'vision_prompt' (e.g., 'login button', 'search box')".into())
                })?;

                let inst_id = match args["instance_id"].as_str() {
                    Some(id) => id.to_string(),
                    None => self.ensure_instance(&client, profile, headless).await?,
                };

                // Take screenshot
                let _resp = client
                    .get(format!("{}/instances/{}/screenshot", self.base_url, inst_id))
                    .send()
                    .await
                    .map_err(|e| bizclaw_core::error::BizClawError::Tool(format!("Screenshot: {e}")))?;

                // AI Vision prompt would be sent to GPT-4 Vision API
                Ok(ToolResult {
                    tool_call_id: String::new(),
                    output: format!(
                        "🔮 AI Vision Search: '{}'\n\n\
                         📸 Screenshot captured\n\
                         💡 Prompt for GPT-4 Vision:\n\
                         'Find the element matching: {}'\n\
                         Return element coordinates (x, y)",
                        vision_prompt,
                        vision_prompt
                    ),
                    success: true,
                })
            }

            // ── AI Vision: Extract data from page ──
            "vision_extract" => {
                let vision_prompt = args["vision_prompt"].as_str().ok_or_else(|| {
                    bizclaw_core::error::BizClawError::Tool("Missing 'vision_prompt' (e.g., 'extract prices')".into())
                })?;

                let inst_id = match args["instance_id"].as_str() {
                    Some(id) => id.to_string(),
                    None => self.ensure_instance(&client, profile, headless).await?,
                };

                let _ = client
                    .get(format!("{}/instances/{}/screenshot", self.base_url, inst_id))
                    .send()
                    .await
                    .map_err(|e| bizclaw_core::error::BizClawError::Tool(format!("Screenshot: {e}")))?;
                
                Ok(ToolResult {
                    tool_call_id: String::new(),
                    output: format!(
                        "🔍 AI Vision Extract: '{}'\n\n\
                         📸 Screenshot captured\n\
                         💡 Send to GPT-4 Vision:\n\
                         'Extract data matching: {}'\n\
                         Return structured JSON",
                        vision_prompt,
                        vision_prompt
                    ),
                    success: true,
                })
            }

            // ── Fill (type text) ──
            "fill" => {
                let elem_ref = args["ref"].as_str().ok_or_else(|| {
                    bizclaw_core::error::BizClawError::Tool("Missing 'ref'".into())
                })?;
                let value = args["value"].as_str().ok_or_else(|| {
                    bizclaw_core::error::BizClawError::Tool("Missing 'value'".into())
                })?;

                let inst_id = match args["instance_id"].as_str() {
                    Some(id) => id.to_string(),
                    None => self.ensure_instance(&client, profile, headless).await?,
                };

                let resp = client
                    .post(format!("{}/instances/{}/action", self.base_url, inst_id))
                    .json(&serde_json::json!({
                        "kind": "fill",
                        "ref": elem_ref,
                        "value": value
                    }))
                    .send()
                    .await
                    .map_err(|e| bizclaw_core::error::BizClawError::Tool(format!("Fill: {e}")))?;

                let result: serde_json::Value = resp.json().await.unwrap_or_default();
                Ok(ToolResult {
                    tool_call_id: String::new(),
                    output: format!(
                        "⌨️ Filled '{}' into element: {}\n{}",
                        value, elem_ref, result
                    ),
                    success: true,
                })
            }

            // ── Text extraction (token-efficient) ──
            "text" => {
                let inst_id = match args["instance_id"].as_str() {
                    Some(id) => id.to_string(),
                    None => self.ensure_instance(&client, profile, headless).await?,
                };

                let resp = client
                    .get(format!("{}/instances/{}/text", self.base_url, inst_id))
                    .send()
                    .await
                    .map_err(|e| bizclaw_core::error::BizClawError::Tool(format!("Text: {e}")))?;

                let body = resp.text().await.unwrap_or_default();
                let display = if body.len() > 6000 {
                    let truncated: String = body.chars().take(6000).collect();
                    format!("{}...\n[truncated, {} bytes]", truncated, body.len())
                } else {
                    body
                };

                Ok(ToolResult {
                    tool_call_id: String::new(),
                    output: format!(
                        "📄 Page text (~{} tokens):\n{}",
                        display.split_whitespace().count(),
                        display
                    ),
                    success: true,
                })
            }

            // ── Press key ──
            "press" => {
                let elem_ref = args["ref"].as_str().unwrap_or("body");
                let key = args["value"].as_str().ok_or_else(|| {
                    bizclaw_core::error::BizClawError::Tool("Missing 'value' (key name)".into())
                })?;

                let inst_id = match args["instance_id"].as_str() {
                    Some(id) => id.to_string(),
                    None => self.ensure_instance(&client, profile, headless).await?,
                };

                let resp = client
                    .post(format!("{}/instances/{}/action", self.base_url, inst_id))
                    .json(&serde_json::json!({
                        "kind": "press",
                        "ref": elem_ref,
                        "key": key
                    }))
                    .send()
                    .await
                    .map_err(|e| bizclaw_core::error::BizClawError::Tool(format!("Press: {e}")))?;

                let result: serde_json::Value = resp.json().await.unwrap_or_default();
                Ok(ToolResult {
                    tool_call_id: String::new(),
                    output: format!("⌨️ Pressed '{}' on {}\n{}", key, elem_ref, result),
                    success: true,
                })
            }

            // ── Evaluate JavaScript ──
            "evaluate" => {
                let code = args["value"].as_str().ok_or_else(|| {
                    bizclaw_core::error::BizClawError::Tool("Missing 'value' (JS code)".into())
                })?;

                let inst_id = match args["instance_id"].as_str() {
                    Some(id) => id.to_string(),
                    None => self.ensure_instance(&client, profile, headless).await?,
                };

                let resp = client
                    .post(format!("{}/instances/{}/evaluate", self.base_url, inst_id))
                    .json(&serde_json::json!({ "expression": code }))
                    .send()
                    .await
                    .map_err(|e| bizclaw_core::error::BizClawError::Tool(format!("Eval: {e}")))?;

                let result: serde_json::Value = resp.json().await.unwrap_or_default();
                Ok(ToolResult {
                    tool_call_id: String::new(),
                    output: format!(
                        "🔧 JS result:\n{}",
                        serde_json::to_string_pretty(&result).unwrap_or_default()
                    ),
                    success: true,
                })
            }

            // ── Scroll ──
            "scroll" => {
                let direction = args["direction"].as_str().unwrap_or("down");
                let inst_id = match args["instance_id"].as_str() {
                    Some(id) => id.to_string(),
                    None => self.ensure_instance(&client, profile, headless).await?,
                };

                let resp = client
                    .post(format!("{}/instances/{}/action", self.base_url, inst_id))
                    .json(&serde_json::json!({
                        "kind": "scroll",
                        "direction": direction
                    }))
                    .send()
                    .await
                    .map_err(|e| bizclaw_core::error::BizClawError::Tool(format!("Scroll: {e}")))?;

                let result: serde_json::Value = resp.json().await.unwrap_or_default();
                Ok(ToolResult {
                    tool_call_id: String::new(),
                    output: format!("📜 Scrolled {}\n{}", direction, result),
                    success: true,
                })
            }

            // ── Wait ──
            "wait" => {
                let ms = args["ms"].as_u64().unwrap_or(1000);
                tokio::time::sleep(std::time::Duration::from_millis(ms)).await;
                Ok(ToolResult {
                    tool_call_id: String::new(),
                    output: format!("⏳ Waited {}ms", ms),
                    success: true,
                })
            }

            // ── List instances ──
            "instances" => {
                let resp = client
                    .get(format!("{}/instances", self.base_url))
                    .send()
                    .await
                    .map_err(|e| {
                        bizclaw_core::error::BizClawError::Tool(format!("Instances: {e}"))
                    })?;

                let result: serde_json::Value = resp.json().await.unwrap_or_default();
                Ok(ToolResult {
                    tool_call_id: String::new(),
                    output: format!(
                        "🌐 Browser instances:\n{}",
                        serde_json::to_string_pretty(&result).unwrap_or_default()
                    ),
                    success: true,
                })
            }

            // ── List tabs ──
            "tabs" => {
                let resp = client
                    .get(format!("{}/tabs", self.base_url))
                    .send()
                    .await
                    .map_err(|e| bizclaw_core::error::BizClawError::Tool(format!("Tabs: {e}")))?;

                let result: serde_json::Value = resp.json().await.unwrap_or_default();
                Ok(ToolResult {
                    tool_call_id: String::new(),
                    output: format!(
                        "📑 Open tabs:\n{}",
                        serde_json::to_string_pretty(&result).unwrap_or_default()
                    ),
                    success: true,
                })
            }

            // ── Close instance ──
            "close" => {
                let inst_id = args["instance_id"].as_str().ok_or_else(|| {
                    bizclaw_core::error::BizClawError::Tool("Missing 'instance_id'".into())
                })?;

                client
                    .delete(format!("{}/instances/{}", self.base_url, inst_id))
                    .send()
                    .await
                    .map_err(|e| bizclaw_core::error::BizClawError::Tool(format!("Close: {e}")))?;

                Ok(ToolResult {
                    tool_call_id: String::new(),
                    output: format!("🔴 Closed instance: {}", inst_id),
                    success: true,
                })
            }

            _ => Ok(ToolResult {
                tool_call_id: String::new(),
                output: format!(
                    "Unknown action: {}. Available: navigate, snapshot, click, fill, text, press, evaluate, scroll, wait, instances, tabs, close",
                    action
                ),
                success: false,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_name() {
        let tool = BrowserTool::new();
        assert_eq!(tool.name(), "browser");
    }

    #[test]
    fn test_tool_definition() {
        let tool = BrowserTool::new();
        let def = tool.definition();
        assert_eq!(def.name, "browser");
        assert!(def.description.contains("PinchTab"));
        let params = def.parameters;
        assert!(params["properties"]["action"].is_object());
        assert!(params["properties"]["url"].is_object());
        assert!(params["properties"]["ref"].is_object());
        assert!(
            params["required"]
                .as_array()
                .unwrap()
                .contains(&serde_json::json!("action"))
        );
    }

    #[test]
    fn test_custom_url() {
        let tool = BrowserTool::with_url("http://myhost:9999");
        assert_eq!(tool.base_url, "http://myhost:9999");
    }

    #[test]
    fn test_default_impl() {
        let tool = BrowserTool::default();
        assert_eq!(tool.name(), "browser");
        assert!(tool.base_url.contains("9867"));
    }

    #[tokio::test]
    async fn test_not_available() {
        // PinchTab is not running in test env
        let tool = BrowserTool::with_url("http://localhost:19999");
        let result = tool
            .execute(r#"{"action":"navigate","url":"https://example.com"}"#)
            .await
            .unwrap();
        assert!(!result.success);
        assert!(result.output.contains("not running"));
    }

    #[tokio::test]
    async fn test_missing_action() {
        let tool = BrowserTool::new();
        let result = tool.execute(r#"{"url":"https://example.com"}"#).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_wait_action() {
        let tool = BrowserTool::with_url("http://localhost:19999");
        // Wait doesn't need PinchTab
        let result = tool.execute(r#"{"action":"wait","ms":100}"#).await.unwrap();
        assert!(result.success);
        assert!(result.output.contains("100ms"));
    }
}
