//! Cookie-based authentication checking for WebAuth providers.
//!
//! Each provider has specific cookies that indicate an active login session.
//! This module checks those cookies via CDP to determine if the user is
//! authenticated.

use crate::cdp::CdpClient;
use crate::types::AuthCheckResult;
use serde_json::Value;
use tracing;

const LOG_TAG: &str = "[CookieAuth]";

/// Cookie requirement for a provider.
#[derive(Debug, Clone)]
pub struct CookieRequirement {
    /// Cookie name to check
    pub name: &'static str,
    /// Domain the cookie belongs to
    pub domain: &'static str,
    /// Whether this cookie is required (vs nice-to-have)
    pub required: bool,
}

/// Provider cookie configurations.
pub fn get_cookie_requirements(provider_id: &str) -> Vec<CookieRequirement> {
    match provider_id {
        "gemini" => vec![
            CookieRequirement {
                name: "__Secure-1PSID",
                domain: ".google.com",
                required: true,
            },
            CookieRequirement {
                name: "__Secure-1PSIDTS",
                domain: ".google.com",
                required: false,
            },
        ],
        "claude" => vec![CookieRequirement {
            name: "sessionKey",
            domain: "claude.ai",
            required: true,
        }],
        "chatgpt" => vec![CookieRequirement {
            name: "__Secure-next-auth.session-token",
            domain: ".chatgpt.com",
            required: true,
        }],
        "deepseek" => vec![CookieRequirement {
            name: "ds_session",
            domain: "chat.deepseek.com",
            required: true,
        }],
        "grok" => vec![CookieRequirement {
            name: "auth_token",
            domain: ".x.com",
            required: true,
        }],
        "qwen" => vec![CookieRequirement {
            name: "cna",
            domain: ".tongyi.aliyun.com",
            required: true,
        }],
        "kimi" => vec![CookieRequirement {
            name: "access_token",
            domain: "kimi.moonshot.cn",
            required: true,
        }],
        _ => vec![],
    }
}

/// Check if a provider is authenticated by verifying cookies via CDP.
pub async fn check_provider_auth(
    cdp: &CdpClient,
    provider_id: &str,
    login_url: &str,
) -> AuthCheckResult {
    let requirements = get_cookie_requirements(provider_id);
    if requirements.is_empty() {
        tracing::warn!(
            "{} No cookie requirements defined for {}",
            LOG_TAG,
            provider_id
        );
        return AuthCheckResult {
            authenticated: false,
            user: None,
        };
    }

    // Get cookies via CDP
    let cookies = match cdp.get_cookies(&[login_url]).await {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(
                "{} Failed to get cookies for {}: {}",
                LOG_TAG,
                provider_id,
                e
            );
            return AuthCheckResult {
                authenticated: false,
                user: None,
            };
        }
    };

    // Check required cookies
    let mut all_required_present = true;
    let mut user_info = None;

    for req in &requirements {
        let found = cookies.iter().any(|c| {
            c.get("name").and_then(|n| n.as_str()) == Some(req.name)
                && c.get("value")
                    .and_then(|v| v.as_str())
                    .map(|v| !v.is_empty())
                    .unwrap_or(false)
        });

        if req.required && !found {
            tracing::debug!(
                "{} Missing required cookie '{}' for {}",
                LOG_TAG,
                req.name,
                provider_id
            );
            all_required_present = false;
        }
    }

    // Try to extract user info from cookies
    if all_required_present {
        // For Google/Gemini, try SAPISID or account name
        if provider_id == "gemini" {
            user_info = cookies.iter().find_map(|c| {
                if c.get("name").and_then(|n| n.as_str()) == Some("SAPISID") {
                    c.get("value").and_then(|v| v.as_str()).map(|s| {
                        let chars: Vec<char> = s.chars().collect();
                        if chars.len() > 8 {
                            let prefix: String = chars[..4].iter().collect();
                            let suffix: String = chars[chars.len() - 4..].iter().collect();
                            format!("{}...{}", prefix, suffix)
                        } else {
                            s.to_string()
                        }
                    })
                } else {
                    None
                }
            });
        }

        tracing::info!(
            "{} {} is authenticated (user: {:?})",
            LOG_TAG,
            provider_id,
            user_info
        );
    }

    AuthCheckResult {
        authenticated: all_required_present,
        user: user_info,
    }
}

/// Check authentication without CDP, by examining a cookie jar (for non-browser mode).
pub fn check_auth_from_cookie_string(provider_id: &str, cookie_header: &str) -> AuthCheckResult {
    let requirements = get_cookie_requirements(provider_id);
    if requirements.is_empty() {
        return AuthCheckResult {
            authenticated: false,
            user: None,
        };
    }

    let cookie_map: std::collections::HashMap<&str, &str> = cookie_header
        .split(';')
        .filter_map(|part| {
            let mut iter = part.trim().splitn(2, '=');
            Some((iter.next()?, iter.next().unwrap_or("")))
        })
        .collect();

    let all_required = requirements.iter().filter(|r| r.required).all(|r| {
        cookie_map
            .get(r.name)
            .map(|v| !v.is_empty())
            .unwrap_or(false)
    });

    AuthCheckResult {
        authenticated: all_required,
        user: None,
    }
}

/// Extract cookies from a CDP cookie array as a header string.
pub fn cookies_to_header(cookies: &[Value]) -> String {
    cookies
        .iter()
        .filter_map(|c| {
            let name = c.get("name")?.as_str()?;
            let value = c.get("value")?.as_str()?;
            Some(format!("{}={}", name, value))
        })
        .collect::<Vec<_>>()
        .join("; ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gemini_cookie_requirements() {
        let reqs = get_cookie_requirements("gemini");
        assert!(!reqs.is_empty());
        assert!(
            reqs.iter()
                .any(|r| r.name == "__Secure-1PSID" && r.required)
        );
    }

    #[test]
    fn test_claude_cookie_requirements() {
        let reqs = get_cookie_requirements("claude");
        assert_eq!(reqs.len(), 1);
        assert_eq!(reqs[0].name, "sessionKey");
    }

    #[test]
    fn test_check_auth_from_cookie_string() {
        let result =
            check_auth_from_cookie_string("gemini", "__Secure-1PSID=abc123; other_cookie=xyz");
        assert!(result.authenticated);

        let result = check_auth_from_cookie_string("gemini", "other_cookie=xyz");
        assert!(!result.authenticated);
    }

    #[test]
    fn test_unknown_provider() {
        let reqs = get_cookie_requirements("unknown_provider");
        assert!(reqs.is_empty());
    }

    #[test]
    fn test_cookies_to_header() {
        let cookies = vec![
            serde_json::json!({"name": "a", "value": "1"}),
            serde_json::json!({"name": "b", "value": "2"}),
        ];
        let header = cookies_to_header(&cookies);
        assert_eq!(header, "a=1; b=2");
    }
}
