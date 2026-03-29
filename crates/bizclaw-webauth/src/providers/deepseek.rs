//! DeepSeek Web Provider — SSE chat API using session cookies.
//!
//! DeepSeek's web chat at `chat.deepseek.com` uses a straightforward
//! REST API with session cookie auth.

use async_trait::async_trait;

use crate::cdp::CdpClient;
use crate::cookie_auth;
use crate::types::{AuthCheckResult, WebAuthModel};

use super::WebProvider;

const LOG_TAG: &str = "[DeepSeek-Web]";
const API_BASE: &str = "https://chat.deepseek.com/api/v0";

pub struct DeepSeekWebProvider {
    models: Vec<WebAuthModel>,
}

impl Default for DeepSeekWebProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl DeepSeekWebProvider {
    pub fn new() -> Self {
        Self {
            models: vec![
                WebAuthModel {
                    id: "webauth-deepseek-chat".to_string(),
                    name: "DeepSeek Chat (WebAuth)".to_string(),
                    context_window: 128000,
                },
                WebAuthModel {
                    id: "webauth-deepseek-reasoner".to_string(),
                    name: "DeepSeek Reasoner (WebAuth)".to_string(),
                    context_window: 128000,
                },
            ],
        }
    }

    async fn send_message(
        &self,
        cdp: &CdpClient,
        prompt: &str,
        use_search: bool,
    ) -> Result<String, String> {
        let escaped_prompt = prompt
            .replace('\\', "\\\\")
            .replace('\"', "\\\"")
            .replace('\n', "\\n");

        let js = format!(
            r#"
            (async () => {{
                try {{
                    // Create chat session
                    const createResp = await fetch('{API_BASE}/chat_session/create', {{
                        method: 'POST',
                        credentials: 'include',
                        headers: {{ 'Content-Type': 'application/json' }},
                        body: JSON.stringify({{ agent: 'chat' }})
                    }});
                    const session = await createResp.json();
                    const chatId = session.data?.biz_data?.id;
                    if (!chatId) return 'ERROR:No chat session ID';

                    // Send message
                    const chatResp = await fetch('{API_BASE}/chat/completion', {{
                        method: 'POST',
                        credentials: 'include',
                        headers: {{
                            'Content-Type': 'application/json',
                            'Accept': 'text/event-stream',
                        }},
                        body: JSON.stringify({{
                            chat_session_id: chatId,
                            prompt: "{escaped_prompt}",
                            ref_file_ids: [],
                            search_enabled: {search},
                            thinking_enabled: false,
                        }})
                    }});

                    const text = await chatResp.text();

                    // Parse SSE
                    let fullText = '';
                    for (const line of text.split('\\n')) {{
                        if (line.startsWith('data: ')) {{
                            try {{
                                const d = JSON.parse(line.slice(6));
                                if (d.choices?.[0]?.delta?.content) {{
                                    fullText += d.choices[0].delta.content;
                                }}
                            }} catch (e) {{}}
                        }}
                    }}

                    // Cleanup chat session
                    fetch(`{API_BASE}/chat_session/${{chatId}}/delete`, {{
                        method: 'POST',
                        credentials: 'include',
                    }}).catch(() => {{}});

                    return fullText || 'ERROR:Empty response';
                }} catch (e) {{
                    return 'ERROR:' + e.message;
                }}
            }})()
            "#,
            search = if use_search { "true" } else { "false" }
        );

        let result = cdp.evaluate_js(&js).await?;
        let text = result.as_str().unwrap_or("");

        if text.starts_with("ERROR:") || text.is_empty() {
            Err(format!("{} {}", LOG_TAG, text))
        } else {
            Ok(text.to_string())
        }
    }
}

#[async_trait]
impl WebProvider for DeepSeekWebProvider {
    fn id(&self) -> &str {
        "deepseek"
    }

    fn name(&self) -> &str {
        "DeepSeek Web"
    }

    fn login_url(&self) -> &str {
        "https://chat.deepseek.com"
    }

    fn models(&self) -> &[WebAuthModel] {
        &self.models
    }

    async fn check_auth(&self, cdp: &CdpClient) -> AuthCheckResult {
        cookie_auth::check_provider_auth(cdp, "deepseek", self.login_url()).await
    }

    async fn chat(&self, cdp: &CdpClient, prompt: &str) -> Result<String, String> {
        self.send_message(cdp, prompt, false).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_info() {
        let p = DeepSeekWebProvider::new();
        assert_eq!(p.id(), "deepseek");
        assert_eq!(p.models().len(), 2);
        assert!(p.models().iter().any(|m| m.id == "webauth-deepseek-chat"));
    }
}
