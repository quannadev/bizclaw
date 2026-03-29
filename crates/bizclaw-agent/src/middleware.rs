//! Agent Middleware Pipeline — composable pre/post processing chain.
//!
//! Ported from DeerFlow 2.0's middleware chain architecture.
//! Each middleware can intercept the agent loop at two points:
//! - `before_model`: preprocess state before LLM call
//! - `after_model`: postprocess response after LLM call
//!
//! DeerFlow runs 12 middlewares in strict order. BizClaw supports the same
//! pattern but in Rust with async traits and priority-based ordering.
//!
//! ## Built-in Middlewares
//! 1. `GuardrailMiddleware` — block dangerous tool calls
//! 2. `SummarizationMiddleware` — compress context when token budget is low
//! 3. `MemoryMiddleware` — queue conversations for async fact extraction
//! 4. `DanglingToolCallMiddleware` — inject placeholder for interrupted tool calls
//! 5. `SubagentLimitMiddleware` — cap concurrent sub-agent spawns

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, info, warn};

use bizclaw_core::types::message::Message;

/// Result of a middleware operation.
#[derive(Debug, Clone)]
pub enum MiddlewareAction {
    /// Continue to next middleware.
    Continue,
    /// Skip remaining middlewares and proceed.
    Skip,
    /// Abort the agent loop with a message.
    Abort(String),
    /// Inject messages into the conversation.
    Inject(Vec<Message>),
}

/// Mutable agent state passed through the middleware chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentState {
    /// Current conversation messages.
    pub messages: Vec<Message>,
    /// Estimated token count.
    pub estimated_tokens: usize,
    /// Max context window.
    pub max_context_tokens: usize,
    /// Current session/thread ID.
    pub session_id: String,
    /// Current model name.
    pub model_name: String,
    /// Tool calls from the last LLM response.
    #[serde(default)]
    pub pending_tool_calls: Vec<bizclaw_core::types::ToolCall>,
    /// Whether plan mode is enabled.
    #[serde(default)]
    pub plan_mode: bool,
    /// Whether sub-agents are enabled.
    #[serde(default)]
    pub subagent_enabled: bool,
    /// Metadata bag for middlewares to share data.
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

impl AgentState {
    /// Context utilization as a fraction (0.0 - 1.0).
    pub fn utilization(&self) -> f32 {
        if self.max_context_tokens == 0 {
            return 0.0;
        }
        self.estimated_tokens as f32 / self.max_context_tokens as f32
    }
}

/// The middleware trait — implement this for custom middlewares.
#[async_trait]
pub trait AgentMiddleware: Send + Sync {
    /// Called before the LLM generates a response.
    /// Return `Continue` to proceed, `Abort` to stop, `Inject` to add messages.
    async fn before_model(&self, state: &mut AgentState) -> MiddlewareAction {
        let _ = state;
        MiddlewareAction::Continue
    }

    /// Called after the LLM response (with tool calls parsed).
    /// Can modify pending_tool_calls, inject messages, or abort.
    async fn after_model(&self, state: &mut AgentState) -> MiddlewareAction {
        let _ = state;
        MiddlewareAction::Continue
    }

    /// Middleware name (for logging and debugging).
    fn name(&self) -> &str;

    /// Execution priority (lower = runs first). Default: 100.
    fn priority(&self) -> i32 {
        100
    }

    /// Whether this middleware is currently enabled.
    fn enabled(&self) -> bool {
        true
    }
}

/// The middleware pipeline — runs all middlewares in priority order.
pub struct MiddlewarePipeline {
    middlewares: Vec<Box<dyn AgentMiddleware>>,
}

impl MiddlewarePipeline {
    /// Create an empty pipeline.
    pub fn new() -> Self {
        Self {
            middlewares: Vec::new(),
        }
    }

    /// Create a pipeline with the default BizClaw middlewares.
    pub fn with_defaults() -> Self {
        let mut pipeline = Self::new();
        pipeline.add(Box::new(DanglingToolCallMiddleware));
        pipeline.add(Box::new(GuardrailMiddleware::new()));
        pipeline.add(Box::new(SummarizationMiddleware::new(0.75, 10)));
        pipeline.add(Box::new(SubagentLimitMiddleware::new(3)));
        pipeline.add(Box::new(MemoryMiddleware::new()));
        pipeline
    }

