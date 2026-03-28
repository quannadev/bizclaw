//! Secret & PII redaction for tenant data protection.
//!
//! Ported from DataClaw's `secrets.py` (MIT License) — regex-based detection
//! of API keys, tokens, connection strings, PII, and high-entropy secrets in
//! arbitrary text. Designed for use as middleware in the gateway's WebSocket,
//! knowledge base ingest, and audit log pipelines.
//!
//! ## Usage
//! ```rust,no_run
//! use bizclaw_security::redactor::SecretRedactor;
//!
//! let redactor = SecretRedactor::new();
//! let (clean, count) = redactor.redact("My key is sk-ant-abc123xyz456...");
//! assert!(count > 0);
//! assert!(clean.contains("[REDACTED]"));
//! ```

use regex::Regex;
use std::sync::LazyLock;

/// Replacement marker for redacted content.
pub const REDACTED: &str = "[REDACTED]";

/// A named pattern for secret detection.
struct SecretPattern {
    name: &'static str,
    regex: Regex,
}

/// Pre-compiled patterns — initialized once, shared across all calls.
static SECRET_PATTERNS: LazyLock<Vec<SecretPattern>> = LazyLock::new(|| {
    vec![
        // ── Tokens & API Keys ──
        SecretPattern {
            name: "jwt",
            regex: Regex::new(r"eyJ[A-Za-z0-9_-]{20,}\.[A-Za-z0-9_-]{20,}\.[A-Za-z0-9_-]{10,}")
                .unwrap(),
        },
        SecretPattern {
            name: "jwt_partial",
            regex: Regex::new(r"eyJ[A-Za-z0-9_-]{15,}").unwrap(),
        },
        SecretPattern {
            name: "anthropic_key",
            regex: Regex::new(r"sk-ant-[A-Za-z0-9_-]{20,}").unwrap(),
        },
        SecretPattern {
            name: "openai_key",
            regex: Regex::new(r"sk-[A-Za-z0-9]{40,}").unwrap(),
        },
        SecretPattern {
            name: "hf_token",
            regex: Regex::new(r"hf_[A-Za-z0-9]{20,}").unwrap(),
        },
        SecretPattern {
            name: "github_token",
            regex: Regex::new(r"(?:ghp|gho|ghs|ghr)_[A-Za-z0-9]{30,}").unwrap(),
        },
        SecretPattern {
            name: "pypi_token",
            regex: Regex::new(r"pypi-[A-Za-z0-9_-]{50,}").unwrap(),
        },
        SecretPattern {
            name: "npm_token",
            regex: Regex::new(r"npm_[A-Za-z0-9]{30,}").unwrap(),
        },
        SecretPattern {
            name: "slack_token",
            regex: Regex::new(r"xox[bpsa]-[A-Za-z0-9-]{20,}").unwrap(),
        },
        // ── Cloud Provider Keys ──
        SecretPattern {
            name: "aws_key",
            // Rust regex doesn't support lookbehind — use word boundary instead
            regex: Regex::new(r"\bAKIA[0-9A-Z]{16}\b").unwrap(),
        },
        SecretPattern {
            name: "aws_secret",
            regex: Regex::new(
                r#"(?i)(?:aws_secret_access_key|secret_key)\s*[=:]\s*['"]?([A-Za-z0-9/+=]{40})['"]?"#,
            )
            .unwrap(),
        },
        // ── Database & Infrastructure ──
        SecretPattern {
            name: "db_url",
            regex: Regex::new(r"postgres(?:ql)?://[^:]+:[^@\s]+@[^\s\x22'`]+").unwrap(),
        },
        SecretPattern {
            name: "discord_webhook",
            regex: Regex::new(
                r"https?://(?:discord\.com|discordapp\.com)/api/webhooks/\d+/[A-Za-z0-9_-]{20,}",
            )
            .unwrap(),
        },
        SecretPattern {
            name: "private_key",
            regex: Regex::new(
                r"-----BEGIN (?:RSA |EC |DSA |OPENSSH )?PRIVATE KEY-----[\s\S]*?-----END (?:RSA |EC |DSA |OPENSSH )?PRIVATE KEY-----",
            )
            .unwrap(),
        },
        // ── CLI flags & env vars with secrets ──
        SecretPattern {
            name: "cli_token_flag",
            regex: Regex::new(
                r"(?i)(?:--|-)(?:access[_-]?token|auth[_-]?token|api[_-]?key|secret|password|token)[\s=]+([A-Za-z0-9_/+=.\-]{8,})",
            )
            .unwrap(),
        },
        SecretPattern {
            name: "env_secret",
            regex: Regex::new(
                r#"(?i)(?:SECRET|PASSWORD|TOKEN|API_KEY|AUTH_KEY|ACCESS_KEY|SERVICE_KEY|DB_PASSWORD|SUPABASE_KEY|SUPABASE_SERVICE|ANON_KEY|SERVICE_ROLE)\s*[=]\s*['"]?([^\s'"]{6,})['"]?"#,
            )
            .unwrap(),
        },
        SecretPattern {
            name: "generic_secret",
            regex: Regex::new(
                r#"(?i)(?:secret[_-]?key|api[_-]?key|api[_-]?secret|access[_-]?token|auth[_-]?token|service[_-]?role[_-]?key|private[_-]?key)\s*[=:]\s*['"]([A-Za-z0-9_/+=.\-]{20,})['"]"#,
            )
            .unwrap(),
        },
        // ── Bearer & URL tokens ──
        SecretPattern {
            name: "bearer",
            regex: Regex::new(
                r"Bearer\s+(eyJ[A-Za-z0-9_-]{20,}\.[A-Za-z0-9_-]{20,}\.[A-Za-z0-9_-]{20,})",
            )
            .unwrap(),
        },
        SecretPattern {
            name: "url_token",
            regex: Regex::new(
                r"(?i)[?&](?:key|token|secret|password|apikey|api_key|access_token|auth)=([A-Za-z0-9_/+=.\-]{8,})",
            )
            .unwrap(),
        },
        // ── PII ──
        SecretPattern {
            name: "ip_address",
            // Rust regex doesn't support lookahead — match all IPs and let allowlist filter
            regex: Regex::new(
                r"\b(?:(?:25[0-5]|2[0-4][0-9]|1[0-9][0-9]|[1-9]?[0-9])\.){3}(?:25[0-5]|2[0-4][0-9]|1[0-9][0-9]|[1-9]?[0-9])\b",
            )
            .unwrap(),
        },
        SecretPattern {
            name: "email",
            regex: Regex::new(r"\b[A-Za-z0-9._%+-]{2,}@[A-Za-z0-9.-]+\.[A-Za-z]{2,}\b").unwrap(),
        },
        // ── Entropy-based detection ──
        SecretPattern {
            name: "high_entropy",
            regex: Regex::new(r#"['"][A-Za-z0-9_/+=.\-]{40,}['"]"#).unwrap(),
        },
    ]
});

/// Pre-compiled allowlist patterns to avoid false positives.
static ALLOWLIST: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    vec![
        Regex::new(r"noreply@").unwrap(),
        Regex::new(r"@example\.com").unwrap(),
        Regex::new(r"@localhost").unwrap(),
        Regex::new(r"@anthropic\.com").unwrap(),
        Regex::new(r"@github\.com").unwrap(),
        Regex::new(r"@users\.noreply\.github\.com").unwrap(),
        Regex::new(r"AKIA\[").unwrap(),
        Regex::new(r"sk-ant-\.\*").unwrap(),
        Regex::new(r"postgres://user:pass@").unwrap(),
        Regex::new(r"postgres://username:password@").unwrap(),
        Regex::new(r"@pytest").unwrap(),
        Regex::new(r"@tasks\.").unwrap(),
        Regex::new(r"@mcp\.").unwrap(),
        Regex::new(r"@server\.").unwrap(),
        Regex::new(r"@app\.").unwrap(),
        Regex::new(r"@router\.").unwrap(),
        // Private / well-known IPs
        Regex::new(r"192\.168\.").unwrap(),
        Regex::new(r"10\.\d+\.\d+\.\d+").unwrap(),
        Regex::new(r"172\.(?:1[6-9]|2\d|3[01])\.").unwrap(),
        Regex::new(r"127\.0\.0\.").unwrap(),
        Regex::new(r"0\.0\.0\.0").unwrap(),
        Regex::new(r"255\.255\.").unwrap(),
        Regex::new(r"8\.8\.8\.8").unwrap(),
        Regex::new(r"8\.8\.4\.4").unwrap(),
        Regex::new(r"1\.1\.1\.1").unwrap(),
    ]
});

