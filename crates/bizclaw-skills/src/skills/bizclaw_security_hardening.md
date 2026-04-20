---
name: bizclaw-security-hardening
description: |
  Security hardening specialist for BizClaw defense-in-depth. Trigger phrases:
  security hardening, vulnerability, OWASP, penetration test, secure coding,
  penetration testing, threat model, security audit, defense-in-depth.
  Scenarios: khi cần harden security, khi cần review vulnerability,
  khi cần implement security features, khi cần threat modeling.
version: 2.0.0
---

# BizClaw Security Hardening

You are a security specialist for the BizClaw AI platform. Focus on defense-in-depth.

## Security Modules (bizclaw-security crate)

### SecretRedactor
Regex-based detection for API keys, JWT, PII, emails, database URLs
- `REDACTOR.scan()` — detect secrets (warns, doesn't modify)
- `REDACTOR.redact()` — replace secrets with `[REDACTED]`
- LazyLock static singleton — thread-safe, zero-alloc after init

### InjectionScanner
Detects prompt injection, jailbreak, role override, data exfiltration
- Blocks dangerous patterns in user input
- Preserves legitimate use cases

### CommandAllowlist
Blocks shell injection (`;`, `|`, `$()`, backticks)
- Whitelist-based approach
- Explicit permission for safe commands

### ApprovalSystem
Human-in-the-loop for dangerous tool calls
- approve/deny/timeout workflow
- Configurable thresholds

### SecretStore
AES-256-CBC encrypted key-value storage
- Encrypted-at-rest secrets
- vault://key reference resolution

## WebSocket Pipeline Security

```
User Message → SecretRedactor.scan() [warn if secrets] → InjectionScanner.check() → Agent
Agent Response → SecretRedactor.redact() [remove leaked secrets] → User
```

### Panic Resilience
- `catch_unwind` wraps entire handler — panics don't crash the gateway
- Invalid JSON → graceful error response, connection stays alive

## Middleware Security Chain

1. **DanglingToolCallMiddleware** (priority 5) — fix state before anything
2. **GuardrailMiddleware** (priority 10) — block dangerous tool calls
3. **SummarizationMiddleware** (priority 30) — prevent context overflow
4. **MemoryMiddleware** (priority 80) — queue for safe fact extraction
5. **SubagentLimitMiddleware** (priority 90) — cap concurrent sub-agents

## Audit Checklist

### Configuration
- [ ] All API keys use `vault://` references, never plaintext in config
- [ ] Database credentials encrypted with SecretStore
- [ ] Environment variables properly set (not in code)

### Secrets Management
- [ ] SecretRedactor covers: OpenAI, Anthropic, GitHub, Bearer, JWT, SSH, DB URLs
- [ ] No secrets in logs (SecretRedactor.redact() before logging)
- [ ] Secrets rotated regularly

### Input Validation
- [ ] InjectionScanner blocks: system prompt leaks, role overrides, data exfil
- [ ] Shell commands pass through CommandAllowlist before execution
- [ ] File paths validated (no path traversal)

### WebSocket Security
- [ ] JWT authentication required
- [ ] WebSocket uses `catch_unwind` for panic resilience
- [ ] Rate limiting on connection attempts

### Network Security
- [ ] CORS properly configured — no wildcard in production
- [ ] Rate limiting on all API endpoints
- [ ] Nginx: HSTS, X-Frame-Options, CSP, no server version header

### Dependencies
- [ ] `cargo audit` passes (no known CVEs)
- [ ] `cargo deny check` passes (license compliance)
- [ ] Dependencies up-to-date

## Vulnerability Response

| Severity | Response Time |
|----------|---------------|
| Critical | 0-24 hours |
| High | 1-3 days |
| Medium | 1 week |
| Low | Next release |

## Common Vulnerabilities

### A01: Broken Access Control
```rust
// ✅ Good: Check ownership
fn get_resource(user_id: &str, resource_id: &str) -> Result<Resource> {
    let resource = db.find(resource_id)?;
    if resource.owner_id != user_id {
        return Err(Error::Forbidden);
    }
    Ok(resource)
}

// ❌ Bad: No ownership check
fn get_resource(resource_id: &str) -> Result<Resource> {
    db.find(resource_id)
}
```

### A02: Cryptographic Failures
```rust
// ✅ Good: Use proven encryption
let encrypted = Aes256Cbc::encrypt(&data, &key)?;

// ❌ Bad: Custom crypto
let encrypted = my_custom_xor(&data, &key)?; // Don't do this
```

### A03: Injection
```rust
// ✅ Good: Parameterized query
sqlx::query("SELECT * FROM users WHERE id = ?").bind(user_id);

// ❌ Bad: String concatenation
format!("SELECT * FROM users WHERE id = {}", user_id)
```

### A09: Logging Failures
```rust
// ✅ Good: Redact before logging
let redactor = SecretRedactor::new();
let (safe_msg, _) = redactor.redact(raw_message);
tracing::info!("message: {}", safe_msg);

// ❌ Bad: Log raw input
tracing::info!("message: {}", raw_message);
```

## Validation

```bash
#!/bin/bash
echo "=== Security Audit ==="

# Run cargo audit
cargo audit || { echo "❌ Vulnerable dependencies"; exit 1; }

# Check for secrets in code
git diff --cached | grep -iE "(api_key|password|secret|token)" && { echo "❌ Secrets detected"; exit 1; }

# Check for unsafe code without docs
grep -r "unsafe" --include="*.rs" crates/ | grep -v "// SAFETY:" && echo "⚠️ Unsafe code without safety docs"

# Check OWASP compliance
echo "✅ Security audit passed"
```
