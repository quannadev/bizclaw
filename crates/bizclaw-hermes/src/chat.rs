//! # Chat Session
//! 
//! Chat session management cho Hermes agent

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// Message role
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

impl MessageRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            MessageRole::System => "system",
            MessageRole::User => "user",
            MessageRole::Assistant => "assistant",
            MessageRole::Tool => "tool",
        }
    }
}

/// Chat message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: MessageRole,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

impl Message {
    pub fn system(content: &str) -> Self {
        Self {
            role: MessageRole::System,
            content: content.to_string(),
            tool_call_id: None,
        }
    }

    pub fn user(content: &str) -> Self {
        Self {
            role: MessageRole::User,
            content: content.to_string(),
            tool_call_id: None,
        }
    }

    pub fn assistant(content: &str) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: content.to_string(),
            tool_call_id: None,
        }
    }

    pub fn tool(content: &str, tool_call_id: &str) -> Self {
        Self {
            role: MessageRole::Tool,
            content: content.to_string(),
            tool_call_id: Some(tool_call_id.to_string()),
        }
    }
}

/// Chat session
#[derive(Debug, Clone)]
pub struct ChatSession {
    pub id: String,
    pub messages: Vec<Message>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub metadata: SessionMetadata,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionMetadata {
    pub user_id: Option<String>,
    pub title: Option<String>,
    pub tags: Vec<String>,
    pub context: Option<String>,
}

impl ChatSession {
    pub fn new() -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            messages: Vec::new(),
            created_at: now,
            updated_at: now,
            metadata: SessionMetadata::default(),
        }
    }

    pub fn with_title(mut self, title: &str) -> Self {
        self.metadata.title = Some(title.to_string());
        self
    }

    pub fn add_message(&mut self, message: Message) {
        self.messages.push(message);
        self.updated_at = Utc::now();
    }

    pub fn add_system(&mut self, content: &str) {
        self.add_message(Message::system(content));
    }

    pub fn add_user(&mut self, content: &str) {
        self.add_message(Message::user(content));
    }

    pub fn add_assistant(&mut self, content: &str) {
        self.add_message(Message::assistant(content));
    }

    pub fn add_tool(&mut self, content: &str, tool_call_id: &str) {
        self.add_message(Message::tool(content, tool_call_id));
    }

    pub fn get_messages(&self) -> &[Message] {
        &self.messages
    }

    pub fn clear(&mut self) {
        self.messages.clear();
        self.updated_at = Utc::now();
    }

    pub fn message_count(&self) -> usize {
        self.messages.len()
    }

    pub fn summary(&self) -> String {
        format!(
            "Session {} ({} messages, updated {})",
            self.id,
            self.messages.len(),
            self.updated_at.format("%Y-%m-%d %H:%M")
        )
    }
}

impl Default for ChatSession {
    fn default() -> Self {
        Self::new()
    }
}
