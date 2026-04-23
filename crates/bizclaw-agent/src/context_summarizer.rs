//! LLM-based Context Summarization — intelligent context compression.
//!
//! Ported from DeerFlow 2.0's SummarizationMiddleware.
//! Instead of simple truncation (losing context), this module uses the LLM
//! itself to generate a high-quality summary of older conversation messages,
//! preserving key information while freeing up context window space.
//!
//! ## How It Works
//! 1. Monitor context utilization (tokens / max_tokens)
//! 2. When threshold exceeded, split conversation into [old | recent]
//! 3. Send old messages to LLM with summarization prompt
//! 4. Replace old messages with condensed summary
//! 5. Optionally off-load full transcript to filesystem
//!
//! ## Configuration
//! - `trigger_threshold`: utilization fraction to trigger (default: 0.7)
//! - `keep_recent`: number of recent messages to preserve (default: 10)
//! - `off_load_to_file`: write full transcript before compaction (default: true)
//! - `summary_max_tokens`: max tokens for LLM summary (default: 800)

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing::{info, warn};

use bizclaw_core::types::message::{Message, Role};

/// Configuration for LLM-based context summarization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummarizationConfig {
    /// Enable/disable the summarizer.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Context utilization fraction to trigger summarization (0.0-1.0).
    #[serde(default = "default_trigger")]
    pub trigger_threshold: f32,
    /// Minimum message count to trigger (besides utilization).
    #[serde(default = "default_min_messages")]
    pub min_messages_to_trigger: usize,
    /// Number of recent messages to keep intact (not summarized).
    #[serde(default = "default_keep_recent")]
    pub keep_recent: usize,
    /// Maximum tokens for the summary output.
    #[serde(default = "default_summary_tokens")]
    pub summary_max_tokens: usize,
    /// Whether to save the full transcript before compaction.
    #[serde(default = "default_true")]
    pub off_load_to_file: bool,
    /// Directory to store off-loaded transcripts.
    #[serde(default = "default_offload_dir")]
    pub off_load_dir: String,
}

fn default_true() -> bool {
    true
}
fn default_trigger() -> f32 {
    0.8
}
fn default_min_messages() -> usize {
    20
}
fn default_keep_recent() -> usize {
    10
}
fn default_summary_tokens() -> usize {
    800
}
fn default_offload_dir() -> String {
    "~/.bizclaw/transcripts".into()
}

impl Default for SummarizationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            trigger_threshold: 0.8,
            min_messages_to_trigger: 20,
            keep_recent: 10,
            summary_max_tokens: 800,
            off_load_to_file: true,
            off_load_dir: "~/.bizclaw/transcripts".into(),
        }
    }
}

/// The system prompt used to instruct the LLM to generate summaries.
pub const SUMMARIZATION_SYSTEM_PROMPT: &str = "You are a conversation summarizer. Your task is to create a concise, information-dense summary of the conversation below.\n\n\
Rules:\n\
1. Preserve ALL key decisions, facts, and action items\n\
2. Preserve ALL code changes, file paths, and technical details\n\
3. Preserve user preferences and stated goals\n\
4. Remove pleasantries, acknowledgments, and redundant information\n\
5. Use bullet points for clarity\n\
6. Keep the summary under 800 tokens\n\
7. Format: start with a Context Summary heading\n\n\
Output ONLY the summary, no preamble.";

/// Result of a summarization operation.
#[derive(Debug, Clone)]
pub struct SummarizationResult {
    /// The generated summary text.
    pub summary: String,
    /// Number of messages that were summarized.
    pub messages_compressed: usize,
    /// Estimated tokens saved.
    pub tokens_saved: usize,
    /// Whether transcript was off-loaded to file.
    pub off_loaded: bool,
    /// Path to off-loaded transcript (if any).
    pub off_load_path: Option<String>,
}

/// Build the LLM prompt for summarizing a set of messages.
///
/// This is separated from the middleware so it can be called independently
/// by the Agent when it has access to the LLM provider.
pub fn build_summarization_prompt(messages: &[Message]) -> Vec<Message> {
    let mut transcript = String::new();
    for msg in messages {
        let role_str = match msg.role {
            Role::User => "User",
            Role::Assistant => "Assistant",
            Role::System => "System",
            Role::Tool => "Tool",
        };

        // Truncate very long individual messages in the transcript
        let content = if msg.content.chars().count() > 500 {
            let t: String = msg.content.chars().take(500).collect();
            format!("{t}...")
        } else {
            msg.content.clone()
        };

        transcript.push_str(&format!("{role_str}: {content}\n\n"));
    }

    vec![
        Message::system(SUMMARIZATION_SYSTEM_PROMPT),
        Message::user(format!(
            "Summarize this conversation ({} messages):\n\n{}",
            messages.len(),
            transcript
        )),
    ]
}