/// A finding from `scan_text`.
#[derive(Debug, Clone)]
pub struct SecretFinding {
    pub pattern_name: &'static str,
    pub start: usize,
    pub end: usize,
    pub matched_text: String,
}

/// Stateless secret scanner & redactor.
///
/// Thread-safe: all patterns are compiled once into static storage.
pub struct SecretRedactor;

impl SecretRedactor {
    /// Create a new redactor instance (zero allocation — patterns are static).
    pub fn new() -> Self {
        Self
    }

    /// Scan text for potential secrets. Returns a list of findings.
    pub fn scan(&self, text: &str) -> Vec<SecretFinding> {
        if text.is_empty() {
            return Vec::new();
        }

        let mut findings = Vec::new();

        for pattern in SECRET_PATTERNS.iter() {
            for m in pattern.regex.find_iter(text) {
                let matched_text = m.as_str();

                // Skip allowlisted matches
                if ALLOWLIST.iter().any(|a| a.is_match(matched_text)) {
                    continue;
                }

                // High-entropy check: verify the inner string actually looks secret
                if pattern.name == "high_entropy" {
                    let inner = &matched_text[1..matched_text.len() - 1]; // strip quotes
                    if !has_mixed_char_types(inner) {
                        continue;
                    }
                    if shannon_entropy(inner) < 3.5 {
                        continue;
                    }
                    // Skip version-like strings with many dots
                    if inner.chars().filter(|&c| c == '.').count() > 2 {
                        continue;
                    }
                }

                findings.push(SecretFinding {
                    pattern_name: pattern.name,
                    start: m.start(),
                    end: m.end(),
                    matched_text: matched_text.to_string(),
                });
            }
        }

        findings
    }