    /// Add a middleware to the pipeline.
    pub fn add(&mut self, middleware: Box<dyn AgentMiddleware>) {
        self.middlewares.push(middleware);
        // Sort by priority after each add
        self.middlewares.sort_by_key(|m| m.priority());
    }

    /// Remove a middleware by name.
    pub fn remove(&mut self, name: &str) -> bool {
        let before = self.middlewares.len();
        self.middlewares.retain(|m| m.name() != name);
        self.middlewares.len() < before
    }

    /// Run all `before_model` middlewares in order.
    pub async fn run_before_model(&self, state: &mut AgentState) -> MiddlewareAction {
        for mw in &self.middlewares {
            if !mw.enabled() {
                continue;
            }
            debug!("⚙️ Middleware [before] {}", mw.name());
            match mw.before_model(state).await {
                MiddlewareAction::Continue => continue,
                MiddlewareAction::Skip => {
                    debug!("⏭️ {} skipped remaining middlewares", mw.name());
                    return MiddlewareAction::Continue;
                }
                MiddlewareAction::Abort(reason) => {
                    warn!("🛑 {} aborted: {}", mw.name(), reason);
                    return MiddlewareAction::Abort(reason);
                }
                MiddlewareAction::Inject(msgs) => {
                    debug!("💉 {} injected {} messages", mw.name(), msgs.len());
                    state.messages.extend(msgs);
                }
            }
        }
        MiddlewareAction::Continue
    }

    /// Run all `after_model` middlewares in order.
    pub async fn run_after_model(&self, state: &mut AgentState) -> MiddlewareAction {
        for mw in &self.middlewares {
            if !mw.enabled() {
                continue;
            }
            debug!("⚙️ Middleware [after] {}", mw.name());
            match mw.after_model(state).await {
                MiddlewareAction::Continue => continue,
                MiddlewareAction::Skip => {
                    return MiddlewareAction::Continue;
                }
                MiddlewareAction::Abort(reason) => {
                    warn!("🛑 {} aborted post-model: {}", mw.name(), reason);
                    return MiddlewareAction::Abort(reason);
                }
                MiddlewareAction::Inject(msgs) => {
                    state.messages.extend(msgs);
                }
            }
        }
        MiddlewareAction::Continue
    }

    /// List active middleware names.
    pub fn list(&self) -> Vec<(&str, i32, bool)> {
        self.middlewares
            .iter()
            .map(|m| (m.name(), m.priority(), m.enabled()))
            .collect()
    }

    /// Count active middlewares.
    pub fn count(&self) -> usize {
        self.middlewares.iter().filter(|m| m.enabled()).count()
    }
}

impl Default for MiddlewarePipeline {
    fn default() -> Self {
        Self::with_defaults()
    }
}

// ─────────────────────────────────────────────────────────────
// Built-in Middlewares
// ─────────────────────────────────────────────────────────────

/// Guardrail Middleware — blocks dangerous tool calls before execution.
///
/// Checks tool calls against an allowlist/blocklist of patterns.
/// Ported from DeerFlow's GuardrailMiddleware with AllowlistProvider.
pub struct GuardrailMiddleware {
    /// Blocked tool name patterns.
    blocked_patterns: Vec<String>,
    /// Blocked argument patterns (e.g., `rm -rf`, `DROP TABLE`).
    blocked_arg_patterns: Vec<String>,
}

impl Default for GuardrailMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

impl GuardrailMiddleware {
    pub fn new() -> Self {
        Self {
            blocked_patterns: vec![],
            blocked_arg_patterns: vec![
                "rm -rf /".into(),
                "rm -rf ~".into(),
                "DROP TABLE".into(),
                "DROP DATABASE".into(),
                "TRUNCATE TABLE".into(),
                "format c:".into(),
                "mkfs".into(),
                "dd if=".into(),
                "> /dev/sda".into(),
                "chmod 777 /".into(),
                "curl | sh".into(),
                "wget | sh".into(),
            ],
        }
    }

