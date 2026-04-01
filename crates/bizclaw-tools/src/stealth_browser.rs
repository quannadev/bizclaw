//! Native Rust Stealth Browser automation tool
//!
//! Provides AI agents with anti-detection browser control to bypass Cloudflare and bot checks.
//! Implemented purely in Rust using `headless_chrome` (no Node.js required).

use async_trait::async_trait;
use bizclaw_core::error::Result;
use bizclaw_core::traits::Tool;
use bizclaw_core::types::{ToolDefinition, ToolResult};
use headless_chrome::{Browser, LaunchOptionsBuilder, Tab};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Native Rust Stealth browser automation tool
pub struct StealthBrowserTool {
    browser: Arc<Mutex<Option<Browser>>>,
    tab: Arc<Mutex<Option<Arc<Tab>>>>,
}

impl StealthBrowserTool {
    pub fn new() -> Self {
        Self {
            browser: Arc::new(Mutex::new(None)),
            tab: Arc::new(Mutex::new(None)),
        }
    }

    async fn ensure_browser_and_tab(
        &self,
        headless: bool,
        profile: Option<&str>,
    ) -> Result<Arc<Tab>> {
        let mut b_guard = self.browser.lock().await;
        let mut t_guard = self.tab.lock().await;

        if let Some(tab) = t_guard.as_ref() {
            return Ok(Arc::clone(tab));
        }

        // Initialize Native Rust Headless browser with anti-detect flags
        tracing::info!(
            "🚀 Starting Native Stealth Browser (Profile: {:?})...",
            profile
        );

        let mut builder = LaunchOptionsBuilder::default();
        builder.headless(headless);

        if let Some(p) = profile {
            let user_data_dir = dirs::home_dir()
                .unwrap_or_else(|| std::path::PathBuf::from("."))
                .join(".bizclaw")
                .join("browser_profiles")
                .join(p);
            std::fs::create_dir_all(&user_data_dir).ok();
            builder.user_data_dir(Some(user_data_dir));
        }

        let launch_options = builder
            .args(vec![
                std::ffi::OsStr::new("--disable-blink-features=AutomationControlled"),
                std::ffi::OsStr::new("--no-sandbox"),
                std::ffi::OsStr::new("--disable-dev-shm-usage"),
                std::ffi::OsStr::new("--window-size=1920,1080"),
                std::ffi::OsStr::new("--user-agent=Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36"),
                std::ffi::OsStr::new("--disable-gpu"), // Usually helps in headless
            ])
            .build()
            .map_err(|e| bizclaw_core::error::BizClawError::Tool(format!("Failed to build launch options: {e}")))?;

        let new_browser = Browser::new(launch_options).map_err(|e| {
            bizclaw_core::error::BizClawError::Tool(format!("Failed starting headless chrome: {e}"))
        })?;

        let new_tab = new_browser.new_tab().map_err(|e| {
            bizclaw_core::error::BizClawError::Tool(format!("Failed opening tab: {e}"))
        })?;

        // We evaluate stealth script after navigating.

        *b_guard = Some(new_browser);
        *t_guard = Some(Arc::clone(&new_tab));

        Ok(new_tab)
    }

    /// Extract innerText directly
    fn extract_text(tab: &Tab) -> std::result::Result<String, anyhow::Error> {
        let text = tab.evaluate("document.body.innerText", false)?;
        let t = text
            .value
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_default();
        Ok(t)
    }

