//! Gemini Web Provider — template capture via CDP + Batchexecute replay.
//!
//! How it works:
//! 1. Enable CDP Fetch interception for `*StreamGenerate*` URLs
//! 2. Navigate to gemini.google.com/app
//! 3. Type "Hello" and click send (triggers internal API)
//! 4. CDP intercepts the request → capture `f.req` template + `at` CSRF token
//! 5. For subsequent messages, replay the template with new prompt text
//!
//! Response format:
//! - Multiline JSON arrays
//! - Find items where `item[0] === "wrb.fr"`
//! - Parse `item[2]` as JSON → text at `data[4][0][1]`

use async_trait::async_trait;
use serde_json::{Value, json};
use std::sync::Mutex;
use tracing;

use crate::cdp::CdpClient;
use crate::cookie_auth;
use crate::types::{AuthCheckResult, WebAuthModel};

use super::WebProvider;

const LOG_TAG: &str = "[Gemini-Web]";

/// Captured Gemini API template.
#[derive(Debug, Clone)]
struct GeminiTemplate {
    /// The inner JSON array from f.req
    inner: Vec<Value>,
    /// CSRF token
    at_token: String,
    /// Full API URL (with account prefix /u/N/)
    url: String,
    /// Account prefix detected (e.g., "/u/0/")
    account_prefix: String,
}

pub struct GeminiWebProvider {
    models: Vec<WebAuthModel>,
    cached_template: Mutex<Option<GeminiTemplate>>,
}

impl GeminiWebProvider {
    pub fn new() -> Self {
        Self {
            models: vec![
                WebAuthModel {
                    id: "webauth-gemini-pro".to_string(),
                    name: "Gemini Pro (WebAuth)".to_string(),
                    context_window: 1048576,
                },
                WebAuthModel {
                    id: "webauth-gemini-flash".to_string(),
                    name: "Gemini Flash (WebAuth)".to_string(),
                    context_window: 1048576,
                },
            ],
            cached_template: Mutex::new(None),
        }
    }