    /// Scan and redact all secrets from text.
    /// Returns `(redacted_text, redaction_count)`.
    pub fn redact(&self, text: &str) -> (String, usize) {
        if text.is_empty() {
            return (String::new(), 0);
        }

        let mut findings = self.scan(text);
        if findings.is_empty() {
            return (text.to_string(), 0);
        }

        // Sort by start position descending — replace from end to avoid index shift
        findings.sort_by(|a, b| b.start.cmp(&a.start));

        // Deduplicate overlapping findings (keep later-starting on overlap)
        let mut deduped: Vec<&SecretFinding> = Vec::new();
        for f in &findings {
            if deduped.is_empty() || f.end <= deduped.last().unwrap().start {
                deduped.push(f);
            }
        }

        let mut result = text.to_string();
        for f in &deduped {
            result.replace_range(f.start..f.end, REDACTED);
        }

        (result, deduped.len())
    }

    /// Redact custom strings (e.g. tenant-specific company names, internal URLs).
    pub fn redact_custom(&self, text: &str, strings: &[&str]) -> (String, usize) {
        if text.is_empty() || strings.is_empty() {
            return (text.to_string(), 0);
        }

        let mut result = text.to_string();
        let mut count = 0;

        for target in strings {
            if target.len() < 3 {
                continue;
            }
            let escaped = regex::escape(target);
            let pattern = if target.len() >= 4 {
                format!(r"\b{}\b", escaped)
            } else {
                escaped
            };
            if let Ok(re) = Regex::new(&pattern) {
                let before_len = result.len();
                result = re.replace_all(&result, REDACTED).to_string();
                if result.len() != before_len {
                    count += 1;
                }
            }
        }

        (result, count)
    }
}

impl Default for SecretRedactor {
    fn default() -> Self {
        Self::new()
    }
}

// ═══ Entropy helpers ═══

/// Calculate Shannon entropy of a string.
/// Higher values indicate more random-looking (likely secret) strings.
fn shannon_entropy(s: &str) -> f64 {
    if s.is_empty() {
        return 0.0;
    }

    let mut freq = [0u32; 256];
    let len = s.len() as f64;

    for byte in s.bytes() {
        freq[byte as usize] += 1;
    }

    let mut entropy = 0.0_f64;
    for &count in &freq {
        if count > 0 {
            let p = count as f64 / len;
            entropy -= p * p.log2();
        }
    }

    entropy
}