    /// Add a blocked tool name pattern.
    pub fn block_tool(&mut self, pattern: impl Into<String>) {
        self.blocked_patterns.push(pattern.into());
    }

    /// Add a blocked argument pattern.
    pub fn block_arg_pattern(&mut self, pattern: impl Into<String>) {
        self.blocked_arg_patterns.push(pattern.into());
    }

    /// Check if a tool call is allowed.
    fn is_allowed(&self, tool_name: &str, arguments: &str) -> bool {
        // Check tool name blocklist
        let name_lower = tool_name.to_lowercase();
        for pattern in &self.blocked_patterns {
            if name_lower.contains(&pattern.to_lowercase()) {
                return false;
            }
        }

        // Check argument blocklist
        let args_lower = arguments.to_lowercase();
        for pattern in &self.blocked_arg_patterns {
            if args_lower.contains(&pattern.to_lowercase()) {
                return false;
            }
        }

        true
    }
}

#[async_trait]
impl AgentMiddleware for GuardrailMiddleware {
    async fn after_model(&self, state: &mut AgentState) -> MiddlewareAction {
        let mut blocked = Vec::new();

        for tc in &state.pending_tool_calls {
            if !self.is_allowed(&tc.function.name, &tc.function.arguments) {
                warn!(
                    "🛡 Guardrail blocked tool call: {} (args contain dangerous pattern)",
                    tc.function.name
                );
                blocked.push(tc.id.clone());
            }
        }

        if !blocked.is_empty() {
            // Remove blocked tool calls
            state
                .pending_tool_calls
                .retain(|tc| !blocked.contains(&tc.id));

            // Inject error messages for blocked calls
            let msgs: Vec<Message> = blocked
                .iter()
                .map(|id| {
                    Message::tool(
                        "⛔ Guardrail blocked this tool call: contains dangerous operation pattern",
                        id,
                    )
                })
                .collect();

            return MiddlewareAction::Inject(msgs);
        }

        MiddlewareAction::Continue
    }

    fn name(&self) -> &str {
        "guardrail"
    }

    fn priority(&self) -> i32 {
        10 // Runs early — blocks before execution
    }
}

/// Summarization Middleware — compresses old context when approaching token limit.
///
/// Ported from DeerFlow's SummarizationMiddleware.
/// Instead of truncating (losing context), summarizes old messages into a single
/// system message, keeping recent messages intact.
pub struct SummarizationMiddleware {
    /// Trigger when utilization exceeds this fraction (0.0-1.0).
    trigger_threshold: f32,
    /// Number of recent messages to keep intact.
    keep_recent: usize,
}

impl SummarizationMiddleware {
    pub fn new(trigger_threshold: f32, keep_recent: usize) -> Self {
        Self {
            trigger_threshold,
            keep_recent,
        }
    }
}

#[async_trait]
impl AgentMiddleware for SummarizationMiddleware {
    async fn before_model(&self, state: &mut AgentState) -> MiddlewareAction {
        let utilization = state.utilization();

        if utilization < self.trigger_threshold || state.messages.len() <= self.keep_recent + 2 {
            return MiddlewareAction::Continue;
        }

        info!(
            "📦 Summarization triggered: {:.0}% utilization, {} messages",
            utilization * 100.0,
            state.messages.len()
        );

        // Keep system prompt (index 0) and last N messages
        let system_msg = state.messages[0].clone();
        let split_point = state.messages.len().saturating_sub(self.keep_recent);

        // Clone old messages to avoid borrow conflict
        let old_messages: Vec<Message> = state.messages[1..split_point].to_vec();
        let old_count = old_messages.len();

        // Build summary of old messages
        let mut summary_parts = Vec::new();
        for msg in &old_messages {
            let role = match msg.role {
                bizclaw_core::types::Role::User => "User",
                bizclaw_core::types::Role::Assistant => "AI",
                bizclaw_core::types::Role::System => continue,
                bizclaw_core::types::Role::Tool => "Tool",
            };
            let content = if msg.content.chars().count() > 150 {
                let t: String = msg.content.chars().take(150).collect();
                format!("{t}...")
            } else {
                msg.content.clone()
            };
            summary_parts.push(format!("{role}: {content}"));
        }

        let summary = format!(
            "[Context Summary — {} earlier messages compressed]\n{}\n[End Summary]",
            old_count,
            summary_parts.join("\n")
        );

        // Rebuild messages
        let recent: Vec<Message> = state.messages[split_point..].to_vec();
        state.messages.clear();
        state.messages.push(system_msg);
        state.messages.push(Message::system(&summary));
        state.messages.extend(recent);

        // Update token estimate
        let new_tokens: usize = state.messages.iter().map(|m| m.content.len() / 3).sum();
        state.estimated_tokens = new_tokens;

        state.metadata.insert(
            "summarized".into(),
            format!("{} messages compressed", old_count),
        );

        MiddlewareAction::Continue
    }