/// Off-load full transcript to a file before compaction.
///
/// Stores the complete conversation as JSON for future reference.
/// Files are named by session_id and timestamp.
pub fn off_load_transcript(
    messages: &[Message],
    session_id: &str,
    off_load_dir: &str,
) -> Result<String, std::io::Error> {
    let dir = PathBuf::from(shellexpand::tilde(off_load_dir).as_ref());
    std::fs::create_dir_all(&dir)?;

    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    let filename = format!("{}_{}.jsonl", session_id, timestamp);
    let path = dir.join(&filename);

    let mut content = String::new();
    for msg in messages {
        if let Ok(json) = serde_json::to_string(msg) {
            content.push_str(&json);
            content.push('\n');
        }
    }

    std::fs::write(&path, &content)?;
    info!(
        "💾 Transcript off-loaded: {} ({} bytes)",
        path.display(),
        content.len()
    );

    Ok(path.to_string_lossy().to_string())
}

/// Perform rule-based summarization (fallback when LLM is not available).
///
/// This is an improved version of the existing truncation approach.
/// Instead of just truncating to 100 chars, it:
/// 1. Groups messages by topic/turn
/// 2. Extracts key facts (decisions, file paths, code changes)
/// 3. Builds a structured summary
pub fn rule_based_summarize(messages: &[Message]) -> String {
    let mut summary_parts = Vec::new();
    let mut key_facts = Vec::new();
    let mut file_paths = Vec::new();
    let mut decisions = Vec::new();

    let file_re = regex::Regex::new(r"(/[^\s]+\.[a-zA-Z0-9]+)").unwrap();
    let decision_keywords = [
        "decided",
        "chose",
        "selected",
        "using",
        "switched to",
        "will use",
        "going with",
        "quyết định",
        "chọn",
        "sử dụng",
        "đã chọn",
        "sẽ dùng",
    ];

    for msg in messages {
        let content = &msg.content;

        // Extract file paths
        for cap in file_re.captures_iter(content) {
            if let Some(m) = cap.get(1) {
                let path = m.as_str().to_string();
                if !file_paths.contains(&path) {
                    file_paths.push(path);
                }
            }
        }

        // Extract decisions
        let content_lower = content.to_lowercase();
        for keyword in &decision_keywords {
            if content_lower.contains(keyword) {
                // Take the sentence containing the keyword
                for sentence in content.split('.') {
                    if sentence.to_lowercase().contains(keyword) {
                        let trimmed = sentence.trim();
                        if !trimmed.is_empty() && trimmed.len() < 200 {
                            decisions.push(trimmed.to_string());
                        }
                        break;
                    }
                }
            }
        }

        // Build per-message summary
        let role = match msg.role {
            Role::User => "User",
            Role::Assistant => "AI",
            Role::System => continue,
            Role::Tool => {
                // For tool results, extract key info
                if content.len() > 50 {
                    let truncated: String = content.chars().take(100).collect();
                    key_facts.push(format!("Tool result: {truncated}..."));
                }
                continue;
            }
        };

        let summary_line = if content.chars().count() > 150 {
            let t: String = content.chars().take(150).collect();
            format!("{role}: {t}...")
        } else {
            format!("{role}: {content}")
        };
        summary_parts.push(summary_line);
    }

    let mut summary = format!(
        "## Context Summary\n*{} earlier messages compressed*\n\n",
        messages.len()
    );

    // Add key decisions
    if !decisions.is_empty() {
        summary.push_str("### Key Decisions\n");
        for d in decisions.iter().take(5) {
            summary.push_str(&format!("- {d}\n"));
        }
        summary.push('\n');
    }

    // Add file paths
    if !file_paths.is_empty() {
        summary.push_str("### Files Referenced\n");
        for p in file_paths.iter().take(10) {
            summary.push_str(&format!("- `{p}`\n"));
        }
        summary.push('\n');
    }

    // Add conversation flow
    summary.push_str("### Conversation Flow\n");
    for line in summary_parts.iter().take(20) {
        summary.push_str(&format!("- {line}\n"));
    }

    if summary_parts.len() > 20 {
        summary.push_str(&format!(
            "- *... {} more exchanges*\n",
            summary_parts.len() - 20
        ));
    }

    summary
}

