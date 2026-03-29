//! WebAuth types — shared across providers, proxy, and pipeline.

use serde::{Deserialize, Serialize};

/// A model exposed by a WebAuth provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebAuthModel {
    /// Model ID (e.g., "webauth-gemini-pro")
    pub id: String,
    /// Display name (e.g., "Gemini Pro (WebAuth)")
    pub name: String,
    /// Context window size
    pub context_window: u32,
}

/// Result of checking if a provider's web session is still valid.
#[derive(Debug, Clone)]
pub struct AuthCheckResult {
    pub authenticated: bool,
    pub user: Option<String>,
}

/// Status of a WebAuth provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProviderStatus {
    /// User is logged in, cookies valid
    Authenticated,
    /// Cookies expired or not present
    Expired,
    /// Never configured / no cookies
    NotConfigured,
    /// Provider is unavailable (e.g., browser crashed)
    Unavailable,
}

impl std::fmt::Display for ProviderStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProviderStatus::Authenticated => write!(f, "authenticated"),
            ProviderStatus::Expired => write!(f, "expired"),
            ProviderStatus::NotConfigured => write!(f, "not-configured"),
            ProviderStatus::Unavailable => write!(f, "unavailable"),
        }
    }
}

/// OpenAI-compatible chat completion request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(default)]
    pub stream: bool,
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<serde_json::Value>,
}

fn default_temperature() -> f32 {
    0.7
}
fn default_max_tokens() -> u32 {
    4096
}

/// A single chat message in OpenAI format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<serde_json::Value>>,
}

/// OpenAI-compatible SSE chunk for streaming responses.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIChatChunk {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<ChunkChoice>,
}

/// A single choice in a streaming chunk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkChoice {
    pub index: u32,
    pub delta: ChunkDelta,
    pub finish_reason: Option<String>,
}

/// Delta content in a streaming chunk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkDelta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<serde_json::Value>>,
}

/// Non-streaming OpenAI-compatible response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIChatResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<ResponseChoice>,
    pub usage: Option<UsageInfo>,
}

/// A single choice in a non-streaming response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseChoice {
    pub index: u32,
    pub message: ResponseMessage,
    pub finish_reason: String,
}

/// Message in a non-streaming response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseMessage {
    pub role: String,
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<serde_json::Value>>,
}

/// Usage info for response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageInfo {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

impl OpenAIChatChunk {
    /// Create a text content chunk.
    pub fn text(model: &str, text: &str, finish: bool) -> Self {
        Self {
            id: format!("chatcmpl-webauth-{}", uuid::Uuid::new_v4()),
            object: "chat.completion.chunk".to_string(),
            created: chrono::Utc::now().timestamp() as u64,
            model: model.to_string(),
            choices: vec![ChunkChoice {
                index: 0,
                delta: ChunkDelta {
                    role: None,
                    content: Some(text.to_string()),
                    tool_calls: None,
                },
                finish_reason: if finish {
                    Some("stop".to_string())
                } else {
                    None
                },
            }],
        }
    }

    /// Create a role-only initial chunk.
    pub fn role(model: &str) -> Self {
        Self {
            id: format!("chatcmpl-webauth-{}", uuid::Uuid::new_v4()),
            object: "chat.completion.chunk".to_string(),
            created: chrono::Utc::now().timestamp() as u64,
            model: model.to_string(),
            choices: vec![ChunkChoice {
                index: 0,
                delta: ChunkDelta {
                    role: Some("assistant".to_string()),
                    content: None,
                    tool_calls: None,
                },
                finish_reason: None,
            }],
        }
    }

    /// Create a tool_calls chunk.
    pub fn tool_calls(model: &str, calls: Vec<serde_json::Value>) -> Self {
        Self {
            id: format!("chatcmpl-webauth-{}", uuid::Uuid::new_v4()),
            object: "chat.completion.chunk".to_string(),
            created: chrono::Utc::now().timestamp() as u64,
            model: model.to_string(),
            choices: vec![ChunkChoice {
                index: 0,
                delta: ChunkDelta {
                    role: None,
                    content: None,
                    tool_calls: Some(calls),
                },
                finish_reason: Some("tool_calls".to_string()),
            }],
        }
    }
}

impl OpenAIChatResponse {
    /// Create a text response.
    pub fn text(model: &str, text: &str) -> Self {
        Self {
            id: format!("chatcmpl-webauth-{}", uuid::Uuid::new_v4()),
            object: "chat.completion".to_string(),
            created: chrono::Utc::now().timestamp() as u64,
            model: model.to_string(),
            choices: vec![ResponseChoice {
                index: 0,
                message: ResponseMessage {
                    role: "assistant".to_string(),
                    content: Some(text.to_string()),
                    tool_calls: None,
                },
                finish_reason: "stop".to_string(),
            }],
            usage: Some(UsageInfo {
                prompt_tokens: 0,
                completion_tokens: 0,
                total_tokens: 0,
            }),
        }
    }

    /// Create a response with tool calls.
    pub fn with_tool_calls(
        model: &str,
        content: Option<String>,
        calls: Vec<serde_json::Value>,
    ) -> Self {
        Self {
            id: format!("chatcmpl-webauth-{}", uuid::Uuid::new_v4()),
            object: "chat.completion".to_string(),
            created: chrono::Utc::now().timestamp() as u64,
            model: model.to_string(),
            choices: vec![ResponseChoice {
                index: 0,
                message: ResponseMessage {
                    role: "assistant".to_string(),
                    content,
                    tool_calls: Some(calls),
                },
                finish_reason: "tool_calls".to_string(),
            }],
            usage: Some(UsageInfo {
                prompt_tokens: 0,
                completion_tokens: 0,
                total_tokens: 0,
            }),
        }
    }
}