    fn name(&self) -> &str {
        "summarization"
    }

    fn priority(&self) -> i32 {
        30
    }
}

/// Dangling Tool Call Middleware — handles interrupted tool calls.
///
/// When a user interrupts the agent mid-tool-call, there are AIMessages with
/// tool_calls but no corresponding ToolMessages. This middleware detects that
/// and injects placeholder ToolMessages so the LLM doesn't get confused.
pub struct DanglingToolCallMiddleware;

#[async_trait]
impl AgentMiddleware for DanglingToolCallMiddleware {
    async fn before_model(&self, state: &mut AgentState) -> MiddlewareAction {
        // Check for tool calls without responses
        let mut dangling_ids = Vec::new();
        let mut responded_ids = std::collections::HashSet::new();

        for msg in &state.messages {
            if let Some(ref id) = msg.tool_call_id {
                responded_ids.insert(id.clone());
            }
            if let Some(ref calls) = msg.tool_calls {
                for tc in calls {
                    dangling_ids.push(tc.id.clone());
                }
            }
        }

        let unresponded: Vec<String> = dangling_ids
            .into_iter()
            .filter(|id| !responded_ids.contains(id))
            .collect();

        if unresponded.is_empty() {
            return MiddlewareAction::Continue;
        }

        debug!(
            "🔧 DanglingToolCall: injecting {} placeholder responses",
            unresponded.len()
        );

        let placeholders: Vec<Message> = unresponded
            .into_iter()
            .map(|id| Message::tool("[Tool call was interrupted by user]", &id))
            .collect();

        MiddlewareAction::Inject(placeholders)
    }

    fn name(&self) -> &str {
        "dangling_tool_call"
    }

    fn priority(&self) -> i32 {
        5 // Runs first — fix state before anything else
    }
}

/// Sub-agent Limit Middleware — caps concurrent sub-agent spawns.
///
/// DeerFlow's SubagentLimitMiddleware truncates excess `task` tool calls
/// to enforce MAX_CONCURRENT_SUBAGENTS limit.
pub struct SubagentLimitMiddleware {
    max_concurrent: usize,
}

impl SubagentLimitMiddleware {
    pub fn new(max_concurrent: usize) -> Self {
        Self { max_concurrent }
    }
}

#[async_trait]
impl AgentMiddleware for SubagentLimitMiddleware {
    async fn after_model(&self, state: &mut AgentState) -> MiddlewareAction {
        if !state.subagent_enabled {
            return MiddlewareAction::Continue;
        }

        let task_calls: Vec<usize> = state
            .pending_tool_calls
            .iter()
            .enumerate()
            .filter(|(_, tc)| tc.function.name == "task" || tc.function.name == "delegate")
            .map(|(i, _)| i)
            .collect();

        if task_calls.len() <= self.max_concurrent {
            return MiddlewareAction::Continue;
        }

        warn!(
            "🚦 SubagentLimit: {} task calls, capping to {}",
            task_calls.len(),
            self.max_concurrent
        );

        // Keep first N task calls, remove the rest
        let to_remove: Vec<usize> = task_calls[self.max_concurrent..].to_vec();
        let removed_ids: Vec<String> = to_remove
            .iter()
            .rev()
            .map(|&i| {
                let id = state.pending_tool_calls[i].id.clone();
                state.pending_tool_calls.remove(i);
                id
            })
            .collect();

        let msgs: Vec<Message> = removed_ids
            .into_iter()
            .map(|id| {
                Message::tool(
                    format!(
                        "⚠️ Sub-agent limit reached (max {}). This task was deferred.",
                        self.max_concurrent
                    ),
                    &id,
                )
            })
            .collect();

        MiddlewareAction::Inject(msgs)
    }