    /// Capture the Batchexecute template via CDP Fetch interception.
    async fn capture_template(&self, cdp: &CdpClient) -> Result<GeminiTemplate, String> {
        tracing::info!("{} Capturing API template...", LOG_TAG);

        // 1. Enable Fetch interception
        cdp.enable_fetch("*StreamGenerate*").await?;

        // 2. Subscribe for the paused request
        let mut rx = cdp.subscribe("Fetch.requestPaused").await;

        // 3. Navigate and trigger a message
        cdp.evaluate_js(
            r#"
            (async () => {
                // Navigate to app
                if (!window.location.href.includes('gemini.google.com/app')) {
                    window.location.href = 'https://gemini.google.com/app';
                    await new Promise(r => setTimeout(r, 3000));
                }

                // Wait for input
                await new Promise(resolve => {
                    const check = () => {
                        const input = document.querySelector('div[contenteditable="true"], textarea, rich-textarea');
                        if (input) resolve(input);
                        else setTimeout(check, 500);
                    };
                    check();
                });

                // Type "Hello" and send
                const input = document.querySelector('div[contenteditable="true"], textarea, rich-textarea');
                if (input.tagName === 'TEXTAREA') {
                    input.value = 'Hello';
                    input.dispatchEvent(new Event('input', { bubbles: true }));
                } else {
                    input.textContent = 'Hello';
                    input.dispatchEvent(new Event('input', { bubbles: true }));
                }

                await new Promise(r => setTimeout(r, 500));

                const sendBtn = document.querySelector('button[aria-label="Send message"], button.send-button, button[data-tooltip="Send"]');
                if (sendBtn && !sendBtn.disabled) {
                    sendBtn.click();
                }
            })()
            "#,
        )
        .await?;

        // 4. Wait for Fetch.requestPaused event (15s timeout)
        let paused = tokio::time::timeout(
            std::time::Duration::from_secs(15),
            rx.recv(),
        )
        .await
        .map_err(|_| format!("{} Template capture timed out", LOG_TAG))?
        .ok_or_else(|| format!("{} Event channel closed", LOG_TAG))?;

        // 5. Extract template from postData
        let request_id = paused
            .get("requestId")
            .and_then(|v| v.as_str())
            .ok_or_else(|| format!("{} No requestId in paused event", LOG_TAG))?
            .to_string();

        let request_url = paused
            .get("request")
            .and_then(|r| r.get("url"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| format!("{} No URL in paused request", LOG_TAG))?
            .to_string();

        let post_data = paused
            .get("request")
            .and_then(|r| r.get("postData"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| format!("{} No postData in paused request", LOG_TAG))?
            .to_string();

        // 6. Parse the template
        let decoded = urlencoding::decode(&post_data)
            .map_err(|e| format!("{} URL decode failed: {}", LOG_TAG, e))?
            .to_string();

        // Extract f.req=...&at=...
        let freq_re = regex::Regex::new(r"f\.req=([\s\S]+?)&at=")
            .map_err(|e| format!("{} Regex error: {}", LOG_TAG, e))?;
        let at_re = regex::Regex::new(r"&at=([^&]+)")
            .map_err(|e| format!("{} Regex error: {}", LOG_TAG, e))?;

        let freq_match = freq_re
            .captures(&decoded)
            .ok_or_else(|| format!("{} No f.req found in postData", LOG_TAG))?;
        let at_match = at_re
            .captures(&decoded)
            .ok_or_else(|| format!("{} No at token found", LOG_TAG))?;

        let freq_str = &freq_match[1];
        let at_token = at_match[1].to_string();

        // Parse outer array, then inner
        let outer: Vec<Value> = serde_json::from_str(freq_str)
            .map_err(|e| format!("{} Outer JSON parse failed: {}", LOG_TAG, e))?;

        let inner_str = outer
            .get(1)
            .and_then(|v| v.as_str())
            .ok_or_else(|| format!("{} No inner string at outer[1]", LOG_TAG))?;

        let inner: Vec<Value> = serde_json::from_str(inner_str)
            .map_err(|e| format!("{} Inner JSON parse failed: {}", LOG_TAG, e))?;

        // 7. Continue the paused request
        cdp.continue_request(&request_id).await?;
        cdp.disable_fetch().await?;

        // Detect account prefix
        let account_prefix = if let Some(idx) = request_url.find("/u/") {
            let end = request_url[idx + 3..]
                .find('/')
                .map(|i| idx + 3 + i + 1)
                .unwrap_or(idx + 5);
            request_url[idx..end].to_string()
        } else {
            String::new()
        };

        tracing::info!(
            "{} Template captured! URL: ...{}, at_token len: {}, inner items: {}",
            LOG_TAG,
            if account_prefix.is_empty() {
                "/app"
            } else {
                &account_prefix
            },
            at_token.len(),
            inner.len()
        );

        Ok(GeminiTemplate {
            inner,
            at_token,
            url: request_url,
            account_prefix,
        })
    }

    /// Send a message using the captured template.
    async fn send_with_template(
        &self,
        cdp: &CdpClient,
        template: &GeminiTemplate,
        prompt: &str,
    ) -> Result<String, String> {
        // Clone template and inject new message
        let mut inner = template.inner.clone();

        // Set message text at inner[0][0]
        if let Some(first) = inner.get_mut(0) {
            if let Some(arr) = first.as_array_mut() {
                if !arr.is_empty() {
                    arr[0] = Value::String(prompt.to_string());
                }
            }
        }

        // Reset conversation metadata (inner[2]) for new conversation
        if inner.len() > 2 {
            inner[2] = Value::Null;
        }

        // Build the outer structure
        let inner_json = serde_json::to_string(&inner)
            .map_err(|e| format!("{} Inner serialize failed: {}", LOG_TAG, e))?;

        let outer = json!([null, inner_json]);
        let outer_json = serde_json::to_string(&outer)
            .map_err(|e| format!("{} Outer serialize failed: {}", LOG_TAG, e))?;

        let freq = urlencoding::encode(&outer_json);
        let at = urlencoding::encode(&template.at_token);
        let body = format!("f.req={}&at={}&", freq, at);

        // Execute fetch via CDP
        let js = format!(
            r#"
            (async () => {{
                try {{
                    const resp = await fetch('{}', {{
                        method: 'POST',
                        credentials: 'include',
                        headers: {{
                            'Content-Type': 'application/x-www-form-urlencoded;charset=UTF-8',
                        }},
                        body: `{}`
                    }});
                    const text = await resp.text();
                    return text;
                }} catch (e) {{
                    return 'ERROR:' + e.message;
                }}
            }})()
            "#,
            template.url,
            body.replace('`', "\\`").replace('\\', "\\\\"),
        );

        let response = cdp.evaluate_js(&js).await?;
        let response_text = response
            .as_str()
            .ok_or_else(|| format!("{} Response is not a string", LOG_TAG))?;

        if response_text.starts_with("ERROR:") {
            return Err(format!("{} Fetch error: {}", LOG_TAG, response_text));
        }

        // Parse batchexecute response
        Self::parse_batchexecute_response(response_text)
    }

    /// Parse Gemini's batchexecute response format.
    ///
    /// Response is multiline, each line is a JSON array.
    /// We look for items where `item[0] === "wrb.fr"`.
    /// The text content is at `data[4][0][1]`.
    fn parse_batchexecute_response(raw: &str) -> Result<String, String> {
        let mut extracted_text = String::new();

        for line in raw.lines() {
            let trimmed = line.trim();
            if !trimmed.starts_with('[') {
                continue;
            }

            if let Ok(arr) = serde_json::from_str::<Vec<Value>>(trimmed) {
                for item in &arr {
                    if let Some(inner_arr) = item.as_array() {
                        // Check if this is a wrb.fr item
                        if inner_arr
                            .first()
                            .and_then(|v| v.as_str())
                            == Some("wrb.fr")
                        {
                            // Parse inner_arr[2] as JSON
                            if let Some(data_str) = inner_arr.get(2).and_then(|v| v.as_str()) {
                                if let Ok(data) = serde_json::from_str::<Value>(data_str) {
                                    // Text at data[4][0][1]
                                    if let Some(text) = data
                                        .get(4)
                                        .and_then(|v| v.get(0))
                                        .and_then(|v| v.get(1))
                                    {
                                        match text {
                                            Value::String(s) => {
                                                extracted_text.push_str(s);
                                            }
                                            Value::Array(arr) => {
                                                // String array — join
                                                for part in arr {
                                                    if let Some(s) = part.as_str() {
                                                        extracted_text.push_str(s);
                                                    }
                                                }
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        if extracted_text.is_empty() {
            Err(format!("{} No text found in batchexecute response", LOG_TAG))
        } else {
            Ok(extracted_text)
        }
    }
}

#[async_trait]
impl WebProvider for GeminiWebProvider {
    fn id(&self) -> &str {
        "gemini"
    }

    fn name(&self) -> &str {
        "Gemini Web"
    }

    fn login_url(&self) -> &str {
        "https://gemini.google.com"
    }

    fn models(&self) -> &[WebAuthModel] {
        &self.models
    }

    async fn check_auth(&self, cdp: &CdpClient) -> AuthCheckResult {
        cookie_auth::check_provider_auth(cdp, "gemini", self.login_url()).await
    }

    async fn initialize(&self, cdp: &CdpClient) -> Result<(), String> {
        let template = self.capture_template(cdp).await?;
        let mut cached = self.cached_template.lock().unwrap();
        *cached = Some(template);
        Ok(())
    }

    async fn chat(&self, cdp: &CdpClient, prompt: &str) -> Result<String, String> {
        // Get or capture template
        let template = {
            let cached = self.cached_template.lock().unwrap();
            cached.clone()
        };

        let template = match template {
            Some(t) => t,
            None => {
                tracing::info!("{} No cached template, capturing...", LOG_TAG);
                let t = self.capture_template(cdp).await?;
                {
                    let mut cached = self.cached_template.lock().unwrap();
                    *cached = Some(t.clone());
                }
                t
            }
        };

        // Check for account switch
        let current_url = cdp.get_url().await.unwrap_or_default();
        let current_prefix = if let Some(idx) = current_url.find("/u/") {
            let end = current_url[idx + 3..]
                .find('/')
                .map(|i| idx + 3 + i + 1)
                .unwrap_or(idx + 5);
            current_url[idx..end].to_string()
        } else {
            String::new()
        };

        if current_prefix != template.account_prefix {
            tracing::warn!(
                "{} Account changed ({} → {}), recapturing template...",
                LOG_TAG,
                template.account_prefix,
                current_prefix
            );
            let t = self.capture_template(cdp).await?;
            {
                let mut cached = self.cached_template.lock().unwrap();
                *cached = Some(t.clone());
            }
            return self.send_with_template(cdp, &t, prompt).await;
        }

        self.send_with_template(cdp, &template, prompt).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_info() {
        let p = GeminiWebProvider::new();
        assert_eq!(p.id(), "gemini");
        assert_eq!(p.models().len(), 2);
        assert!(p.models().iter().any(|m| m.id == "webauth-gemini-pro"));
        assert!(p.models().iter().any(|m| m.id == "webauth-gemini-flash"));
    }

    #[test]
    fn test_parse_batchexecute_response() {
        // Gemini batchexecute: data[4][0][1] = text content
        // data[4][0] is [prompt_echo, response_text]
        let inner_data = json!([
            null, null, null, null,
            [["echo", "Hello! How can I help you?"]]
        ]);
        let inner_str = serde_json::to_string(&inner_data).unwrap();

        let wrb_item = json!(["wrb.fr", "method", inner_str, null, null, null, "generic"]);
        let line = serde_json::to_string(&json!([wrb_item])).unwrap();

        let raw = String::from(")]}\'\n\n") + &line;

        let result = GeminiWebProvider::parse_batchexecute_response(&raw);
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        let text = result.unwrap();
        assert!(text.contains("Hello"), "Expected 'Hello' in: {}", text);
    }

    #[test]
    fn test_parse_empty_response() {
        let raw = "some random text\n";
        let result = GeminiWebProvider::parse_batchexecute_response(raw);
        assert!(result.is_err());
    }
}
