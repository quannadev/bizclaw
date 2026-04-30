//! # Hermes Agent Core
//! 
//! Agent chính xử lý requests với tool-calling

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::tools::{Tool, ToolCall, ToolResult};
use crate::chat::{Message, MessageRole};
use crate::models::ModelConfig;

/// Agent configuration
#[derive(Debug, Clone)]
pub struct AgentConfig {
    pub model: String,
    pub base_url: String,
    pub api_key: Option<String>,
    pub max_tokens: u32,
    pub temperature: f32,
    pub tools_enabled: bool,
    pub system_prompt: String,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            model: " NousResearch/Hermes-2-Pro-Llama-3-8B".to_string(),
            base_url: "http://localhost:8080".to_string(),
            api_key: None,
            max_tokens: 4096,
            temperature: 0.7,
            tools_enabled: true,
            system_prompt: Self::default_system_prompt(),
        }
    }
}

impl AgentConfig {
    pub fn default_system_prompt() -> String {
        r#"Bạn là một AI agent thông minh, được thiết kế để hỗ trợ doanh nghiệp SME Việt Nam.
        
Bạn có khả năng:
- Trả lời câu hỏi bằng tiếng Việt và tiếng Anh
- Sử dụng các công cụ (tools) khi cần thiết
- Phân tích dữ liệu và đưa ra đề xuất
- Làm việc với các file và tài liệu
- Tìm kiếm thông tin trên internet

Khi được yêu cầu, hãy sử dụng các công cụ có sẵn để hoàn thành tác vụ.
Nếu không có công cụ phù hợp, hãy trả lời dựa trên kiến thức của bạn.

Luôn trả lời một cách hữu ích, chính xác và thân thiện."#.to_string()
    }

    pub fn for_sme_vietnam() -> Self {
        Self {
            model: " NousResearch/Hermes-2-Pro-Llama-3-8B".to_string(),
            base_url: "http://localhost:8080".to_string(),
            api_key: None,
            max_tokens: 4096,
            temperature: 0.7,
            tools_enabled: true,
            system_prompt: Self::sme_vietnam_prompt(),
        }
    }

    pub fn sme_vietnam_prompt() -> String {
        r#"Bạn là một AI assistant chuyên hỗ trợ doanh nghiệp SME Việt Nam.

Nhiệm vụ của bạn:
1. Hỗ trợ quản lý: theo dõi đơn hàng, khách hàng, nhân viên
2. Phân tích kinh doanh: báo cáo doanh thu, lợi nhuận, xu hướng
3. Chăm sóc khách hàng: trả lời câu hỏi, giải quyết khiếu nại
4. Tự động hóa: tạo email, tài liệu, báo cáo

Nguyên tắc:
- Ưu tiên tiếng Việt trong giao tiếp
- Đưa ra các giải pháp thực tế, có thể áp dụng ngay
- Báo cáo rõ ràng, có số liệu cụ thể
- Đề xuất cải tiến dựa trên dữ liệu"#.to_string()
    }
}

/// Agent response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResponse {
    pub content: String,
    pub tool_calls: Option<Vec<ToolCall>>,
    pub session_id: String,
    pub tokens_used: u32,
    pub model: String,
}

/// Hermes Agent
pub struct HermesAgent {
    config: AgentConfig,
    tools: Vec<Box<dyn Tool>>,
    session_id: String,
}

impl HermesAgent {
    pub fn new(config: AgentConfig) -> Self {
        Self {
            config,
            tools: Vec::new(),
            session_id: uuid::Uuid::new_v4().to_string(),
        }
    }

    pub fn with_tools(mut self, tools: Vec<Box<dyn Tool>>) -> Self {
        self.tools = tools;
        self
    }

    pub fn add_tool(&mut self, tool: Box<dyn Tool>) {
        self.tools.push(tool);
    }

    pub async fn chat(&self, message: &str) -> Result<AgentResponse, AgentError> {
        let messages = vec![
            Message::system(&self.config.system_prompt),
            Message::user(message),
        ];

        self.execute(messages).await
    }

    pub async fn execute(&self, messages: Vec<Message>) -> Result<AgentResponse, AgentError> {
        // Build request to Hermes-compatible endpoint
        let request = self.build_request(&messages)?;
        
        // Call API
        let response = self.call_api(&request).await?;
        
        // Parse response
        self.parse_response(response).await
    }

    async fn execute_with_tools(
        &self,
        messages: Vec<Message>,
        max_iterations: u32,
    ) -> Result<AgentResponse, AgentError> {
        let mut current_messages = messages;
        let mut iteration = 0;
        
        loop {
            iteration += 1;
            
            let request = self.build_request(&current_messages)?;
            let response = self.call_api(&request).await?;
            
            let parsed = self.parse_response(response).await?;
            
            // Check if there are tool calls
            if let Some(tool_calls) = parsed.tool_calls.clone() {
                if iteration >= max_iterations {
                    return Ok(AgentResponse {
                        content: format!(
                            "{}...\n[LIMIT REACHED] Đã đạt giới hạn {} iterations",
                            parsed.content, max_iterations
                        ),
                        tool_calls: None,
                        session_id: self.session_id.clone(),
                        tokens_used: parsed.tokens_used,
                        model: self.config.model.clone(),
                    });
                }

                // Execute tool calls
                let tool_results = self.execute_tools(&tool_calls).await;
                
                // Add assistant message and tool results to conversation
                current_messages.push(Message::assistant(&parsed.content));
                
                for (tool_call, result) in tool_calls.iter().zip(tool_results.into_iter()) {
                    current_messages.push(Message::tool(&result.content, &tool_call.id));
                }
            } else {
                // No tool calls, return response
                return Ok(parsed);
            }
        }
    }