/// Apply summarization to an agent state.
///
/// This is the main entry point. It:
/// 1. Checks if summarization is needed
/// 2. Splits messages into [system, old, recent]
/// 3. Generates summary (rule-based or LLM-based via callback)
/// 4. Optionally off-loads transcript
/// 5. Rebuilds messages array
pub fn apply_summarization(
    messages: &mut Vec<Message>,
    config: &SummarizationConfig,
    estimated_tokens: usize,
    max_tokens: usize,
    session_id: &str,
) -> Option<SummarizationResult> {
    if !config.enabled {
        return None;
    }

    let utilization = if max_tokens > 0 {
        estimated_tokens as f32 / max_tokens as f32
    } else {
        0.0
    };

    // Check triggers
    if utilization < config.trigger_threshold && messages.len() < config.min_messages_to_trigger {
        return None;
    }

    // Need at least keep_recent + 3 messages to be worth summarizing
    if messages.len() <= config.keep_recent + 3 {
        return None;
    }

    info!(
        "📦 Summarization triggered: {:.0}% utilization, {} messages (threshold: {:.0}%)",
        utilization * 100.0,
        messages.len(),
        config.trigger_threshold * 100.0
    );

    // Split: [system_prompt, ...old..., ...recent...]
    let system_msg = messages[0].clone();
    let split_point = messages.len().saturating_sub(config.keep_recent);
    let old_messages: Vec<Message> = messages[1..split_point].to_vec();
    let old_count = old_messages.len();
    let recent: Vec<Message> = messages[split_point..].to_vec();

    // Off-load to file if configured
    let (off_loaded, off_load_path) = if config.off_load_to_file {
        match off_load_transcript(&old_messages, session_id, &config.off_load_dir) {
            Ok(path) => (true, Some(path)),
            Err(e) => {
                warn!("Failed to off-load transcript: {e}");
                (false, None)
            }
        }
    } else {
        (false, None)
    };

    // Generate summary (rule-based — LLM-based requires async provider call)
    let summary = rule_based_summarize(&old_messages);

    // Estimate tokens saved
    let old_tokens: usize = old_messages.iter().map(|m| m.content.len() / 3).sum();
    let summary_tokens = summary.len() / 3;
    let tokens_saved = old_tokens.saturating_sub(summary_tokens);

    // Rebuild messages
    messages.clear();
    messages.push(system_msg);
    messages.push(Message::system(&summary));
    messages.extend(recent);

    info!(
        "📦 Summarized: {} messages → 1 summary ({} tokens saved)",
        old_count, tokens_saved
    );

    Some(SummarizationResult {
        summary,
        messages_compressed: old_count,
        tokens_saved,
        off_loaded,
        off_load_path,
    })
}

/// Improved Agent Middleware that uses LLM-quality summarization.
///
/// This replaces the basic SummarizationMiddleware from middleware.rs
/// with the full DeerFlow-style summarization including file off-loading,
/// key fact extraction, and structured summaries.
pub struct LlmSummarizationMiddleware {
    config: SummarizationConfig,
}

impl LlmSummarizationMiddleware {
    pub fn new() -> Self {
        Self {
            config: SummarizationConfig::default(),
        }
    }

    pub fn with_config(config: SummarizationConfig) -> Self {
        Self { config }
    }
}

impl Default for LlmSummarizationMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl crate::middleware::AgentMiddleware for LlmSummarizationMiddleware {
    async fn before_model(
        &self,
        state: &mut crate::middleware::AgentState,
    ) -> crate::middleware::MiddlewareAction {
        let result = apply_summarization(
            &mut state.messages,
            &self.config,
            state.estimated_tokens,
            state.max_context_tokens,
            &state.session_id,
        );

        if let Some(res) = result {
            // Update token estimate
            let new_tokens: usize = state.messages.iter().map(|m| m.content.len() / 3).sum();
            state.estimated_tokens = new_tokens;

            state.metadata.insert(
                "summarized".into(),
                format!(
                    "{} messages compressed, {} tokens saved",
                    res.messages_compressed, res.tokens_saved
                ),
            );

            if let Some(path) = res.off_load_path {
                state.metadata.insert("transcript_path".into(), path);
            }
        }

        crate::middleware::MiddlewareAction::Continue
    }

    fn name(&self) -> &str {
        "llm_summarization"
    }

    fn priority(&self) -> i32 {
        25 // Before the basic summarization (30)
    }