    fn name(&self) -> &str {
        "subagent_limit"
    }

    fn priority(&self) -> i32 {
        90 // Runs late — after guardrail
    }
}

/// Memory Middleware — queues conversations for async fact extraction.
///
/// Ported from DeerFlow's MemoryMiddleware. Filters to user + final AI responses
/// and queues them for debounced background processing.
pub struct MemoryMiddleware {
    enabled: bool,
}

impl Default for MemoryMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryMiddleware {
    pub fn new() -> Self {
        Self { enabled: true }
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
}

#[async_trait]
impl AgentMiddleware for MemoryMiddleware {
    async fn after_model(&self, state: &mut AgentState) -> MiddlewareAction {
        if state.pending_tool_calls.is_empty() {
            // Final response — mark for memory save
            state
                .metadata
                .insert("memory_save_pending".into(), "true".into());
            debug!("🧠 MemoryMiddleware: queued for memory save");
        }
        MiddlewareAction::Continue
    }

    fn name(&self) -> &str {
        "memory"
    }

    fn priority(&self) -> i32 {
        80
    }

    fn enabled(&self) -> bool {
        self.enabled
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bizclaw_core::types::message::Role;

    fn make_state(msg_count: usize) -> AgentState {
        let mut messages = vec![Message::system("System prompt")];
        for i in 0..msg_count {
            messages.push(Message::user(&format!("User message {}", i)));
            messages.push(Message::assistant(&format!("Response {}", i)));
        }
        AgentState {
            estimated_tokens: msg_count * 100,
            max_context_tokens: 4000,
            messages,
            session_id: "test".into(),
            model_name: "gpt-4o-mini".into(),
            pending_tool_calls: vec![],
            plan_mode: false,
            subagent_enabled: false,
            metadata: HashMap::new(),
        }
    }

    #[tokio::test]
    async fn test_pipeline_runs_in_order() {
        let pipeline = MiddlewarePipeline::with_defaults();
        let mut state = make_state(3);

        let result = pipeline.run_before_model(&mut state).await;
        assert!(matches!(result, MiddlewareAction::Continue));
    }

    #[tokio::test]
    async fn test_guardrail_blocks_dangerous_commands() {
        let guardrail = GuardrailMiddleware::new();
        let mut state = make_state(1);
        state.pending_tool_calls = vec![bizclaw_core::types::ToolCall {
            id: "tc_1".into(),
            r#type: "function".into(),
            function: bizclaw_core::types::FunctionCall {
                name: "shell".into(),
                arguments: r#"{"command": "rm -rf /"}"#.into(),
            },
        }];

        let result = guardrail.after_model(&mut state).await;
        assert!(matches!(result, MiddlewareAction::Inject(_)));
        assert!(state.pending_tool_calls.is_empty()); // blocked!
    }

    #[tokio::test]
    async fn test_guardrail_allows_safe_commands() {
        let guardrail = GuardrailMiddleware::new();
        let mut state = make_state(1);
        state.pending_tool_calls = vec![bizclaw_core::types::ToolCall {
            id: "tc_1".into(),
            r#type: "function".into(),
            function: bizclaw_core::types::FunctionCall {
                name: "shell".into(),
                arguments: r#"{"command": "ls -la"}"#.into(),
            },
        }];

        let result = guardrail.after_model(&mut state).await;
        assert!(matches!(result, MiddlewareAction::Continue));
        assert_eq!(state.pending_tool_calls.len(), 1); // still there
    }

    #[tokio::test]
    async fn test_summarization_triggers_on_threshold() {
        let mw = SummarizationMiddleware::new(0.5, 4);
        let mut state = make_state(20); // 20 turns = 41 messages
        state.max_context_tokens = 2000;
        state.estimated_tokens = 1500; // 75% utilization

        let before_count = state.messages.len();
        mw.before_model(&mut state).await;

        assert!(state.messages.len() < before_count);
        assert!(state.metadata.contains_key("summarized"));
    }

    #[tokio::test]
    async fn test_summarization_skips_when_low() {
        let mw = SummarizationMiddleware::new(0.75, 4);
        let mut state = make_state(3);
        state.max_context_tokens = 10000;
        state.estimated_tokens = 300; // 3% utilization

        let before_count = state.messages.len();
        mw.before_model(&mut state).await;
        assert_eq!(state.messages.len(), before_count); // unchanged
    }

    #[tokio::test]
    async fn test_dangling_tool_call_detection() {
        let mw = DanglingToolCallMiddleware;
        let mut state = make_state(0);
        // Add assistant message with tool call but no tool response
        state.messages.push(Message {
            role: Role::Assistant,
            content: String::new(),
            name: None,
            tool_call_id: None,
            tool_calls: Some(vec![bizclaw_core::types::ToolCall {
                id: "orphan_1".into(),
                r#type: "function".into(),
                function: bizclaw_core::types::FunctionCall {
                    name: "search".into(),
                    arguments: "{}".into(),
                },
            }]),
        });

        let result = mw.before_model(&mut state).await;
        match result {
            MiddlewareAction::Inject(msgs) => {
                assert_eq!(msgs.len(), 1);
                assert_eq!(msgs[0].tool_call_id.as_deref(), Some("orphan_1"));
            }
            _ => panic!("Expected Inject"),
        }
    }

    #[tokio::test]
    async fn test_subagent_limit_caps_tasks() {
        let mw = SubagentLimitMiddleware::new(2);
        let mut state = make_state(1);
        state.subagent_enabled = true;
        state.pending_tool_calls = (0..5)
            .map(|i| bizclaw_core::types::ToolCall {
                id: format!("tc_{i}"),
                r#type: "function".into(),
                function: bizclaw_core::types::FunctionCall {
                    name: "task".into(),
                    arguments: format!(r#"{{"description": "task {i}"}}"#),
                },
            })
            .collect();

        let result = mw.after_model(&mut state).await;
        assert!(matches!(result, MiddlewareAction::Inject(_)));
        assert_eq!(state.pending_tool_calls.len(), 2); // capped to 2
    }

    #[tokio::test]
    async fn test_subagent_limit_noop_when_disabled() {
        let mw = SubagentLimitMiddleware::new(2);
        let mut state = make_state(1);
        state.subagent_enabled = false;
        state.pending_tool_calls = (0..5)
            .map(|i| bizclaw_core::types::ToolCall {
                id: format!("tc_{i}"),
                r#type: "function".into(),
                function: bizclaw_core::types::FunctionCall {
                    name: "task".into(),
                    arguments: "{}".into(),
                },
            })
            .collect();

        let result = mw.after_model(&mut state).await;
        assert!(matches!(result, MiddlewareAction::Continue));
        assert_eq!(state.pending_tool_calls.len(), 5); // unchanged
    }

    #[tokio::test]
    async fn test_pipeline_add_remove() {
        let mut pipeline = MiddlewarePipeline::new();
        assert_eq!(pipeline.count(), 0);

        pipeline.add(Box::new(GuardrailMiddleware::new()));
        pipeline.add(Box::new(DanglingToolCallMiddleware));
        assert_eq!(pipeline.count(), 2);

        assert!(pipeline.remove("guardrail"));
        assert_eq!(pipeline.count(), 1);

        assert!(!pipeline.remove("nonexistent"));
    }

    #[tokio::test]
    async fn test_pipeline_priority_order() {
        let pipeline = MiddlewarePipeline::with_defaults();
        let names = pipeline.list();
        // dangling_tool_call (5) → guardrail (10) → summarization (30) → memory (80) → subagent_limit (90)
        assert_eq!(names[0].0, "dangling_tool_call");
        assert_eq!(names[1].0, "guardrail");
        assert_eq!(names[2].0, "summarization");
    }

    #[tokio::test]
    async fn test_memory_middleware_marks_save() {
        let mw = MemoryMiddleware::new();
        let mut state = make_state(1);
        // No pending tool calls → final response
        state.pending_tool_calls.clear();

        mw.after_model(&mut state).await;
        assert_eq!(
            state.metadata.get("memory_save_pending"),
            Some(&"true".to_string())
        );
    }
}
