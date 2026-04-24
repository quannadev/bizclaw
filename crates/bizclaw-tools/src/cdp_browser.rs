//! CDP Browser tool — direct Chrome DevTools Protocol automation
//!
//! Provides AI agents with browser control using native CDP WebSocket connection.
//! No external dependencies needed - connects directly to Chrome's DevTools.
//!
//! ## Features
//! - Direct CDP WebSocket connection
//! - Native Chrome control without external servers
//! - Supports all browser actions: navigate, click, type, extract
//! - Skills system for domain-specific patterns
//!
//! ## Requirements
//! Chrome must be running with remote debugging enabled:
//! `google-chrome --remote-debugging-port=9222`

use async_trait::async_trait;
use bizclaw_core::error::Result;
use bizclaw_core::traits::Tool;
use bizclaw_core::types::{ToolDefinition, ToolResult};
use bizclaw_browser::{BrowserSession, SessionConfig};

pub struct CdpBrowserTool {
    config: SessionConfig,
}

impl CdpBrowserTool {
    pub fn new() -> Self {
        Self {
            config: SessionConfig::default(),
        }
    }

    pub fn with_port(port: u16) -> Self {
        let mut config = SessionConfig::default();
        config.chrome_debug_port = port;
        Self { config }
    }

    pub fn with_viewport(width: u32, height: u32) -> Self {
        let config = SessionConfig {
            chrome_debug_port: 9222,
            page_id: None,
            enable_screenshots: true,
            viewport: Some(bizclaw_browser::ViewportConfig {
                width,
                height,
                device_scale_factor: Some(1.0),
            }),
            user_agent: None,
            headless: false,
        };
        Self { config }
    }
}

