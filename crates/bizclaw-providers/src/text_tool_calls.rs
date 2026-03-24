//! Text-based tool call translation for WebAuth providers.
//!
//! Web chat models (Gemini, ChatGPT, Claude, etc.) don't support native
//! OpenAI `tools`/`function_calling`. This module provides:
//!
//! 1. **System prompt transformation**: Replace native tool definitions with
//!    text-based instructions that web chat guardrails accept.
//! 2. **Response parsing**: Extract tool call JSON from model text output.
//! 3. **Format conversion**: Convert text-based tool calls to OpenAI format.
//!
//! # Supported Output Formats
//!
//! - Bare JSON: `{"action":"function_call","name":"read","arguments":{"path":"/a.txt"}}`
//! - Code blocks: ````tool_call\n{...}\n````
//! - Blockquote: `> {"action":"function_call",...}`
//!
//! # Key Design Decisions
//!
//! - Uses `<system_instruction>` tag (NOT `<system>` — Gemini rejects it)
//! - Balanced-brace JSON extraction (NOT regex — handles nested JSON)
//! - Each provider should use its own parser to avoid cross-contamination

use bizclaw_core::types::{FunctionCall, Message, Role, ToolCall, ToolDefinition};
use serde_json::Value;

// ─── System Prompt Transformation ──────────────────────────────────────────

