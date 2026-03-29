//! ChatGPT Web Provider — conversation API using session cookies.
//!
//! ChatGPT uses `https://chatgpt.com/backend-api/conversation` with
//! session tokens for authentication. Responses are SSE-streamed.
//!
//! **Unique challenge**: ChatGPT has a code sandbox and tries to use it
//! instead of outputting tool calls. The system prompt uses "Two Environments"
//! framing to prevent this.

use async_trait::async_trait;

use crate::cdp::CdpClient;
use crate::cookie_auth;
use crate::types::{AuthCheckResult, WebAuthModel};

use super::WebProvider;

const LOG_TAG: &str = "[ChatGPT-Web]";
const API_BASE: &str = "https://chatgpt.com/backend-api";

pub struct ChatGPTWebProvider {
    models: Vec<WebAuthModel>,
}

impl Default for ChatGPTWebProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl ChatGPTWebProvider {
    pub fn new() -> Self {
        Self {
            models: vec![
                WebAuthModel {
                    id: "webauth-chatgpt-4o".to_string(),
                    name: "ChatGPT-4o (WebAuth)".to_string(),
                    context_window: 128000,
                },
                WebAuthModel {
                    id: "webauth-chatgpt-o1".to_string(),
                    name: "ChatGPT o1 (WebAuth)".to_string(),
                    context_window: 128000,
                },
            ],
        }
    }

    /// Get the access token from the session page.
    async fn get_access_token(&self, cdp: &CdpClient) -> Result<String, String> {
        let js = format!(
            r#"
            (async () => {{
                try {{
                    const resp = await fetch('{API_BASE}/auth/session', {{
                        credentials: 'include'
                    }});
                    const data = await resp.json();
                    return data.accessToken || '';
                }} catch (e) {{
                    return 'ERROR:' + e.message;
                }}
            }})()
            "#
        );
        let result = cdp.evaluate_js(&js).await?;
        let token = result.as_str().unwrap_or("");

        if token.is_empty() || token.starts_with("ERROR:") {
            Err(format!("{} Could not get access token: {}", LOG_TAG, token))
        } else {
            Ok(token.to_string())
        }
    }

    /// Send a message via the conversation API.
    async fn send_message(
        &self,
        cdp: &CdpClient,
        access_token: &str,
        prompt: &str,
        model: &str,
    ) -> Result<String, String> {
        let escaped_prompt = prompt
            .replace('\\', "\\\\")
            .replace('\"', "\\\"")
            .replace('\n', "\\n")
            .replace('\r', "\\r");
        let escaped_token = access_token.replace('\\', "\\\\").replace('\"', "\\\"");

        let js = format!(
            r#"
            (async () => {{
                try {{
                    const msgId = crypto.randomUUID();
                    const parentId = crypto.randomUUID();

                    const resp = await fetch('{API_BASE}/conversation', {{
                        method: 'POST',
                        credentials: 'include',
                        headers: {{
                            'Content-Type': 'application/json',
                            'Authorization': 'Bearer {escaped_token}',
                        }},
                        body: JSON.stringify({{
                            action: 'next',
                            messages: [{{
                                id: msgId,
                                author: {{ role: 'user' }},
                                content: {{ content_type: 'text', parts: ["{escaped_prompt}"] }},
                                metadata: {{}}
                            }}],
                            parent_message_id: parentId,
                            model: '{model}',
                            timezone_offset_min: -420,
                            history_and_training_disabled: true,
                        }})
                    }});

                    const text = await resp.text();

                    // Parse SSE: look for [DONE] or data blocks
                    let lastContent = '';
                    let convId = '';
                    for (const line of text.split('\\n')) {{
                        if (line.startsWith('data: ') && line !== 'data: [DONE]') {{
                            try {{
                                const d = JSON.parse(line.slice(6));
                                if (d.conversation_id) convId = d.conversation_id;
                                const parts = d.message?.content?.parts;
                                if (parts && parts.length > 0 && typeof parts[0] === 'string') {{
                                    lastContent = parts[0];
                                }}
                            }} catch (e) {{}}
                        }}
                    }}

                    // Cleanup: delete conversation
                    if (convId) {{
                        fetch(`{API_BASE}/conversation/${{convId}}`, {{
                            method: 'PATCH',
                            credentials: 'include',
                            headers: {{
                                'Content-Type': 'application/json',
                                'Authorization': 'Bearer {escaped_token}'
                            }},
                            body: JSON.stringify({{ is_visible: false }})
                        }}).catch(() => {{}});
                    }}

                    return lastContent || 'ERROR:No content in response';
                }} catch (e) {{
                    return 'ERROR:' + e.message;
                }}
            }})()
            "#
        );

        let result = cdp.evaluate_js(&js).await?;
        let text = result
            .as_str()
            .ok_or_else(|| format!("{} Response is not a string", LOG_TAG))?;

        if text.starts_with("ERROR:") {
            return Err(format!("{} {}", LOG_TAG, text));
        }

        Ok(text.to_string())
    }
}

#[async_trait]
impl WebProvider for ChatGPTWebProvider {
    fn id(&self) -> &str {
        "chatgpt"
    }

    fn name(&self) -> &str {
        "ChatGPT Web"
    }

    fn login_url(&self) -> &str {
        "https://chatgpt.com"
    }

    fn models(&self) -> &[WebAuthModel] {
        &self.models
    }

    async fn check_auth(&self, cdp: &CdpClient) -> AuthCheckResult {
        cookie_auth::check_provider_auth(cdp, "chatgpt", self.login_url()).await
    }

    async fn chat(&self, cdp: &CdpClient, prompt: &str) -> Result<String, String> {
        let token = self.get_access_token(cdp).await?;
        let model = "gpt-4o";
        self.send_message(cdp, &token, prompt, model).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_info() {
        let p = ChatGPTWebProvider::new();
        assert_eq!(p.id(), "chatgpt");
        assert_eq!(p.login_url(), "https://chatgpt.com");
        assert!(p.models().iter().any(|m| m.id == "webauth-chatgpt-4o"));
    }
}