/// Check if a string contains a mix of uppercase, lowercase, and digits.
fn has_mixed_char_types(s: &str) -> bool {
    let mut has_upper = false;
    let mut has_lower = false;
    let mut has_digit = false;

    for c in s.chars() {
        if c.is_ascii_uppercase() {
            has_upper = true;
        } else if c.is_ascii_lowercase() {
            has_lower = true;
        } else if c.is_ascii_digit() {
            has_digit = true;
        }
        if has_upper && has_lower && has_digit {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_jwt() {
        let redactor = SecretRedactor::new();
        let text = "token: eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";
        let findings = redactor.scan(text);
        assert!(!findings.is_empty());
        assert!(findings.iter().any(|f| f.pattern_name == "jwt"));
    }

    #[test]
    fn test_detect_anthropic_key() {
        let redactor = SecretRedactor::new();
        let text = "ANTHROPIC_API_KEY=sk-ant-api03-abcdefghijklmnopqrstuvwxyz1234567890";
        let findings = redactor.scan(text);
        assert!(findings.iter().any(|f| f.pattern_name == "anthropic_key"));
    }

    #[test]
    fn test_detect_openai_key() {
        let redactor = SecretRedactor::new();
        let text =
            "key is sk-abcdefghijABCDEFGHIJabcdefghijABCDEFGHIJ12345678";
        let findings = redactor.scan(text);
        assert!(findings.iter().any(|f| f.pattern_name == "openai_key"));
    }

    #[test]
    fn test_detect_github_token() {
        let redactor = SecretRedactor::new();
        let text = "GITHUB_TOKEN=ghp_abcdefghij1234567890ABCDEFGHIJ12";
        let findings = redactor.scan(text);
        assert!(findings.iter().any(|f| f.pattern_name == "github_token"));
    }

    #[test]
    fn test_detect_database_url() {
        let redactor = SecretRedactor::new();
        let text = "DATABASE_URL=postgresql://admin:s3cret_p4ss@db.example.com:5432/mydb";
        let findings = redactor.scan(text);
        assert!(findings.iter().any(|f| f.pattern_name == "db_url"));
    }

    #[test]
    fn test_detect_email() {
        let redactor = SecretRedactor::new();
        let text = "Contact us at john.doe@company.com for more info.";
        let findings = redactor.scan(text);
        assert!(findings.iter().any(|f| f.pattern_name == "email"));
    }

    #[test]
    fn test_allowlist_skips_noreply() {
        let redactor = SecretRedactor::new();
        let text = "Email from noreply@github.com should not be flagged.";
        let findings = redactor.scan(text);
        // Should be empty or not contain the noreply email
        assert!(!findings.iter().any(|f| f.matched_text.contains("noreply@")));
    }

    #[test]
    fn test_allowlist_skips_private_ip() {
        let redactor = SecretRedactor::new();
        let text = "Server at 192.168.1.100 is fine. Also 10.0.0.1 and 8.8.8.8.";
        let findings = redactor.scan(text);
        assert!(!findings
            .iter()
            .any(|f| f.pattern_name == "ip_address" && f.matched_text.starts_with("192.168.")));
        assert!(!findings
            .iter()
            .any(|f| f.pattern_name == "ip_address" && f.matched_text.starts_with("10.")));
        assert!(!findings
            .iter()
            .any(|f| f.pattern_name == "ip_address" && f.matched_text == "8.8.8.8"));
    }

    #[test]
    fn test_redact_replaces_secrets() {
        let redactor = SecretRedactor::new();
        let text = "My key is sk-ant-api03-abcdefghijklmnopqrstuvwxyz1234567890 ok?";
        let (clean, count) = redactor.redact(text);
        assert!(count > 0);
        assert!(clean.contains(REDACTED));
        assert!(!clean.contains("sk-ant-api03"));
    }

    #[test]
    fn test_redact_no_secrets() {
        let redactor = SecretRedactor::new();
        let text = "This is just normal text with no secrets at all.";
        let (clean, count) = redactor.redact(text);
        assert_eq!(count, 0);
        assert_eq!(clean, text);
    }

    #[test]
    fn test_detect_private_key() {
        let redactor = SecretRedactor::new();
        let text = "-----BEGIN RSA PRIVATE KEY-----\nMIIBogIBAAJ...\n-----END RSA PRIVATE KEY-----";
        let findings = redactor.scan(text);
        assert!(findings.iter().any(|f| f.pattern_name == "private_key"));
    }

    #[test]
    fn test_detect_bearer_token() {
        let redactor = SecretRedactor::new();
        let text = "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";
        let findings = redactor.scan(text);
        assert!(
            findings.iter().any(|f| f.pattern_name == "bearer" || f.pattern_name == "jwt"),
            "Should detect bearer/jwt pattern"
        );
    }

    #[test]
    fn test_high_entropy_detection() {
        let redactor = SecretRedactor::new();
        // High entropy mixed-case string in quotes — should be flagged
        let text = r#"secret = "aB3dE5fG7hI9jK1lM3nO5pQ7rS9tU1vW3xY5zA7bC9dE1fG3h""#;
        let findings = redactor.scan(text);
        // At least env_secret or high_entropy should match
        assert!(!findings.is_empty());
    }

    #[test]
    fn test_shannon_entropy() {
        // Random-looking string should have high entropy
        assert!(shannon_entropy("aB3dE5fG7hI9jK1lM3") > 3.5);
        // Repetitive string should have low entropy
        assert!(shannon_entropy("aaaaaaa") < 1.0);
    }

    #[test]
    fn test_redact_custom_strings() {
        let redactor = SecretRedactor::new();
        let text = "The internal project codename is OperationPhoenix and our domain is secret-corp.internal.com";
        let (clean, count) = redactor.redact_custom(text, &["OperationPhoenix", "secret-corp.internal.com"]);
        assert!(count > 0);
        assert!(!clean.contains("OperationPhoenix"));
        assert!(clean.contains(REDACTED));
    }
}