    /// Implement a quick DOM Snapshot for Interactive Elements (e5, e6 anchors)
    fn get_interactive_snapshot(tab: &Tab) -> std::result::Result<String, anyhow::Error> {
        let script = r#"
            (function() {
                let result = [];
                let refCount = 0;
                
                function isVisible(el) {
                    if (!el) return false;
                    const style = window.getComputedStyle(el);
                    return style.display !== 'none' && style.visibility !== 'hidden' && style.opacity !== '0';
                }

                function traverse(node) {
                    if (node.nodeType === Node.TEXT_NODE) {
                        const t = node.textContent.trim();
                        if (t && isVisible(node.parentElement)) {
                            result.push(t);
                        }
                    } else if (node.nodeType === Node.ELEMENT_NODE) {
                        if (!isVisible(node)) return;
                        const tag = node.tagName.toLowerCase();
                        if (['script', 'style', 'noscript', 'svg'].includes(tag)) return;
                        
                        let interactable = false;
                        if (['a', 'button', 'input', 'select', 'textarea'].includes(tag) || node.onclick || node.getAttribute('role') === 'button') {
                            interactable = true;
                        }
                        
                        if (interactable) {
                            refCount++;
                            const refId = 'e' + refCount;
                            node.setAttribute('data-pinch-id', refId); // mark it
                            
                            let label = node.innerText || node.value || node.placeholder || node.getAttribute('aria-label') || tag;
                            label = label.trim().substring(0, 50).replace(/\n/g, ' ');
                            result.push(`[${label}](${refId})`);
                        } else {
                            for (const child of node.childNodes) traverse(child);
                        }
                    }
                }
                traverse(document.body);
                return result.join(' | ');
            })();
        "#;
        let res = tab.evaluate(script, false)?;
        Ok(res
            .value
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_default())
    }
}