/// Transform system prompt for WebAuth models that don't support native tool calling.
///
/// Replaces the `## Tooling` section with text-based tool calling instructions.
/// The model outputs JSON that the proxy parses back into OpenAI `tool_calls` format.
///
/// **Critical**: This function ONLY replaces `## Tooling` — preserving all persona
/// context. Without persona context, models refuse tool calls (they say "I can't
/// access files").
pub fn transform_system_prompt_for_webchat(
    system_text: &str,
    tools: &[ToolDefinition],
) -> String {
    let tool_names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();

    let mut transformed = system_text.to_string();

    // Replace ## Tooling section with JSON action format
    if let Some(start) = find_section_start(&transformed, "## Tooling") {
        if let Some(end) = find_next_h2(&transformed, start) {
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
    if let Some(start) = find_section_start(&transformed, "## Tool Call Style") {
        if let Some(end) = find_next_h2(&transformed, start) {
            transformed.replace_range(start..end, "");
        }
    }

    // Also remove ## Tool availability section if present
    if let Some(start) = find_section_start(&transformed, "## Tool availability") {
        if let Some(end) = find_next_h2(&transformed, start) {
            // Keep tool list for reference but simplify
            transformed.replace_range(start..end, "");
        }
    }

    transformed
}

/// Transform system prompt specifically for ChatGPT web chat.
///
/// ChatGPT's Thinking model has a real code sandbox. It tries to execute
/// commands there instead of outputting tool calls. The "Two Environments"
/// framing explains that the sandbox ≠ host system and gives concrete examples.
pub fn transform_system_prompt_for_chatgpt(
    system_text: &str,
    _tools: &[ToolDefinition],
    workspace: &str,
) -> String {
    let mut transformed = system_text.to_string();

    if let Some(start) = find_section_start(&transformed, "## Tooling") {
        if let Some(end) = find_next_h2(&transformed, start) {
            let replacement = format!(
                r#"## System Access (IMPORTANT — read carefully)
You have TWO environments:
1. ❌ Code sandbox — an isolated container. It does NOT have the user's files.
2. ✅ Host system — the user's real machine. Accessed ONLY through blockquote requests.

To access the user's files, output blockquote JSON:
> {{"action": "function_call", "name": "read", "arguments": {{"path": "/path/to/file"}}}}

> {{"action": "function_call", "name": "exec", "arguments": {{"command": "shell command"}}}}

> {{"action": "function_call", "name": "write", "arguments": {{"path": "/path", "content": "data"}}}}

Example — Session Startup (reading workspace files):
> {{"action": "function_call", "name": "read", "arguments": {{"path": "{workspace}/SOUL.md"}}}}
> {{"action": "function_call", "name": "read", "arguments": {{"path": "{workspace}/USER.md"}}}}
> {{"action": "function_call", "name": "read", "arguments": {{"path": "{workspace}/MEMORY.md"}}}}

Output ALL blockquote requests FIRST. Wait for results. Then respond to the user.
❌ NEVER use your code sandbox to read files — those are NOT the user's files.

"#
            );
            transformed.replace_range(start..end, &replacement);
        }
    }

    // Remove ## Tool Call Style section
    if let Some(start) = find_section_start(&transformed, "## Tool Call Style") {
        if let Some(end) = find_next_h2(&transformed, start) {
            transformed.replace_range(start..end, "");
        }
    }

    transformed
}

// ─── Message Consolidation ─────────────────────────────────────────────────

/// Consolidate messages array into a single text prompt.
///
/// Web chat providers only accept a single text input, so we flatten
/// the entire conversation context into one coherent message.
///
/// Uses `<system_instruction>` tag (NOT `<system>` — Gemini rejects it).
pub fn consolidate_messages(
    messages: &[Message],
    tools: &[ToolDefinition],
    provider_type: ProviderType,
) -> String {
    let mut parts = Vec::new();

    for msg in messages {
        if msg.content.trim().is_empty() {
            continue;
        }

        let text = if msg.role == Role::System {
            match provider_type {
                ProviderType::ChatGPT => {
                    transform_system_prompt_for_chatgpt(&msg.content, tools, "/workspace")
                }
                _ => transform_system_prompt_for_webchat(&msg.content, tools),
            }
        } else {
            msg.content.clone()
        };

        if text.trim().is_empty() {
            continue;
        }

        match msg.role {
            Role::System => {
                parts.push(format!(
                    "<system_instruction>\n{}\n</system_instruction>",
                    text
                ))
            }
            Role::User => parts.push(format!("<user>\n{}\n</user>", text)),
            Role::Assistant => parts.push(format!("<assistant>\n{}\n</assistant>", text)),
            Role::Tool => parts.push(format!("<tool_result>\n{}\n</tool_result>", text)),
        }
    }

    // If only one user message and no system/assistant context,
    // just send the raw text (simpler for the model)
    let has_system = messages.iter().any(|m| m.role == Role::System);
    let has_assistant = messages.iter().any(|m| m.role == Role::Assistant);
    let user_msgs: Vec<&Message> = messages.iter().filter(|m| m.role == Role::User).collect();

    if !has_system && !has_assistant && user_msgs.len() == 1 {
        return user_msgs[0].content.clone();
    }

    parts.join("\n\n")
}

/// Provider type for selecting the appropriate prompt strategy.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ProviderType {
    /// Gemini web — uses "MANDATORY" approach with bare JSON output
    Gemini,
    /// ChatGPT web — uses "Two Environments" + blockquote format
    ChatGPT,
    /// Claude web — limited tool call support
    Claude,
    /// DeepSeek web — similar to Gemini approach
    DeepSeek,
    /// Grok web — similar to Gemini approach
    Grok,
    /// Generic/default — uses standard JSON action format
    Generic,
}

// ─── Tool Call Parsing ─────────────────────────────────────────────────────

/// Parse text-based tool calls from model response text.
///
/// Dispatches to multiple strategies in order of specificity:
/// 1. ````tool_call` code blocks (most explicit)
/// 2. Blockquote `>` format (ChatGPT style)
/// 3. Balanced-brace JSON extraction (Gemini style)
///
/// Returns OpenAI-format `ToolCall` structs ready for the agent loop.
pub fn parse_text_tool_calls(text: &str) -> Vec<ToolCall> {
    // Strategy 1: ```tool_call code blocks
    let code_calls = parse_code_block_tool_calls(text);
    if !code_calls.is_empty() {
        return to_openai_tool_calls(code_calls);
    }

    // Strategy 2: Blockquote format
    let quote_calls = parse_blockquote_tool_calls(text);
    if !quote_calls.is_empty() {
        return to_openai_tool_calls(quote_calls);
    }

    // Strategy 3: Balanced-brace JSON extraction
    let json_calls = parse_json_tool_calls(text);
    to_openai_tool_calls(json_calls)
}

/// Parse tool calls specifically for Gemini-like providers (bare JSON).
pub fn parse_gemini_tool_calls(text: &str) -> Vec<ToolCall> {
    to_openai_tool_calls(parse_json_tool_calls(text))
}

/// Parse tool calls specifically for ChatGPT-like providers (blockquote format).
pub fn parse_chatgpt_tool_calls(text: &str) -> Vec<ToolCall> {
    let quote_calls = parse_blockquote_tool_calls(text);
    if !quote_calls.is_empty() {
        return to_openai_tool_calls(quote_calls);
    }
    // Fallback to JSON extraction
    to_openai_tool_calls(parse_json_tool_calls(text))
}

// ─── Internal Parser Implementations ───────────────────────────────────────

/// Internal parsed tool call representation.
#[derive(Debug)]
struct TextToolCall {
    name: String,
    params: Value,
    #[allow(dead_code)] // Kept for debug logging and future provider-specific parsers
    raw: String,
}

/// Convert internal TextToolCall to OpenAI ToolCall format.
fn to_openai_tool_calls(calls: Vec<TextToolCall>) -> Vec<ToolCall> {
    calls
        .into_iter()
        .enumerate()
        .map(|(i, tc)| ToolCall {
            id: format!("call_text_{}_{}", chrono_millis(), i),
            r#type: "function".to_string(),
            function: FunctionCall {
                name: tc.name,
                arguments: serde_json::to_string(&tc.params).unwrap_or_else(|_| "{}".to_string()),
            },
        })
        .collect()
}

/// Simple millisecond timestamp for IDs.
fn chrono_millis() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

/// Parse ````tool_call\n{...}\n```` code blocks.
fn parse_code_block_tool_calls(text: &str) -> Vec<TextToolCall> {
    let mut calls = Vec::new();
    let mut remaining = text;

    while let Some(start_idx) = remaining.find("```tool_call") {
        let after = &remaining[start_idx + 12..]; // skip "```tool_call"

        // Find the closing ```
        if let Some(end_idx) = after.find("```") {
            let content = after[..end_idx].trim();

            if let Ok(parsed) = serde_json::from_str::<Value>(content) {
                if let Some(name) = extract_function_name(&parsed) {
                    let params = extract_function_params(&parsed);
                    calls.push(TextToolCall {
                        name,
                        params,
                        raw: remaining[start_idx..start_idx + 12 + end_idx + 3].to_string(),
                    });
                }
            }
            remaining = &after[end_idx + 3..];
        } else {
            break;
        }
    }

    calls
}

/// Parse blockquote format: `> {"action":"function_call",...}`
///
/// Used by ChatGPT — it outputs tool calls as blockquotes to differentiate
/// from regular text.
fn parse_blockquote_tool_calls(text: &str) -> Vec<TextToolCall> {
    let mut calls = Vec::new();

    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(json_part) = trimmed.strip_prefix("> ") {
            let json_part = json_part.trim();
            if json_part.starts_with('{') {
                if let Ok(parsed) = serde_json::from_str::<Value>(json_part) {
                    if parsed
                        .get("action")
                        .and_then(|v| v.as_str())
                        == Some("function_call")
                    {
                        if let Some(name) = parsed.get("name").and_then(|v| v.as_str()) {
                            let params = parsed
                                .get("arguments")
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
/// Gemini outputs multiple JSON objects concatenated on ONE LINE with no
/// separator. Standard regex fails on nested `{}`. This function walks
/// character by character, tracking brace depth.
///
/// Example input:
/// ```text
/// {"action":"function_call","name":"read","arguments":{"path":"/etc/hostname"}}{"action":"function_call","name":"exec","arguments":{"command":"ls"}}
/// ```
fn parse_json_tool_calls(text: &str) -> Vec<TextToolCall> {
    let mut calls = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if chars[i] == '{' {
            let start = i;
            let mut depth = 0;

            // Walk to find matching closing brace
            while i < chars.len() {
                match chars[i] {
                    '{' => depth += 1,
                    '}' => {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                    _ => {}
                }
                i += 1;
            }

            if depth != 0 {
                // Unbalanced braces — skip
                i += 1;
                continue;
            }

            let candidate: String = chars[start..=i.min(chars.len() - 1)].iter().collect();

            if let Ok(parsed) = serde_json::from_str::<Value>(&candidate) {
                if let Some(name) = extract_function_name(&parsed) {
                    let params = extract_function_params(&parsed);
                    calls.push(TextToolCall {
                        name,
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

/// Extract function name from a parsed JSON tool call.
///
/// Supports multiple formats:
/// - `{"action":"function_call", "name":"read", ...}`
/// - `{"function":"read", "params":{...}}`
/// - `{"name":"read", "arguments":{...}}`
fn extract_function_name(parsed: &Value) -> Option<String> {
    // Format 1: {"action":"function_call", "name":"..."}
    if parsed
        .get("action")
        .and_then(|v| v.as_str())
        == Some("function_call")
    {
        return parsed.get("name").and_then(|v| v.as_str()).map(String::from);
    }

    // Format 2: {"function":"...", "params":{...}}
    if let Some(name) = parsed.get("function").and_then(|v| v.as_str()) {
        return Some(name.to_string());
    }

    // Format 3: {"name":"...", "arguments":{...}} (if has arguments key)
    if parsed.get("arguments").is_some() || parsed.get("params").is_some() {
        if let Some(name) = parsed.get("name").and_then(|v| v.as_str()) {
            // Avoid matching arbitrary JSON objects that just have a "name" field
            // but aren't tool calls
            if !name.is_empty() {
                return Some(name.to_string());
            }
        }
    }

    // Format 4: {"action":"read", "arguments":{...}} — action IS the function name
    if let Some(action) = parsed.get("action").and_then(|v| v.as_str()) {
        if action != "function_call" && parsed.get("arguments").is_some() {
            return Some(action.to_string());
        }
    }

    None
}

/// Extract function parameters from a parsed JSON tool call.
fn extract_function_params(parsed: &Value) -> Value {
    parsed
        .get("arguments")
        .or_else(|| parsed.get("params"))
        .cloned()
        .unwrap_or(Value::Object(serde_json::Map::new()))
}

// ─── Content Cleanup ───────────────────────────────────────────────────────

/// Strip text-based tool call blocks from response text.
///
/// Returns the cleaned text content after removing any JSON tool call
/// blocks that were parsed.
pub fn strip_tool_call_text(text: &str, tool_calls: &[ToolCall]) -> String {
    if tool_calls.is_empty() {
        return text.to_string();
    }

    let mut result = text.to_string();

    // Remove ```tool_call blocks
    while let Some(start) = result.find("```tool_call") {
        let after = &result[start + 12..];
        if let Some(end) = after.find("```") {
            result.replace_range(start..start + 12 + end + 3, "");
        } else {
            break;
        }
    }

    // Remove blockquote tool call lines
    let lines: Vec<&str> = result
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            if let Some(json_part) = trimmed.strip_prefix("> ") {
                let json_part = json_part.trim();
                if let Ok(parsed) = serde_json::from_str::<Value>(json_part) {
                    if parsed
                        .get("action")
                        .and_then(|v| v.as_str())
                        == Some("function_call")
                    {
                        return false; // Remove this line
                    }
                }
            }
            true
        })
        .collect();
    result = lines.join("\n");

    // Remove bare JSON tool call objects
    for tc in tool_calls {
        let name = &tc.function.name;
        // Build a pattern to identify tool call JSON in text
        let patterns = [
            format!(r#""name":"{}""#, name),
            format!(r#""name": "{}""#, name),
            format!(r#""function":"{}""#, name),
            format!(r#""function": "{}""#, name),
        ];

        for pattern in &patterns {
            // Find JSON objects containing this pattern
            let chars: Vec<char> = result.chars().collect();
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
                                if depth == 0 {
                                    break;
                                }
                            }
                            _ => {}
                        }
                        i += 1;
                    }
                    if depth == 0 {
                        let candidate: String =
                            chars[start..=i.min(chars.len() - 1)].iter().collect();
                        if candidate.contains(pattern.as_str()) {
                            result = result.replacen(&candidate, "", 1);
                            break; // Re-scan from start after removal
                        }
                    }
                }
                i += 1;
            }
        }
    }

    // Clean up extra whitespace
    result = result
        .lines()
        .map(|l| l.trim_end())
        .collect::<Vec<&str>>()
        .join("\n");

    // Remove leading/trailing blank lines
    result.trim().to_string()
}

// ─── Helpers ───────────────────────────────────────────────────────────────

/// Find the start of a markdown section by its heading.
fn find_section_start(text: &str, heading: &str) -> Option<usize> {
    text.find(heading)
}

/// Find the start of the next `## ` heading after `start`.
fn find_next_h2(text: &str, start: usize) -> Option<usize> {
    let after = &text[start..];
    // Skip past the current heading line
    let skip = after.find('\n').map(|i| i + 1).unwrap_or(0);

    if let Some(pos) = after[skip..].find("\n## ") {
        Some(start + skip + pos + 1) // +1 for the \n
    } else {
        // No next ## — section goes to end of text
        Some(text.len())
    }
}

// ─── Tests ─────────────────────────────────────────────────────────────────

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

        let args0: Value = serde_json::from_str(&calls[0].function.arguments).unwrap();
        assert_eq!(args0["path"], "/etc/hostname");
    }

    #[test]
    fn test_parse_bare_json_multiline() {
        let text = r#"{"action":"function_call","name":"read","arguments":{"path":"/a.txt"}}
{"action":"function_call","name":"read","arguments":{"path":"/b.txt"}}"#;
        let calls = parse_text_tool_calls(text);
        assert_eq!(calls.len(), 2);
    }

    #[test]
    fn test_parse_blockquote_tool_calls() {
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

    #[test]
    fn test_parse_code_block_tool_calls() {
        let text = "Sure, let me check:\n```tool_call\n{\"name\": \"read\", \"params\": {\"path\": \"/etc/hosts\"}}\n```\n";
        let calls = parse_text_tool_calls(text);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].function.name, "read");
    }

    #[test]
    fn test_parse_alternative_json_format() {
        let text = r#"{"function":"shell","params":{"command":"ls -la"}}"#;
        let calls = parse_text_tool_calls(text);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].function.name, "shell");
    }

    #[test]
    fn test_no_tool_calls_in_regular_text() {
        let text = "Hello! I can help you with that. Let me know what you need.";
        let calls = parse_text_tool_calls(text);
        assert_eq!(calls.len(), 0);
    }

    #[test]
    fn test_no_false_positive_on_json_data() {
        // JSON that contains "name" but isn't a tool call
        let text = r#"Here is the user data: {"name": "Alice", "age": 30}"#;
        let calls = parse_text_tool_calls(text);
        assert_eq!(calls.len(), 0);
    }

    #[test]
    fn test_mixed_text_and_tool_calls() {
        let text = r#"Let me check that for you.
{"action":"function_call","name":"shell","arguments":{"command":"ls -la /tmp"}}
I'll review the results."#;
        let calls = parse_text_tool_calls(text);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].function.name, "shell");
    }

    #[test]
    fn test_strip_tool_call_text() {
        let text = r#"Let me check.
{"action":"function_call","name":"read","arguments":{"path":"/etc/hostname"}}
I'll get back to you."#;
        let calls = parse_text_tool_calls(text);
        let cleaned = strip_tool_call_text(text, &calls);
        assert!(!cleaned.contains("function_call"));
        assert!(cleaned.contains("Let me check."));
        assert!(cleaned.contains("I'll get back to you."));
    }

    #[test]
    fn test_strip_blockquote_tool_calls() {
        let text = r#"Reading files:
> {"action": "function_call", "name": "read", "arguments": {"path": "/a.txt"}}
Done."#;
        let calls = parse_text_tool_calls(text);
        let cleaned = strip_tool_call_text(text, &calls);
        assert!(!cleaned.contains("function_call"));
        assert!(cleaned.contains("Reading files:"));
        assert!(cleaned.contains("Done."));
    }

    #[test]
    fn test_nested_json_in_arguments() {
        let text = r#"{"action":"function_call","name":"write","arguments":{"path":"/a.json","content":"{\"key\":\"value\"}"}}"#;
        let calls = parse_text_tool_calls(text);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].function.name, "write");
    }

    #[test]
    fn test_transform_system_prompt() {
        let system = "You are a helpful assistant.\n\n## Tooling\nUse tools wisely.\n\n## Other\nBe nice.";
        let tools = vec![ToolDefinition {
            name: "read".to_string(),
            description: "Read a file".to_string(),
            parameters: serde_json::json!({}),
        }];
        let result = transform_system_prompt_for_webchat(system, &tools);

        assert!(result.contains("## Tool Use — MANDATORY"));
        assert!(result.contains("read"));
        assert!(!result.contains("## Tooling"));
        assert!(result.contains("## Other")); // Preserved
        assert!(result.contains("You are a helpful assistant.")); // Preserved
    }

    #[test]
    fn test_consolidate_uses_system_instruction_tag() {
        let messages = vec![Message::system("You are a helper.".to_string())];
        let result = consolidate_messages(&messages, &[], ProviderType::Gemini);
        assert!(result.contains("<system_instruction>"));
        assert!(!result.contains("\n<system>\n")); // NOT <system> — Gemini rejects it
    }

    #[test]
    fn test_consolidate_single_user_message() {
        let messages = vec![Message::user("Hello!".to_string())];
        let result = consolidate_messages(&messages, &[], ProviderType::Generic);
        assert_eq!(result, "Hello!"); // No wrapping tags for simple messages
    }

    #[test]
    fn test_tool_call_ids_are_unique() {
        let text = r#"{"action":"function_call","name":"a","arguments":{}}{"action":"function_call","name":"b","arguments":{}}"#;
        let calls = parse_text_tool_calls(text);
        assert_eq!(calls.len(), 2);
        assert_ne!(calls[0].id, calls[1].id);
    }

    #[test]
    fn test_gemini_parser_isolated() {
        let text = r#"{"action":"function_call","name":"read","arguments":{"path":"/etc/hostname"}}"#;
        let calls = parse_gemini_tool_calls(text);
        assert_eq!(calls.len(), 1);
    }

    #[test]
    fn test_chatgpt_parser_prefers_blockquote() {
        let text = r#"> {"action": "function_call", "name": "read", "arguments": {"path": "/a.txt"}}
Some other {"name": "noise", "value": 42} text"#;
        let calls = parse_chatgpt_tool_calls(text);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].function.name, "read");
    }
}