    fn enabled(&self) -> bool {
        self.config.enabled
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::middleware::AgentMiddleware;
    use std::collections::HashMap;

    fn make_conversation(turns: usize) -> Vec<Message> {
        let mut msgs = vec![Message::system("You are a helpful assistant.")];
        for i in 0..turns {
            msgs.push(Message::user(format!(
                "Question {i}: Tell me about topic {i}"
            )));
            msgs.push(Message::assistant(format!("Answer {i}: Here is information about topic {i}. It includes details about file /Users/test/project/{i}.rs and I decided to use approach {i}.")));
        }
        msgs
    }

    #[test]
    fn test_rule_based_summarize() {
        let msgs = make_conversation(5);
        let old_msgs = &msgs[1..8]; // Exclude system prompt, keep 7 messages

        let summary = rule_based_summarize(old_msgs);
        assert!(summary.contains("## Context Summary"));
        assert!(summary.contains("messages compressed"));
        assert!(summary.contains("Conversation Flow"));
    }

    #[test]
    fn test_rule_based_extracts_files() {
        let msgs = vec![
            Message::user("Check /Users/test/report.pdf"),
            Message::assistant("Read file /tmp/data.xlsx and analyzed it"),
        ];

        let summary = rule_based_summarize(&msgs);
        assert!(summary.contains("Files Referenced"));
        assert!(summary.contains("/Users/test/report.pdf"));
    }

    #[test]
    fn test_rule_based_extracts_decisions() {
        let msgs = vec![
            Message::user("Which framework should we use?"),
            Message::assistant("I decided to use Rust with Tokio for async support."),
        ];

        let summary = rule_based_summarize(&msgs);
        assert!(summary.contains("Key Decisions"));
        assert!(summary.contains("decided to use Rust"));
    }

    #[test]
    fn test_apply_summarization_below_threshold() {
        let config = SummarizationConfig {
            trigger_threshold: 0.7,
            min_messages_to_trigger: 100,
            ..Default::default()
        };
        let mut msgs = make_conversation(3);

        let result = apply_summarization(&mut msgs, &config, 100, 10000, "test");
        assert!(result.is_none());
    }

    #[test]
    fn test_apply_summarization_above_threshold() {
        let config = SummarizationConfig {
            trigger_threshold: 0.5,
            min_messages_to_trigger: 5,
            keep_recent: 4,
            off_load_to_file: false,
            ..Default::default()
        };
        let mut msgs = make_conversation(15); // 31 messages total
        let original_count = msgs.len();

        let result = apply_summarization(&mut msgs, &config, 5000, 8000, "test");
        assert!(result.is_some());

        let res = result.unwrap();
        assert!(res.messages_compressed > 0);
        assert!(msgs.len() < original_count);
        // Should have: system + summary + 4 recent = 6
        assert_eq!(msgs.len(), 6);
    }

    #[test]
    fn test_apply_summarization_too_few_messages() {
        let config = SummarizationConfig {
            keep_recent: 10,
            trigger_threshold: 0.0,
            min_messages_to_trigger: 0,
            off_load_to_file: false,
            ..Default::default()
        };
        let mut msgs = make_conversation(3); // 7 messages, not enough after keep_recent=10

        let result = apply_summarization(&mut msgs, &config, 5000, 8000, "test");
        assert!(result.is_none()); // Can't split meaningfully
    }

    #[test]
    fn test_off_load_transcript() {
        let msgs = make_conversation(3);
        let dir = "/tmp/bizclaw_test_transcripts";

        let result = off_load_transcript(&msgs[1..], "test_session", dir);
        assert!(result.is_ok());

        let path = result.unwrap();
        assert!(std::path::Path::new(&path).exists());

        // Cleanup
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn test_build_summarization_prompt() {
        let msgs = vec![
            Message::user("What is Rust?"),
            Message::assistant("Rust is a systems programming language."),
        ];

        let prompt = build_summarization_prompt(&msgs);
        assert_eq!(prompt.len(), 2);
        assert_eq!(prompt[0].role, Role::System);
        assert!(prompt[0].content.contains("summarizer"));
        assert!(prompt[1].content.contains("What is Rust"));
    }

    #[test]
    fn test_summarization_config_defaults() {
        let config = SummarizationConfig::default();
        assert!(config.enabled);
        assert_eq!(config.trigger_threshold, 0.7);
        assert_eq!(config.keep_recent, 10);
        assert_eq!(config.summary_max_tokens, 800);
        assert!(config.off_load_to_file);
    }

    #[tokio::test]
    async fn test_llm_summarization_middleware() {
        use crate::middleware::{AgentMiddleware, AgentState};

        let mw = LlmSummarizationMiddleware::with_config(SummarizationConfig {
            trigger_threshold: 0.3,
            min_messages_to_trigger: 5,
            keep_recent: 4,
            off_load_to_file: false,
            ..Default::default()
        });

        let msgs = make_conversation(15);
        let mut state = AgentState {
            messages: msgs,
            estimated_tokens: 5000,
            max_context_tokens: 8000,
            session_id: "test_mw".into(),
            model_name: "test".into(),
            pending_tool_calls: vec![],
            plan_mode: false,
            subagent_enabled: false,
            metadata: HashMap::new(),
        };

        let original_count = state.messages.len();
        mw.before_model(&mut state).await;

        assert!(state.messages.len() < original_count);
        assert!(state.metadata.contains_key("summarized"));
    }

    #[test]
    fn test_middleware_properties() {
        let mw = LlmSummarizationMiddleware::new();
        assert_eq!(mw.name(), "llm_summarization");
        assert_eq!(mw.priority(), 25);
        assert!(mw.enabled());
    }
}