impl Default for StealthBrowserTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for StealthBrowserTool {
    fn name(&self) -> &str {
        "stealth_browser"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "stealth_browser".into(),
            description: concat!(
                "Use native Rust stealth browser automation to navigate pages bypassing Cloudflare, captchas, and bot detection. ",
                "Fully built-in, NO Node.js required. ",
                "Actions: navigate, snapshot, click, fill, text, press, evaluate, close."
            ).into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["navigate", "snapshot", "click", "fill", "text", "press", "evaluate", "upload", "close"],
                        "description": "Browser action to perform: navigate, snapshot, click, fill, text, press, evaluate, upload, close."
                    },
                    "url": {
                        "type": "string",
                        "description": "URL to navigate to"
                    },
                    "ref": {
                        "type": "string",
                        "description": "Element reference (e.g. 'e5')"
                    },
                    "value": {
                        "type": "string",
                        "description": "Text to type or JS code"
                    },
                    "headless": {
                        "type": "boolean",
                        "description": "Run headless (default: true)"
                    },
                    "profile": {
                        "type": "string",
                        "description": "Browser profile name to persist cookies/login state (e.g., 'facebook_bot')"
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

        if action == "close" {
            let mut b_guard = self.browser.lock().await;
            let mut t_guard = self.tab.lock().await;
            *t_guard = None;
            *b_guard = None;
            return Ok(ToolResult {
                tool_call_id: String::new(),
                output: "🔴 Native browser instance closed.".into(),
                success: true,
            });
        }

        let headless = args["headless"].as_bool().unwrap_or(true);
        let profile = args["profile"].as_str();

        let tab = self.ensure_browser_and_tab(headless, profile).await?;

        // Perform actions on the blocking thread to avoid locking tokio runtime
        let args_clone = args.clone();
        let action_str = action.to_string();

        let result =
            tokio::task::spawn_blocking(move || -> std::result::Result<String, anyhow::Error> {
                match action_str.as_str() {
                    "navigate" => {
                        let url = args_clone["url"]
                            .as_str()
                            .ok_or_else(|| anyhow::anyhow!("Missing url"))?;
                        tab.navigate_to(url)?;
                        tab.wait_until_navigated()?;

                        let stealth_script = r#"
                        Object.defineProperty(navigator, 'webdriver', { get: () => undefined });
                        window.chrome = { runtime: {} };
                        if (window.navigator.permissions) {
                            const originalQuery = window.navigator.permissions.query;
                            window.navigator.permissions.query = (parameters) => {
                                return parameters.name === 'notifications' ?
                                    Promise.resolve({ state: Notification.permission }) :
                                    originalQuery(parameters);
                            };
                        }
                    "#;
                        let _ = tab.evaluate(stealth_script, false);

                        Ok(format!("✅ Navigated to: {}", url))
                    }
                    "snapshot" => {
                        let snap = Self::get_interactive_snapshot(&tab)?;
                        Ok(format!("📸 Page snapshot:\n{}", snap))
                    }
                    "text" => {
                        let txt = Self::extract_text(&tab)?;
                        Ok(format!("📄 Page text:\n{}", txt))
                    }
                    "click" => {
                        let e_ref = args_clone["ref"]
                            .as_str()
                            .ok_or_else(|| anyhow::anyhow!("Missing ref"))?;
                        let selector = format!("[data-pinch-id=\"{}\"]", e_ref);
                        let element = tab.wait_for_element(&selector)?;
                        element.click()?;
                        Ok(format!("🖱️ Clicked {}", e_ref))
                    }
                    "fill" => {
                        let e_ref = args_clone["ref"]
                            .as_str()
                            .ok_or_else(|| anyhow::anyhow!("Missing ref"))?;
                        let val = args_clone["value"].as_str().unwrap_or_default();
                        let selector = format!("[data-pinch-id=\"{}\"]", e_ref);
                        let element = tab.wait_for_element(&selector)?;
                        element.click()?;
                        // Types directly
                        element.type_into(val)?;
                        Ok(format!("⌨️ Typed '{}' into {}", val, e_ref))
                    }
                    "upload" => {
                        let e_ref = args_clone["ref"]
                            .as_str()
                            .ok_or_else(|| anyhow::anyhow!("Missing ref"))?;
                        let path_val = args_clone["value"]
                            .as_str()
                            .ok_or_else(|| anyhow::anyhow!("Missing value (file path)"))?;
                        let selector = format!("[data-pinch-id=\"{}\"]", e_ref);
                        let element = tab.wait_for_element(&selector)?;

                        if !std::path::Path::new(path_val).exists() {
                            return Err(anyhow::anyhow!("File does not exist: {}", path_val));
                        }

                        element.set_input_files(&[path_val])?;
                        Ok(format!("📁 Uploaded file '{}' successfully.", path_val))
                    }
                    "evaluate" => {
                        let code = args_clone["value"].as_str().unwrap_or_default();
                        let res = tab.evaluate(code, false)?;
                        let txt = res
                            .value
                            .and_then(|v| v.as_str().map(|s| s.to_string()))
                            .unwrap_or_default();
                        Ok(format!("🔧 JS Evaluate Output:\n{}", txt))
                    }
                    _ => Ok(format!("Unsupported action in native mode: {}", action_str)),
                }
            })
            .await
            .map_err(|e| bizclaw_core::error::BizClawError::Tool(format!("Task panic: {e}")))?;

        match result {
            Ok(output) => Ok(ToolResult {
                tool_call_id: String::new(),
                output,
                success: true,
            }),
            Err(e) => Ok(ToolResult {
                tool_call_id: String::new(),
                output: format!("❌ Error: {}", e),
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
        let tool = StealthBrowserTool::new();
        assert_eq!(tool.name(), "stealth_browser");
    }

    #[test]
    fn test_tool_definition() {
        let tool = StealthBrowserTool::new();
        let def = tool.definition();
        assert_eq!(def.name, "stealth_browser");
        assert!(def.description.contains("native Rust stealth browser"));
        let params = def.parameters;
        assert!(params["properties"]["action"].is_object());
        assert!(params["properties"]["url"].is_object());
    }

    #[tokio::test]
    async fn test_missing_action() {
        let tool = StealthBrowserTool::new();
        let result = tool.execute(r#"{"url":"https://example.com"}"#).await;
        assert!(result.is_err());
    }
}
