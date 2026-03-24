# WebAuth Provider Implementation Guide for BizClaw

> **Reference Implementation**: [CrawBot feature/built-in-browser](https://github.com/Neurons-AI/crawbot/tree/feature/built-in-browser)
>
> **Date**: 2026-03-24
> **Status**: Implementation Plan — Ready for Development

---

## Table of Contents

1. [Overview](#overview)
2. [Architecture](#architecture)
3. [Phase 1: Text-Based Tool Call Translation Layer (Rust)](#phase-1-text-based-tool-call-translation-layer)
4. [Phase 2: WebAuth Proxy Server](#phase-2-webauth-proxy-server)
5. [Phase 3: Provider Implementations](#phase-3-provider-implementations)
6. [Phase 4: Pipeline Integration](#phase-4-pipeline-integration)
7. [Phase 5: Zalo Channel Integration](#phase-5-zalo-channel-integration)
8. [Key Pitfalls & Solutions](#key-pitfalls--solutions)
9. [Provider-Specific Notes](#provider-specific-notes)
10. [Debug & Testing Tools](#debug--testing-tools)
11. [Reference File Paths](#reference-file-paths)

---

## Overview

WebAuth providers allow BizClaw to use web-based AI models (Gemini, Claude, ChatGPT, DeepSeek, Grok, etc.) through their web interfaces instead of API keys. This is achieved by:

1. **Embedding a browser** (Electron WebContentsView) that maintains the user's login session
2. **Intercepting API requests** via CDP (Chrome DevTools Protocol) to capture request templates
3. **Replaying requests** with different prompts using the captured template
4. **Translating tool calls** from text-based format to OpenAI-compatible function calls

The critical innovation is **text-based tool calling**: web chat models don't support native `tools`/`function_calling` parameters, so we transform the system prompt to instruct the model to output structured JSON, then parse it back into OpenAI `tool_calls` format.

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│ BizClaw Agent Loop                                                  │
│                                                                     │
│  1. Agent prepares messages + tools                                 │
│  2. Calls provider.chat(messages, tools, params)                    │
│                                                                     │
│  ┌───────────────────────────────────────────────┐                  │
│  │ OpenAI-Compatible Provider (existing)          │                  │
│  │ • Sends to /v1/chat/completions               │                  │
│  │ • Gets back standard tool_calls               │                  │
│  └───────────────┬───────────────────────────────┘                  │
│                  │                                                   │
│                  ▼                                                   │
│  ┌───────────────────────────────────────────────┐                  │
│  │ WebAuth Proxy (localhost HTTP server)          │                  │
│  │ OpenAI-compatible /v1/chat/completions         │                  │
│  │                                                │                  │
│  │  ┌─────────────────────────────────────────┐  │                  │
│  │  │ Request Pipeline:                        │  │                  │
│  │  │ 1. Transform system prompt               │  │                  │
│  │  │    (native tools → text-based format)     │  │                  │
│  │  │ 2. Consolidate messages to single prompt  │  │                  │
│  │  │ 3. Route to web provider                  │  │                  │
│  │  └─────────────────────────────────────────┘  │                  │
│  │                                                │                  │
│  │  ┌─────────────────────────────────────────┐  │                  │
│  │  │ Response Pipeline:                       │  │                  │
│  │  │ 1. Parse text response                   │  │                  │
│  │  │ 2. Extract tool_call JSON blocks         │  │                  │
│  │  │ 3. Convert to OpenAI tool_calls format   │  │                  │
│  │  │ 4. Return as SSE stream                   │  │                  │
│  │  └─────────────────────────────────────────┘  │                  │
│  └───────────────┬───────────────────────────────┘                  │
│                  │                                                   │
│                  ▼                                                   │
│  ┌───────────────────────────────────────────────┐                  │
│  │ Web Provider (per-model implementation)        │                  │
│  │ • Gemini: Batchexecute API + CDP template capture                │
│  │ • Claude: Streaming API interception                             │
│  │ • ChatGPT: Conversation API proxy                                │
│  │ • DeepSeek: SSE chat API                                         │
│  │ • Grok: Twitter API proxy                                        │
│  └───────────────┬───────────────────────────────┘                  │
│                  │                                                   │
│                  ▼                                                   │
│  ┌───────────────────────────────────────────────┐                  │
│  │ Electron WebContentsView (logged-in browser)   │                  │
│  │ • Maintains session cookies                    │                  │
│  │ • Executes JS via executeJavaScript()          │                  │
│  │ • CDP access via debugger API                  │                  │
│  └───────────────────────────────────────────────┘                  │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Phase 1: Text-Based Tool Call Translation Layer

> **Goal**: Add text-based tool call support to the existing Rust provider, so models that can't use native `tools` can still execute function calls.

### 1.1 System Prompt Transformer

Transform the system prompt to replace native tool definitions with text-based instructions.

```rust
// crates/bizclaw-providers/src/text_tool_calls.rs

/// Transform system prompt for web chat models that don't support native tool calling.
///
/// Replaces the `## Tooling` section with text-based tool call instructions.
/// The model outputs JSON that the proxy parses back into OpenAI tool_calls format.
pub fn transform_system_prompt_for_webchat(system_text: &str, tools: &[ToolDefinition]) -> String {
    // Build tool list with descriptions
    let tool_list: Vec<String> = tools.iter().map(|t| {
        format!("  {}: {}", t.name, t.description)
    }).collect();

    let tool_names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();

    // Replace ## Tooling section with JSON action format
    let mut transformed = system_text.to_string();

    // Find and replace the tooling section
    if let Some(start) = transformed.find("## Tooling") {
        if let Some(end) = transformed[start..].find("\n## ").map(|i| start + i) {
            let replacement = format!(
r#"## Tool Use — MANDATORY
YOU MUST USE TOOLS. You are connected to a REAL system with FULL access.

⚠️ RULE: Tool call FIRST, talk LATER. Never answer without checking first.
⚠️ RULE: Never say "I can't access" — you CAN. Use the tool.

Format — output ONLY this JSON, nothing else:
{{"action": "function_call", "name": "TOOL", "arguments": {{"key": "value"}}}}

Multiple tools — one JSON per line:
{{"action": "function_call", "name": "read", "arguments": {{"path": "/a.txt"}}}}
{{"action": "function_call", "name": "exec", "arguments": {{"cmd": "ls"}}}}

Tools: {}

After tool result arrives, THEN respond to user. Plain text ONLY when no lookup needed.

"#,
                tool_names.join(" | ")
            );
            transformed.replace_range(start..end, &replacement);
        }
    }

    // Also remove ## Tool Call Style section if present
    if let Some(start) = transformed.find("## Tool Call Style") {
        if let Some(end) = transformed[start..].find("\n## ").map(|i| start + i) {
            transformed.replace_range(start..end, "");
        }
    }

    transformed
}
```

### 1.2 Response Parser — Extract Tool Calls from Text

```rust
// crates/bizclaw-providers/src/text_tool_calls.rs (continued)

use bizclaw_core::types::{ToolCall, FunctionCall};
use serde_json::Value;

/// Parsed tool call from model text output
#[derive(Debug)]
struct TextToolCall {
    name: String,
    params: Value,
    raw: String,
}

/// Parse text-based tool calls from model response.
///
/// Supports multiple formats:
/// 1. ```tool_call\n{...}\n``` code blocks
/// 2. Bare JSON with balanced-brace extraction
/// 3. Blockquote format: > {"action":"function_call",...}
pub fn parse_text_tool_calls(text: &str) -> Vec<ToolCall> {
    let mut calls = Vec::new();

    // Strategy 1: Match ```tool_call\n{...}\n``` blocks
    let code_block_calls = parse_code_block_tool_calls(text);
    if !code_block_calls.is_empty() {
        return code_block_calls.into_iter().enumerate().map(|(i, tc)| {
            ToolCall {
                id: format!("call_text_{}", i),
                r#type: "function".to_string(),
                function: FunctionCall {
                    name: tc.name,
                    arguments: serde_json::to_string(&tc.params).unwrap_or_default(),
                },
            }
        }).collect();
    }

    // Strategy 2: Blockquote format
    let blockquote_calls = parse_blockquote_tool_calls(text);
    if !blockquote_calls.is_empty() {
        return blockquote_calls.into_iter().enumerate().map(|(i, tc)| {
            ToolCall {
                id: format!("call_text_{}", i),
                r#type: "function".to_string(),
                function: FunctionCall {
                    name: tc.name,
                    arguments: serde_json::to_string(&tc.params).unwrap_or_default(),
                },
            }
        }).collect();
    }

    // Strategy 3: Balanced-brace JSON extraction
    let json_calls = parse_json_tool_calls(text);
    for (i, tc) in json_calls.into_iter().enumerate() {
        calls.push(ToolCall {
            id: format!("call_text_{}", i),
            r#type: "function".to_string(),
            function: FunctionCall {
                name: tc.name,
                arguments: serde_json::to_string(&tc.params).unwrap_or_default(),
            },
        });
    }

    calls
}

/// Parse ```tool_call code blocks
fn parse_code_block_tool_calls(text: &str) -> Vec<TextToolCall> {
    let mut calls = Vec::new();
    let mut remaining = text;

    while let Some(start) = remaining.find("```tool_call") {
        let after_marker = &remaining[start + 12..]; // skip "```tool_call"
        if let Some(end) = after_marker.find("```") {
            let content = after_marker[..end].trim();
            if let Ok(parsed) = serde_json::from_str::<Value>(content) {
                if let Some(name) = parsed.get("name").and_then(|v| v.as_str()) {
                    let params = parsed.get("params")
                        .or_else(|| parsed.get("arguments"))
                        .cloned()
                        .unwrap_or(Value::Object(serde_json::Map::new()));
                    calls.push(TextToolCall {
                        name: name.to_string(),
                        params,
                        raw: remaining[start..start + 12 + end + 3].to_string(),
                    });
                }
            }
            remaining = &after_marker[end + 3..];
        } else {
            break;
        }
    }

    calls
}

/// Parse blockquote format: > {"action":"function_call",...}
fn parse_blockquote_tool_calls(text: &str) -> Vec<TextToolCall> {
    let mut calls = Vec::new();

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("> ") {
            let json_part = trimmed[2..].trim();
            if json_part.starts_with('{') {
                if let Ok(parsed) = serde_json::from_str::<Value>(json_part) {
                    if parsed.get("action").and_then(|v| v.as_str()) == Some("function_call") {
                        if let Some(name) = parsed.get("name").and_then(|v| v.as_str()) {
                            let params = parsed.get("arguments")
                                .or_else(|| parsed.get("params"))
                                .cloned()
                                .unwrap_or(Value::Object(serde_json::Map::new()));
                            calls.push(TextToolCall {
                                name: name.to_string(),
                                params,
                                raw: line.to_string(),
                            });
                        }
                    }
                }
            }
        }
    }

    calls
}

/// Parse bare JSON with balanced-brace extraction.
///
/// Handles Gemini's output format where multiple JSON objects are
/// concatenated on one line with no separator.
fn parse_json_tool_calls(text: &str) -> Vec<TextToolCall> {
    let mut calls = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if chars[i] == '{' {
            let start = i;
            let mut depth = 0;

            while i < chars.len() {
                match chars[i] {
                    '{' => depth += 1,
                    '}' => {
                        depth -= 1;
                        if depth == 0 { break; }
                    }
                    _ => {}
                }
                i += 1;
            }

            let candidate: String = chars[start..=i.min(chars.len() - 1)].iter().collect();

            if let Ok(parsed) = serde_json::from_str::<Value>(&candidate) {
                // Format 1: {"action": "function_call", "name": "...", "arguments": {...}}
                if parsed.get("action").and_then(|v| v.as_str()) == Some("function_call") {
                    if let Some(name) = parsed.get("name").and_then(|v| v.as_str()) {
                        let params = parsed.get("arguments")
                            .or_else(|| parsed.get("params"))
                            .cloned()
                            .unwrap_or(Value::Object(serde_json::Map::new()));
                        calls.push(TextToolCall {
                            name: name.to_string(),
                            params,
                            raw: candidate,
                        });
                    }
                }
                // Format 2: {"function": "...", "params": {...}}
                else if let Some(name) = parsed.get("function").and_then(|v| v.as_str()) {
                    let params = parsed.get("params")
                        .or_else(|| parsed.get("arguments"))
                        .cloned()
                        .unwrap_or(Value::Object(serde_json::Map::new()));
                    calls.push(TextToolCall {
                        name: name.to_string(),
                        params,
                        raw: candidate,
                    });
                }
            }
        }
        i += 1;
    }

    calls
}

/// Consolidate messages array into a single text prompt.
///
/// Web chat providers only accept a single text input, so we flatten
/// the entire conversation context into one coherent message.
///
/// Uses `<system_instruction>` tag (NOT `<system>` — Gemini rejects it).
pub fn consolidate_messages(messages: &[Message], system_transform: Option<&dyn Fn(&str) -> String>) -> String {
    let mut parts = Vec::new();

    for msg in messages {
        let text = if msg.role == Role::System {
            if let Some(transform) = system_transform {
                transform(&msg.content)
            } else {
                msg.content.clone()
            }
        } else {
            msg.content.clone()
        };

        if text.trim().is_empty() {
            continue;
        }

        match msg.role {
            Role::System => parts.push(format!("<system_instruction>\n{}\n</system_instruction>", text)),
            Role::User => parts.push(format!("<user>\n{}\n</user>", text)),
            Role::Assistant => parts.push(format!("<assistant>\n{}\n</assistant>", text)),
            Role::Tool => parts.push(format!("<tool_result>\n{}\n</tool_result>", text)),
        }
    }

    // If only one user message and no context, send raw text
    let has_system = messages.iter().any(|m| m.role == Role::System);
    let has_assistant = messages.iter().any(|m| m.role == Role::Assistant);
    let user_msgs: Vec<&Message> = messages.iter().filter(|m| m.role == Role::User).collect();

    if !has_system && !has_assistant && user_msgs.len() == 1 {
        return user_msgs[0].content.clone();
    }

    parts.join("\n\n")
}

/// Strip text-based tool call blocks from response, returning clean text content
pub fn strip_tool_call_text(text: &str, tool_calls: &[ToolCall]) -> String {
    // Simple approach: remove JSON objects that match function_call pattern
    let mut result = text.to_string();

    // Remove ```tool_call blocks
    while let Some(start) = result.find("```tool_call") {
        if let Some(end) = result[start + 12..].find("```") {
            result.replace_range(start..start + 12 + end + 3, "");
        } else {
            break;
        }
    }

    // Remove blockquote tool calls
    let lines: Vec<&str> = result.lines().filter(|line| {
        let trimmed = line.trim();
        if trimmed.starts_with("> ") {
            let json_part = trimmed[2..].trim();
            if let Ok(parsed) = serde_json::from_str::<Value>(json_part) {
                if parsed.get("action").and_then(|v| v.as_str()) == Some("function_call") {
                    return false; // Remove this line
                }
            }
        }
        true
    }).collect();
    result = lines.join("\n");

    // Remove bare JSON tool calls (balanced brace)
    // This is trickier — for now, remove known patterns
    for tc in tool_calls {
        // Try to find the raw JSON in the text
        let pattern = format!("\"name\":\"{}\"", tc.function.name);
        // Use a simple heuristic: find JSON containing this tool name
        let chars: Vec<char> = result.chars().collect();
        let pattern_chars: Vec<char> = pattern.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            if chars[i] == '{' {
                let start = i;
                let mut depth = 0;
                while i < chars.len() {
                    match chars[i] {
                        '{' => depth += 1,
                        '}' => { depth -= 1; if depth == 0 { break; } }
                        _ => {}
                    }
                    i += 1;
                }
                let candidate: String = chars[start..=i.min(chars.len() - 1)].iter().collect();
                if candidate.contains(&pattern) {
                    result = result.replacen(&candidate, "", 1);
                    break;
                }
            }
            i += 1;
        }
    }

    result.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_bare_json_tool_calls() {
        let text = r#"{"action":"function_call","name":"read","arguments":{"path":"/etc/hostname"}}{"action":"function_call","name":"exec","arguments":{"command":"ls"}}"#;
        let calls = parse_text_tool_calls(text);
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].function.name, "read");
        assert_eq!(calls[1].function.name, "exec");
    }

    #[test]
    fn test_parse_blockquote_tool_calls() {
        let text = r#"> {"action": "function_call", "name": "read", "arguments": {"path": "/etc/hostname"}}
> {"action": "function_call", "name": "exec", "arguments": {"command": "ls"}}"#;
        let calls = parse_text_tool_calls(text);
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].function.name, "read");
        assert_eq!(calls[1].function.name, "exec");
    }

    #[test]
    fn test_parse_code_block_tool_calls() {
        let text = "Sure, let me check that:\n```tool_call\n{\"name\": \"read\", \"params\": {\"path\": \"/etc/hosts\"}}\n```\n";
        let calls = parse_text_tool_calls(text);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].function.name, "read");
    }

    #[test]
    fn test_no_tool_calls_in_regular_text() {
        let text = "Hello! I can help you with that. Let me know what you need.";
        let calls = parse_text_tool_calls(text);
        assert_eq!(calls.len(), 0);
    }

    #[test]
    fn test_consolidate_system_instruction_tag() {
        // Verify we use <system_instruction> not <system>
        let messages = vec![
            Message::system("You are a helpful assistant."),
        ];
        let result = consolidate_messages(&messages, None);
        assert!(result.contains("<system_instruction>"));
        assert!(!result.contains("<system>") || result.contains("<system_instruction>"));
    }
}
```

### 1.3 Integration into OpenAI-Compatible Provider

```rust
// In crates/bizclaw-providers/src/openai_compatible.rs
// Add after parsing response, BEFORE returning ProviderResponse:

// === TEXT-BASED TOOL CALL FALLBACK ===
// If model returned no native tool_calls but content contains structured
// text-based tool calls (from webauth or text-prompted models), parse them.
if tool_calls.is_empty() && !tools.is_empty() {
    if let Some(ref text) = content {
        let text_calls = crate::text_tool_calls::parse_text_tool_calls(text);
        if !text_calls.is_empty() {
            tracing::info!(
                "🔧 Parsed {} text-based tool call(s) from response",
                text_calls.len()
            );
            // Strip tool call text from content
            let clean_text = crate::text_tool_calls::strip_tool_call_text(text, &text_calls);
            return Ok(ProviderResponse {
                content: if clean_text.is_empty() { None } else { Some(clean_text) },
                tool_calls: text_calls,
                finish_reason: Some("tool_calls".into()),
                usage,
            });
        }
    }
}
```

---

## Phase 2: WebAuth Proxy Server

> **Goal**: Create a localhost HTTP server that exposes OpenAI-compatible endpoints, routing to web providers.

### 2.1 Proxy Architecture

The WebAuth Proxy runs as a local HTTP server (`127.0.0.1:PORT`) that:
- Accepts OpenAI-compatible requests at `/v1/chat/completions`
- Lists available models at `/v1/models`
- Routes requests to the correct provider based on model prefix (e.g., `webauth-gemini-pro`)
- Streams responses back as Server-Sent Events (SSE)

**BizClaw Integration**: Configure the proxy as a `custom:http://127.0.0.1:PORT/v1` provider in the config. The existing `OpenAiCompatibleProvider` handles it seamlessly.

### 2.2 Key Design: Request Pipeline

```
Incoming OpenAI Request
    │
    ├── Extract model name → find provider
    ├── Transform system prompt (tools → text instructions)
    ├── Consolidate messages (multi-turn → single prompt)
    ├── Provider.chatCompletion(webview, request)
    │    │
    │    ├── Gemini: f.req batchexecute replay
    │    ├── Claude: /api/chat streaming
    │    ├── ChatGPT: /backend-api/conversation
    │    └── etc.
    │
    ├── Parse response text for tool call JSON
    ├── Convert to OpenAI SSE chunks
    └── Stream back to caller
```

### 2.3 Key Design: Response Pipeline

```
Model Response Text
    │
    ├── parseTextToolCalls(text)
    │    ├── Strategy 1: ```tool_call blocks
    │    ├── Strategy 2: > blockquote JSON (ChatGPT)
    │    └── Strategy 3: Balanced-brace JSON (Gemini)
    │
    ├── If tool calls found:
    │    ├── Strip tool call text from content
    │    ├── Emit text content chunk (if any)
    │    ├── Emit tool_calls chunk with finish_reason: "tool_calls"
    │    └── Each tool call: {index, id, type: "function", function: {name, arguments}}
    │
    └── If no tool calls:
         └── Emit regular text chunk with finish_reason: "stop"
```

---

## Phase 3: Provider Implementations

### 3.1 Gemini Web Provider (Reference Implementation)

**How it works:**

1. **Template Capture** via CDP `Fetch.requestPaused`:
   - Enable CDP Fetch interception for `*StreamGenerate*` URLs
   - Navigate to `/app` and type "Hello" in the input
   - Click send → CDP intercepts the real HTTP request
   - Parse `f.req` body to extract the inner JSON template
   - Extract `at` token (CSRF protection)
   - `Fetch.continueRequest` to let the original request complete
   - Disable `Fetch.enable` to stop intercepting

2. **Request Replay**:
   - Clone captured template
   - Set `template[0][0] = new_message` (the prompt)
   - Reset `template[2]` (conversation metadata)
   - Build `f.req=<JSON>&at=<token>&` POST body
   - Execute via `webview.executeJavaScript(fetch(url, {...}))` with `credentials: 'include'`

3. **Response Parsing**:
   - Response is multiline, each line is a JSON array
   - Find items where `item[0] === 'wrb.fr'`
   - Parse `item[2]` as JSON → response text at `data[4][0][1]`
   - `data[4][0][1]` is either a string or string array  — join with ''

4. **Tool Call Extraction**:
   - Feed response text to `parseTextToolCalls()`
   - Convert to OpenAI `tool_calls` format
   - Emit as SSE chunk

```rust
// Pseudo-code for Gemini template capture (Rust/headless equivalent)
struct GeminiTemplate {
    inner: Vec<Value>,
    at_token: String,
    url: String,
}

async fn capture_template(webview: &WebView) -> Option<GeminiTemplate> {
    // 1. Enable CDP Fetch interception
    webview.send_cdp("Fetch.enable", json!({
        "patterns": [{"urlPattern": "*StreamGenerate*", "requestStage": "Request"}]
    })).await;

    // 2. Navigate to /app
    webview.execute_js("window.location.href = 'https://gemini.google.com/app'").await;

    // 3. Wait for input to appear
    webview.wait_for_selector("textarea, [contenteditable]").await;

    // 4. Type and send "Hello"
    webview.execute_js(r#"
        const input = document.querySelector('textarea');
        input.value = 'Hello';
        input.dispatchEvent(new Event('input', { bubbles: true }));
        setTimeout(() => {
            const btn = document.querySelector('button[aria-label="Send message"]');
            if (btn && !btn.disabled) btn.click();
        }, 1000);
    "#).await;

    // 5. Wait for Fetch.requestPaused event
    let paused = webview.wait_for_cdp_event("Fetch.requestPaused", 15000).await;

    // 6. Extract template from postData
    let post_data = paused["request"]["postData"].as_str()?;
    let decoded = urlencoding::decode(post_data).ok()?;
    let freq_match = regex::Regex::new(r"f\.req=([\s\S]+?)&at=").ok()?.captures(&decoded)?;
    let at_match = regex::Regex::new(r"&at=([^&]+)").ok()?.captures(&decoded)?;

    let outer: Vec<Value> = serde_json::from_str(freq_match.get(1)?.as_str()).ok()?;
    let inner: Vec<Value> = serde_json::from_str(outer[1].as_str()?).ok()?;

    // 7. Continue the paused request
    webview.send_cdp("Fetch.continueRequest", json!({
        "requestId": paused["requestId"]
    })).await;
    webview.send_cdp("Fetch.disable", json!({})).await;

    Some(GeminiTemplate {
        inner,
        at_token: at_match.get(1)?.as_str().to_string(),
        url: paused["request"]["url"].as_str()?.to_string(),
    })
}
```

### 3.2 Claude Web Provider

Claude's web API uses a streaming conversation API. Key differences:
- Intercept POST to `https://claude.ai/api/chat`
- Capture organization ID and conversation ID from page context  
- Claude rejects `<system>` tags — use `<system_instruction>` or just embed naturally
- Response is SSE with `event: completion` chunks

### 3.3 ChatGPT Web Provider

ChatGPT requires a completely different prompt strategy:
- GPT-5.4/4o has a code sandbox and will try to execute commands there
- Use "Two Environments" framing to redirect tool calls
- Blockquote `>` format for tool call output
- Response from `/backend-api/conversation` SSE stream

### 3.4 Model ID Conventions

```
webauth-gemini-pro     → Gemini Web → Pro model
webauth-gemini-flash   → Gemini Web → Flash model
webauth-claude-sonnet  → Claude Web → Sonnet model
webauth-chatgpt-4o     → ChatGPT Web → GPT-4o model
webauth-deepseek-chat  → DeepSeek Web → V3 model
webauth-grok-3         → Grok Web → Grok 3 model
```

---

## Phase 4: Pipeline Integration

### 4.1 Configuration (config.toml)

```toml
[LLM]
provider = "custom"
endpoint = "http://127.0.0.1:${WEBAUTH_PORT}/v1"
model = "webauth-gemini-pro"
```

Or dynamically, the WebAuth pipeline can update the config at runtime when authenticated providers are detected.

### 4.2 Health Check Flow

```
Every 5 minutes:
  1. For each provider:
     a. Check cookies in Electron session
     b. Call provider.checkAuth(webview)
     c. If expired: remove webview, mark as unavailable
     d. If valid: ensure webview exists
  2. Update model list (/v1/models)
  3. Notify UI of status changes
```

### 4.3 BizClaw Integration Points

| Component | File | What to modify |
|-----------|------|---------------|
| Provider registry | `crates/bizclaw-providers/src/lib.rs` | Add `text_tool_calls` module |
| Text tool parser | `crates/bizclaw-providers/src/text_tool_calls.rs` | New file |
| OpenAI provider | `crates/bizclaw-providers/src/openai_compatible.rs` | Add text-based fallback |
| Config | `config.quickstart.toml` | Add webauth example |
| Agent engine | `crates/bizclaw-agent/src/lib.rs` | No changes needed (uses ProviderResponse) |

---

## Phase 5: Zalo Channel Integration

> **Reference**: `electron/utils/zalouser-login.ts` in CrawBot

### 5.1 How Zalo Login Works (CrawBot Pattern)

CrawBot uses `zca-js` library for Zalo Personal login:

```
1. Start QR login session via zca-js
2. Display QR code image (base64 data URL) to user
3. User scans QR with Zalo app on phone
4. Receive callbacks: QRGenerated → QRScanned → GotLoginInfo
5. Save credentials (imei, cookie, userAgent) to disk
6. Use credentials for subsequent API calls
```

### 5.2 Key Zalo Credential Structure

```json
{
  "imei": "device-id-from-login",
  "cookie": { /* zalo session cookie object */ },
  "userAgent": "browser-user-agent-string",
  "createdAt": "2026-03-24T12:00:00.000Z",
  "lastUsedAt": "2026-03-24T12:00:00.000Z"
}
```

### 5.3 BizClaw Zalo Integration

```
crates/bizclaw-channels/
├── src/
│   ├── zalo/
│   │   ├── mod.rs          # Zalo channel implementation
│   │   ├── auth.rs         # QR login + credential management
│   │   ├── api.rs          # Zalo API wrapper (send/receive messages)
│   │   └── types.rs        # Zalo-specific types
```

---

## Key Pitfalls & Solutions

### Pitfall 1: Gemini rejects `<system>` tag
**Symptom**: Gemini returns 400 or ignores system prompt  
**Solution**: Use `<system_instruction>` instead of `<system>` tag  
**CrawBot ref**: `shared-utils.ts` line in `consolidateMessages()`

### Pitfall 2: Multiple JSON objects on one line (no separator)
**Symptom**: Only first tool call parsed  
**Solution**: Balanced-brace extraction (walk char by char), NOT regex  
**CrawBot ref**: `parseJsonToolCalls()` in `shared-utils.ts`

### Pitfall 3: Model refuses tool calls without persona context
**Symptom**: Model says "I can't access files"  
**Solution**: System prompt MUST include persona context ("You are X running inside Y on real system Z"). The `transformSystemPrompt` only replaces `## Tooling` — preserves all persona context  
**CrawBot ref**: `webchat-tool-use-prompt-research.md` Key Lesson #2

### Pitfall 4: ChatGPT uses its sandbox instead of tool calls
**Symptom**: Model executes Python code instead of outputting tool JSON  
**Solution**: "Two Environments" framing — explain sandbox ≠ host, blockquote format  
**CrawBot ref**: `transformSystemPromptForChatGPT()` in `shared-utils.ts`

### Pitfall 5: CDP template capture hangs
**Symptom**: `Fetch.requestPaused` never fires  
**Solution**: Ensure Fetch is enabled BEFORE triggering the request. Verify input is focused and send button is not disabled. Use 15s timeout.  
**CrawBot ref**: `captureTemplate()` in `gemini-web.ts`

### Pitfall 6: Account switch invalidates template
**Symptom**: 401/400 errors after user switches Google account  
**Solution**: Track page URL, detect `/u/N/` prefix change, invalidate `cachedTemplate`  
**CrawBot ref**: `apiChat()` account change detection in `gemini-web.ts`

### Pitfall 7: Electron throttles hidden WebContentsView
**Symptom**: `executeJavaScript()` hangs for hidden views  
**Solution**: Set `webContents.setBackgroundThrottling(false)`, position off-screen at `(-9999, -9999)` instead of `setVisible(false)`  
**CrawBot ref**: `ensureProviderWebview()` in `webauth-pipeline.ts`

### Pitfall 8: Gemini response format — data[4][0][1]
**Symptom**: Can't find response text in batchexecute output  
**Solution**: Parse multiline output, find `wrb.fr` array items, parse `item[2]` as JSON, text is at `data[4][0][1]` (can be string OR string array)  
**CrawBot ref**: Response parsing in `apiChat()` fetch callback

### Pitfall 9: Parser cross-contamination between providers
**Symptom**: Fixing Gemini parser breaks ChatGPT parsing  
**Solution**: Each provider calls its OWN parser function. Don't share a single parse pipeline.  
**CrawBot ref**: `webchat-tool-use-prompt-research.md` Key Lesson #6

---

## Provider-Specific Notes

### Gemini Web
- **API**: Batchexecute StreamGenerate (`POST /_/BardChatUi/data/assistant.lamda.BardFrontendService/StreamGenerate`)
- **Auth**: Cookie `__Secure-1PSID`
- **Template**: Captured via CDP Fetch interception
- **CSRF**: `at` token extracted from POST body
- **Response**: Multiline JSON arrays, text at `data[4][0][1]`
- **Models**: Pro, Flash (same endpoint, different behavior)
- **Tool calling**: Bare JSON output, balanced-brace parser
- **Images**: Inject at `template[0][4]` as `[[base64, mediaType], null]`

### Claude Web  
- **API**: `POST https://claude.ai/api/chat`
- **Auth**: Cookie-based session  
- **Response**: SSE stream with `event: completion` 
- **Tool calling**: Currently limited — Claude web guardrails are strict
- **Note**: Claude refuses most tool call prompts via web chat (unlike API)

### ChatGPT Web
- **API**: `POST https://chatgpt.com/backend-api/conversation`
- **Auth**: Cookie + auth token
- **Response**: SSE stream
- **Tool calling**: "Two Environments" prompt + blockquote `>` format
- **Key challenge**: Model has a real code sandbox that interferes
- **Models**: GPT-5.4 Thinking, GPT-4o, GPT-4o-mini

### DeepSeek Web
- **API**: `POST https://chat.deepseek.com/api/v0/chat/submit`
- **Auth**: Cookie-based session
- **Response**: SSE stream
- **Tool calling**: JSON action format (same as Gemini)

### Grok Web
- **API**: Twitter/X conversation API
- **Auth**: X.com session cookies
- **Response**: SSE stream
- **Tool calling**: JSON action format

---

## Debug & Testing Tools

### Quick Test: Text Tool Call Parser

```rust
#[test]
fn test_gemini_style_output() {
    let text = r#"Let me check that for you.
{"action":"function_call","name":"shell","arguments":{"command":"ls -la /tmp"}}
I'll review the results and get back to you."#;

    let calls = parse_text_tool_calls(text);
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].function.name, "shell");

    let args: Value = serde_json::from_str(&calls[0].function.arguments).unwrap();
    assert_eq!(args["command"], "ls -la /tmp");
}

#[test]
fn test_chatgpt_blockquote_output() {
    let text = r#"I'll read those files for you:

> {"action": "function_call", "name": "read", "arguments": {"path": "/workspace/SOUL.md"}}
> {"action": "function_call", "name": "read", "arguments": {"path": "/workspace/USER.md"}}
> {"action": "function_call", "name": "read", "arguments": {"path": "/workspace/MEMORY.md"}}

Let me wait for the results."#;

    let calls = parse_text_tool_calls(text);
    assert_eq!(calls.len(), 3);
    assert_eq!(calls[0].function.name, "read");
    assert_eq!(calls[1].function.name, "read");
    assert_eq!(calls[2].function.name, "read");
}
```

### Automated Testing Pattern

```bash
# Start WebAuth proxy in test mode
BIZCLAW_WEBAUTH_PORT=0 cargo run -- --webauth-test

# Test with curl
curl http://127.0.0.1:PORT/v1/models | jq

curl http://127.0.0.1:PORT/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "webauth-gemini-pro",
    "messages": [{"role": "user", "content": "Hello"}],
    "stream": true
  }'
```

### CDP Debug Commands

```javascript
// Check if cookie exists (Gemini)
document.cookie.split(';').some(c => c.trim().startsWith('__Secure-1PSID='))

// Force template recapture
provider.cachedTemplate = null;

// Check if input is ready
!!document.querySelector('textarea, [contenteditable="true"]')

// Send button state
document.querySelector('button[aria-label="Send message"]')?.disabled
```

---

## Reference File Paths

### CrawBot (feature/built-in-browser)

| File | Purpose |
|------|---------|
| `electron/browser/providers/shared-utils.ts` | 🔑 System prompt transform + tool call parsers |
| `electron/browser/providers/gemini-web.ts` | Gemini provider: template capture + API replay |
| `electron/browser/providers/chatgpt-web.ts` | ChatGPT provider: two environments strategy |
| `electron/browser/providers/claude-web.ts` | Claude provider: streaming API |
| `electron/browser/providers/deepseek-web.ts` | DeepSeek provider |
| `electron/browser/providers/grok-web.ts` | Grok provider via X.com |
| `electron/browser/providers/types.ts` | WebProvider interface + types |
| `electron/browser/providers/base-provider.ts` | Base class with common utilities |
| `electron/browser/providers/wcv-adapter.ts` | WebContentsView → WebviewLike adapter |
| `electron/browser/providers/cookie-auth-checker.ts` | Cookie-based auth checking |
| `electron/browser/webauth-proxy.ts` | OpenAI-compatible HTTP proxy server |
| `electron/browser/webauth-pipeline.ts` | Pipeline orchestrator |
| `electron/browser/webauth-views.ts` | Browser tab management |
| `electron/utils/zalouser-login.ts` | Zalo QR login via zca-js |
| `docs/webchat-tool-use-prompt-research.md` | Research: 30+ prompt variants tested |
| `docs/webauth-provider-implementation-guide.md` | Original implementation guide |

### BizClaw (this repo)

| File | Purpose |
|------|---------|
| `crates/bizclaw-providers/src/text_tool_calls.rs` | 🆕 Text-based tool call translation |
| `crates/bizclaw-providers/src/openai_compatible.rs` | Existing OpenAI provider (add fallback) |
| `crates/bizclaw-providers/src/lib.rs` | Module registration |
| `crates/bizclaw-providers/src/provider_registry.rs` | Provider configs |
| `crates/bizclaw-core/src/types/tool_call.rs` | ToolCall, FunctionCall types |
| `crates/bizclaw-core/src/types/message.rs` | Message, ProviderResponse types |
| `crates/bizclaw-agent/src/lib.rs` | Agent loop (uses ProviderResponse) |
| `crates/bizclaw-channels/src/` | Channel implementations (add Zalo) |

---

## Implementation Priority

1. **Phase 1** (Immediate): `text_tool_calls.rs` — Pure Rust, no external deps, testable
2. **Phase 2** (Next): WebAuth Proxy — Needs Electron/browser integration  
3. **Phase 3** (Parallel): Gemini provider first (best documented), then others
4. **Phase 4** (Integration): Config + health checks
5. **Phase 5** (Later): Zalo channel (independent of WebAuth)

---

> **Note**: The CrawBot implementation is in TypeScript (Electron). BizClaw's equivalent needs to be in Rust for the core logic, with Electron/Tauri for the browser embedding if needed. The text-based tool call translation layer (Phase 1) works purely in Rust and can be used immediately with any OpenAI-compatible proxy.
