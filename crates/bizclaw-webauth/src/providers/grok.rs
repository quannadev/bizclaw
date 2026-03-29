//! Grok Web Provider — X.com's AI using session cookies.
//!
//! Grok is embedded in X.com (Twitter) and uses the X API with
//! the user's auth_token cookie for authentication.

use async_trait::async_trait;

use crate::cdp::CdpClient;
use crate::cookie_auth;
use crate::types::{AuthCheckResult, WebAuthModel};

use super::WebProvider;

const LOG_TAG: &str = "[Grok-Web]";
const API_BASE: &str = "https://api.x.com/2/grok";

pub struct GrokWebProvider {
    models: Vec<WebAuthModel>,
}

impl Default for GrokWebProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl GrokWebProvider {
    pub fn new() -> Self {
        Self {
            models: vec![WebAuthModel {
                id: "webauth-grok".to_string(),
                name: "Grok (WebAuth)".to_string(),
                context_window: 128000,
            }],
        }
    }

    /// Get bearer token from X.com page.
    async fn get_bearer_token(&self, cdp: &CdpClient) -> Result<String, String> {
        // X.com stores the bearer token in a main.js bundle
        // We can capture it from the page's runtime
        let js = r#"
            (async () => {
                try {
                    // Try to get from existing API calls
                    const token = document.querySelector('meta[name="bearer-token"]')?.content;
                    if (token) return token;

                    // Fallback: use the known public bearer token
                    return 'AAAAAAAAAAAAAAAAAAAAANRILgAAAAAAnNwIzUejRCOuH5E6I8xnZz4puTs=1Zv7ttfk8LF81IUq16cHjhLTvJu4FA33AGWWjCpTnA';
                } catch (e) {
                    return '';
                }
            })()
        "#;

        let result = cdp.evaluate_js(js).await?;
        result
            .as_str()
            .filter(|s| !s.is_empty())
            .map(String::from)
            .ok_or_else(|| format!("{} Could not get bearer token", LOG_TAG))
    }

    /// Get CSRF token from cookies.
    async fn get_csrf_token(&self, cdp: &CdpClient) -> Result<String, String> {
        let cookies = cdp.get_cookies(&["https://x.com"]).await?;
        cookies
            .iter()
            .find_map(|c| {
                if c.get("name")?.as_str()? == "ct0" {
                    c.get("value")?.as_str().map(String::from)
                } else {
                    None
                }
            })
            .ok_or_else(|| format!("{} No ct0 CSRF cookie found", LOG_TAG))
    }

    async fn send_message(
        &self,
        cdp: &CdpClient,
        bearer: &str,
        csrf: &str,
        prompt: &str,
    ) -> Result<String, String> {
        let escaped_prompt = prompt
            .replace('\\', "\\\\")
            .replace('\"', "\\\"")
            .replace('\n', "\\n");
        let escaped_bearer = bearer.replace('\\', "\\\\").replace('\"', "\\\"");
        let escaped_csrf = csrf.replace('\\', "\\\\").replace('\"', "\\\"");

        let js = format!(
            r#"
            (async () => {{
                try {{
                    const resp = await fetch('{API_BASE}/add_response.json', {{
                        method: 'POST',
                        credentials: 'include',
                        headers: {{
                            'Content-Type': 'application/json',
                            'Authorization': 'Bearer {escaped_bearer}',
                            'x-csrf-token': '{escaped_csrf}',
                            'x-twitter-active-user': 'yes',
                            'x-twitter-auth-type': 'OAuth2Session',
                        }},
                        body: JSON.stringify({{
                            responses: [{{
                                message: "{escaped_prompt}",
                                sender: 1
                            }}],
                            systemPromptName: '',
                            grokModelOptionId: 'grok-2',
                            conversationId: '',
                        }})
                    }});

                    const text = await resp.text();

                    // Parse JSONL response
                    let fullText = '';
                    for (const line of text.split('\\n')) {{
                        if (line.trim()) {{
                            try {{
                                const d = JSON.parse(line);
                                if (d.result?.message) {{
                                    fullText = d.result.message;
                                }}
                            }} catch (e) {{}}
                        }}
                    }}

                    return fullText || 'ERROR:Empty response';
                }} catch (e) {{
                    return 'ERROR:' + e.message;
                }}
            }})()
            "#
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
impl WebProvider for GrokWebProvider {
    fn id(&self) -> &str {
        "grok"
    }

    fn name(&self) -> &str {
        "Grok Web"
    }

    fn login_url(&self) -> &str {
        "https://x.com"
    }

    fn models(&self) -> &[WebAuthModel] {
        &self.models
    }

    async fn check_auth(&self, cdp: &CdpClient) -> AuthCheckResult {
        cookie_auth::check_provider_auth(cdp, "grok", self.login_url()).await
    }

    async fn chat(&self, cdp: &CdpClient, prompt: &str) -> Result<String, String> {
        let bearer = self.get_bearer_token(cdp).await?;
        let csrf = self.get_csrf_token(cdp).await?;
        self.send_message(cdp, &bearer, &csrf, prompt).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_info() {
        let p = GrokWebProvider::new();
        assert_eq!(p.id(), "grok");
        assert_eq!(p.login_url(), "https://x.com");
        assert_eq!(p.models().len(), 1);
    }
}