impl Default for CdpBrowserTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for CdpBrowserTool {
    fn name(&self) -> &str {
        "cdp_browser"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "cdp_browser".into(),
            description: concat!(
                "Control Chrome browser via CDP (Chrome DevTools Protocol). ",
                "Direct WebSocket connection - no external server needed. ",
                "Actions:\n",
                "  - navigate(url): Go to URL\n",
                "  - click(selector): Click element by CSS selector\n",
                "  - type_text(selector, text): Type into input field\n",
                "  - get_text(selector): Extract text from element\n",
                "  - extract_content(selector): Get element HTML and links\n",
                "  - screenshot(): Capture page screenshot (base64)\n",
                "  - press_key(key): Press keyboard key\n",
                "  - scroll(x, y): Scroll to coordinates\n",
                "  - wait_for(selector, ms): Wait for element\n",
                "  - get_page_info(): Get current URL and title\n",
                "\nRequirements: Chrome with --remote-debugging-port=9222"
            ).into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["navigate", "click", "type_text", "get_text", 
                                 "extract_content", "extract_all", "screenshot",
                                 "press_key", "scroll", "hover", "wait_for",
                                 "get_page_info", "go_back", "go_forward", "reload"],
                        "description": "Browser action to perform"
                    },
                    "url": {
                        "type": "string",
                        "description": "URL to navigate to"
                    },
                    "selector": {
                        "type": "string",
                        "description": "CSS selector for element"
                    },
                    "text": {
                        "type": "string",
                        "description": "Text to type"
                    },
                    "key": {
                        "type": "string",
                        "description": "Key to press (e.g., 'Enter', 'Escape')"
                    },
                    "x": {
                        "type": "integer",
                        "description": "X coordinate for scroll"
                    },
                    "y": {
                        "type": "integer",
                        "description": "Y coordinate for scroll"
                    },
                    "tag": {
                        "type": "string",
                        "description": "HTML tag to extract all (e.g., 'a', 'img')"
                    },
                    "timeout": {
                        "type": "integer",
                        "description": "Timeout in ms for wait_for (default: 10000)"
                    },
                    "direction": {
                        "type": "string",
                        "enum": ["up", "down"],
                        "description": "Scroll direction"
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

        let session = match BrowserSession::create(self.config.clone()).await {
            Ok(s) => s,
            Err(e) => return Ok(ToolResult {
                tool_call_id: String::new(),
                output: format!(
                    "⚠️ Failed to connect to Chrome at port {}.\n\n\
                     Make sure Chrome is running with remote debugging:\n\n\
                     Mac:\n\
                     /Applications/Google\\ Chrome.app/Contents/MacOS/Google\\ Chrome \
                     --remote-debugging-port=9222\n\n\
                     Error: {}",
                    self.config.chrome_debug_port,
                    e
                ),
                success: false,
            }),
        };

        let result = match action {
            "navigate" => {
                let url = args["url"].as_str().ok_or_else(|| {
                    bizclaw_core::error::BizClawError::Tool("Missing 'url'".into())
                })?;
                session.navigate(url).await
            }
            "click" => {
                let selector = args["selector"].as_str().ok_or_else(|| {
                    bizclaw_core::error::BizClawError::Tool("Missing 'selector'".into())
                })?;
                session.tools.click(selector).await
            }
            "type_text" => {
                let selector = args["selector"].as_str().ok_or_else(|| {
                    bizclaw_core::error::BizClawError::Tool("Missing 'selector'".into())
                })?;
                let text = args["text"].as_str().ok_or_else(|| {
                    bizclaw_core::error::BizClawError::Tool("Missing 'text'".into())
                })?;
                session.tools.type_text(selector, text).await
            }
            "get_text" => {
                let selector = args["selector"].as_str().ok_or_else(|| {
                    bizclaw_core::error::BizClawError::Tool("Missing 'selector'".into())
                })?;
                session.tools.get_text(selector).await
            }
            "extract_content" => {
                let selector = args["selector"].as_str().unwrap_or("body");
                session.tools.extract_content(selector).await
            }
            "extract_all" => {
                let tag = args["tag"].as_str().unwrap_or("a");
                session.tools.extract_all(tag).await
            }
            "screenshot" => {
                session.tools.screenshot().await.map(|ss| {
                    bizclaw_browser::BrowserToolResult::ok(serde_json::json!({
                        "screenshot": format!("data:image/png;base64,{}", ss)
                    }))
                })
            }
            "press_key" => {
                let key = args["key"].as_str().ok_or_else(|| {
                    bizclaw_core::error::BizClawError::Tool("Missing 'key'".into())
                })?;
                session.tools.press_key(key).await
            }
            "scroll" => {
                let x = args["x"].as_i64().unwrap_or(0) as i32;
                let y = args["y"].as_i64().unwrap_or(0) as i32;
                session.tools.scroll(x, y).await
            }
            "hover" => {
                let selector = args["selector"].as_str().ok_or_else(|| {
                    bizclaw_core::error::BizClawError::Tool("Missing 'selector'".into())
                })?;
                session.tools.hover(selector).await
            }
            "wait_for" => {
                let selector = args["selector"].as_str().ok_or_else(|| {
                    bizclaw_core::error::BizClawError::Tool("Missing 'selector'".into())
                })?;
                let timeout = args["timeout"].as_u64().unwrap_or(10000);
                session.tools.wait_for_selector(selector, timeout).await
            }
            "get_page_info" => {
                session.tools.get_page_info().await
            }
            "go_back" => {
                session.tools.press_key("Alt+Left").await
            }
            "go_forward" => {
                session.tools.press_key("Alt+Right").await
            }
            "reload" => {
                session.tools.press_key("F5").await
            }
            _ => {
                return Ok(ToolResult {
                    tool_call_id: String::new(),
                    output: format!("Unknown action: {}. Available: navigate, click, type_text, get_text, extract_content, extract_all, screenshot, press_key, scroll, hover, wait_for, get_page_info, go_back, go_forward, reload", action),
                    success: false,
                });
            }
        };

        match result {
            Ok(r) => {
                if r.success {
                    let mut output = serde_json::to_string_pretty(&r.data)
                        .unwrap_or_else(|_| "{}".to_string());
                    
                    if let Some(ref ss) = r.screenshot {
                        output.push_str(&format!("\n\n📸 Screenshot: data:image/png;base64,{}", ss));
                    }
                    
                    Ok(ToolResult {
                        tool_call_id: String::new(),
                        output,
                        success: true,
                    })
                } else {
                    Ok(ToolResult {
                        tool_call_id: String::new(),
                        output: r.error.unwrap_or_else(|| "Unknown error".into()),
                        success: false,
                    })
                }
            }
            Err(e) => Ok(ToolResult {
                tool_call_id: String::new(),
                output: format!("❌ Error: {}", e),
                success: false,
            }),
        }
    }
}