    fn build_request(&self, messages: &[Message]) -> Result<ChatCompletionRequest, AgentError> {
        let tools_json = if self.config.tools_enabled && !self.tools.is_empty() {
            Some(self.build_tools_schema()?)
        } else {
            None
        };

        Ok(ChatCompletionRequest {
            model: self.config.model.trim().to_string(),
            messages: messages.iter().map(|m| ChatMessage {
                role: match m.role {
                    MessageRole::System => "system",
                    MessageRole::User => "user",
                    MessageRole::Assistant => "assistant",
                    MessageRole::Tool => "tool",
                }.to_string(),
                content: Some(m.content.clone()),
                tool_calls: None,
            }).collect(),
            temperature: Some(self.config.temperature),
            max_tokens: Some(self.config.max_tokens),
            tools: tools_json,
            stream: Some(false),
        })
    }

    fn build_tools_schema(&self) -> Result<serde_json::Value, AgentError> {
        let tools: Vec<serde_json::Value> = self.tools
            .iter()
            .map(|t| {
                serde_json::json!({
                    "type": "function",
                    "function": {
                        "name": t.name(),
                        "description": t.description(),
                        "parameters": t.parameters(),
                    }
                })
            })
            .collect();

        Ok(serde_json::json!({ "tools": tools }))
    }

    async fn call_api(&self, request: &ChatCompletionRequest) -> Result<String, AgentError> {
        let client = reqwest::Client::new();
        
        let url = format!("{}/v1/chat/completions", self.config.base_url);
        
        let mut req_builder = client.post(&url)
            .header("Content-Type", "application/json");
        
        if let Some(ref api_key) = self.config.api_key {
            req_builder = req_builder.header("Authorization", format!("Bearer {}", api_key));
        }

        let response = req_builder
            .json(request)
            .send()
            .await
            .map_err(|e| AgentError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(AgentError::ApiError(format!(
                "API returned status: {}", response.status()
            )));
        }

        response.text().await.map_err(|e| AgentError::NetworkError(e.to_string()))
    }

    async fn parse_response(&self, response_text: String) -> Result<AgentResponse, AgentError> {
        let response: ApiResponse = serde_json::from_str(&response_text)
            .map_err(|e| AgentError::ParseError(e.to_string()))?;

        let choice = response.choices.into_iter().next()
            .ok_or_else(|| AgentError::ParseError("No choices in response".to_string()))?;

        let tool_calls = if let Some(tc) = choice.message.tool_calls {
            let calls: Vec<ToolCall> = tc.into_iter().map(|tc| ToolCall {
                id: tc.id,
                name: tc.function.name,
                arguments: tc.function.arguments,
            }).collect();
            Some(calls)
        } else {
            None
        };

        let tokens_used = response.usage.map(|u| u.total_tokens).unwrap_or(0);

        Ok(AgentResponse {
            content: choice.message.content.unwrap_or_default(),
            tool_calls,
            session_id: self.session_id.clone(),
            tokens_used,
            model: self.config.model.clone(),
        })
    }

    async fn execute_tools(&self, tool_calls: &[ToolCall]) -> Vec<ToolResult> {
        let mut results = Vec::new();

        for call in tool_calls {
            let result = self.execute_single_tool(call).await;
            results.push(result);
        }

        results
    }

    async fn execute_single_tool(&self, call: &ToolCall) -> ToolResult {
        for tool in &self.tools {
            if tool.name() == call.name {
                match tool.execute(&call.arguments).await {
                    Ok(content) => return ToolResult {
                        tool_call_id: call.id.clone(),
                        content,
                        success: true,
                    },
                    Err(e) => return ToolResult {
                        tool_call_id: call.id.clone(),
                        content: format!("Error: {}", e),
                        success: false,
                    },
                }
            }
        }

        ToolResult {
            tool_call_id: call.id.clone(),
            content: format!("Tool '{}' not found", call.name),
            success: false,
        }
    }

    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    pub fn reset_session(&mut self) {
        self.session_id = uuid::Uuid::new_v4().to_string();
    }
}

// Internal types
#[derive(Debug, Serialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
}

#[derive(Debug, Serialize)]
struct ChatMessage {
    role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Deserialize)]
struct ApiResponse {
    id: String,
    choices: Vec<ApiChoice>,
    #[serde(skip_serializing_if = "Option::is_none")]
    usage: Option<Usage>,
}

#[derive(Debug, Deserialize)]
struct ApiChoice {
    message: ApiMessage,
    finish_reason: String,
}

#[derive(Debug, Deserialize)]
struct ApiMessage {
    content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<ApiToolCall>>,
}

#[derive(Debug, Deserialize)]
struct ApiToolCall {
    id: String,
    #[serde(rename = "type")]
    call_type: String,
    function: ApiFunction,
}

#[derive(Debug, Deserialize)]
struct ApiFunction {
    name: String,
    arguments: String,
}

#[derive(Debug, Deserialize)]
struct Usage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

#[derive(Debug)]
pub enum AgentError {
    NetworkError(String),
    ApiError(String),
    ParseError(String),
    ToolError(String),
}

impl std::fmt::Display for AgentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentError::NetworkError(e) => write!(f, "Network error: {}", e),
            AgentError::ApiError(e) => write!(f, "API error: {}", e),
            AgentError::ParseError(e) => write!(f, "Parse error: {}", e),
            AgentError::ToolError(e) => write!(f, "Tool error: {}", e),
        }
    }
}

impl std::error::Error for AgentError {}
