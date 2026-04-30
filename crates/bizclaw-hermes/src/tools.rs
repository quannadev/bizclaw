//! # Hermes Tools
//! 
//! Tool definitions cho Hermes agent

use serde::{Deserialize, Serialize};
use async_trait::async_trait;
use std::io::Write;

/// Tool call request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: String,
}

/// Tool execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub tool_call_id: String,
    pub content: String,
    pub success: bool,
}

/// Tool trait
#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters(&self) -> serde_json::Value;
    async fn execute(&self, arguments: &str) -> Result<String, String>;
}

/// Hermes built-in tools
pub mod builtin {
    use super::*;
    use std::collections::HashMap;

    /// Search the web for information
    pub struct WebSearchTool {
        api_key: Option<String>,
    }

    impl WebSearchTool {
        pub fn new(api_key: Option<String>) -> Self {
            Self { api_key }
        }
    }

    #[async_trait]
    impl Tool for WebSearchTool {
        fn name(&self) -> &str {
            "web_search"
        }

        fn description(&self) -> &str {
            "Tìm kiếm thông tin trên internet. Input: query (string) - câu hỏi cần tìm kiếm"
        }

        fn parameters(&self) -> serde_json::Value {
            serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Câu hỏi hoặc từ khóa tìm kiếm"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Số lượng kết quả trả về",
                        "default": 5
                    }
                },
                "required": ["query"]
            })
        }

        async fn execute(&self, arguments: &str) -> Result<String, String> {
            let args: serde_json::Value = serde_json::from_str(arguments)
                .map_err(|e| e.to_string())?;
            
            let query = args["query"].as_str().ok_or("Missing query")?;
            let limit = args["limit"].as_i64().unwrap_or(5) as usize;

            // In real implementation, call search API
            Ok(format!(
                "Tìm thấy {} kết quả cho '{}':\n1. Result 1\n2. Result 2\n3. Result 3",
                limit, query
            ))
        }
    }

    /// Calculator tool
    pub struct CalculatorTool;

    impl CalculatorTool {
        pub fn new() -> Self {
            Self
        }
    }

    #[async_trait]
    impl Tool for CalculatorTool {
        fn name(&self) -> &str {
            "calculator"
        }

        fn description(&self) -> &str {
            "Thực hiện phép tính đơn giản. Input: expression (string) - biểu thức toán học"
        }

        fn parameters(&self) -> serde_json::Value {
            serde_json::json!({
                "type": "object",
                "properties": {
                    "expression": {
                        "type": "string",
                        "description": "Biểu thức toán học, ví dụ: 2 + 2, 10 * 5, 100 / 4"
                    }
                },
                "required": ["expression"]
            })
        }

        async fn execute(&self, arguments: &str) -> Result<String, String> {
            let args: serde_json::Value = serde_json::from_str(arguments)
                .map_err(|e| e.to_string())?;
            
            let expr = args["expression"].as_str().ok_or("Missing expression")?;
            
            // Simple expression evaluation
            let result = evaluate_expression(expr)?;
            Ok(format!("{} = {}", expr, result))
        }
    }

    fn evaluate_expression(expr: &str) -> Result<f64, String> {
        let expr = expr.replace(" ", "");
        
        // Handle basic operations
        if let Some(pos) = expr.find('+') {
            let a: f64 = expr[..pos].parse().map_err(|_| "Invalid number")?;
            let b: f64 = expr[pos+1..].parse().map_err(|_| "Invalid number")?;
            return Ok(a + b);
        }
        
        if let Some(pos) = expr.find('-') {
            if pos > 0 {
                let a: f64 = expr[..pos].parse().map_err(|_| "Invalid number")?;
                let b: f64 = expr[pos+1..].parse().map_err(|_| "Invalid number")?;
                return Ok(a - b);
            }
        }
        
        if let Some(pos) = expr.find('*') {
            let a: f64 = expr[..pos].parse().map_err(|_| "Invalid number")?;
            let b: f64 = expr[pos+1..].parse().map_err(|_| "Invalid number")?;
            return Ok(a * b);
        }
        
        if let Some(pos) = expr.find('/') {
            let a: f64 = expr[..pos].parse().map_err(|_| "Invalid number")?;
            let b: f64 = expr[pos+1..].parse().map_err(|_| "Invalid number")?;
            if b == 0.0 {
                return Err("Division by zero".to_string());
            }
            return Ok(a / b);
        }
        
        // Just a number
        expr.parse().map_err(|_| "Invalid expression".to_string())
    }

    /// File operations tool
    pub struct FileTool {
        base_path: String,
    }

    impl FileTool {
        pub fn new(base_path: &str) -> Self {
            Self {
                base_path: base_path.to_string(),
            }
        }
    }

    #[async_trait]
    impl Tool for FileTool {
        fn name(&self) -> &str {
            "file_operations"
        }

        fn description(&self) -> &str {
            "Đọc hoặc ghi file. Operations: read, write, append"
        }

        fn parameters(&self) -> serde_json::Value {
            serde_json::json!({
                "type": "object",
                "properties": {
                    "operation": {
                        "type": "string",
                        "enum": ["read", "write", "append"],
                        "description": "Loại thao tác"
                    },
                    "path": {
                        "type": "string",
                        "description": "Đường dẫn file (relative to base path)"
                    },
                    "content": {
                        "type": "string",
                        "description": "Nội dung cần ghi (cho write/append)"
                    }
                },
                "required": ["operation", "path"]
            })
        }

        async fn execute(&self, arguments: &str) -> Result<String, String> {
            let args: serde_json::Value = serde_json::from_str(arguments)
                .map_err(|e| e.to_string())?;
            
            let operation = args["operation"].as_str().ok_or("Missing operation")?;
            let path = args["path"].as_str().ok_or("Missing path")?;
            
            let full_path = format!("{}/{}", self.base_path, path);
            
            match operation {
                "read" => {
                    std::fs::read_to_string(&full_path)
                        .map_err(|e| e.to_string())
                }
                "write" => {
                    let content = args["content"].as_str().unwrap_or("");
                    std::fs::write(&full_path, content)
                        .map_err(|e| e.to_string())?;
                    Ok(format!("Đã ghi file: {}", path))
                }
                "append" => {
                    let content = args["content"].as_str().unwrap_or("");
                    let mut file = std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(&full_path)
                        .map_err(|e: std::io::Error| e.to_string())?;
                    file.write_all(content.as_bytes())
                        .map_err(|e: std::io::Error| e.to_string())?;
                    Ok(format!("Đã thêm vào file: {}", path))
                }
                _ => Err(format!("Unknown operation: {}", operation))
            }
        }
    }

    /// Database query tool
    pub struct DatabaseTool {
        connection_string: String,
    }

    impl DatabaseTool {
        pub fn new(connection_string: &str) -> Self {
            Self {
                connection_string: connection_string.to_string(),
            }
        }
    }

    #[async_trait]
    impl Tool for DatabaseTool {
        fn name(&self) -> &str {
            "database_query"
        }

        fn description(&self) -> &str {
            "Truy vấn cơ sở dữ liệu SQL"
        }

        fn parameters(&self) -> serde_json::Value {
            serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Câu lệnh SQL (SELECT, INSERT, UPDATE, DELETE)"
                    }
                },
                "required": ["query"]
            })
        }

        async fn execute(&self, arguments: &str) -> Result<String, String> {
            let args: serde_json::Value = serde_json::from_str(arguments)
                .map_err(|e| e.to_string())?;
            
            let query = args["query"].as_str().ok_or("Missing query")?;
            
            // Simplified - in real impl would connect to DB
            Ok(format!(
                "Query executed: {}\nRows affected: 0 (simulated)",
                query
            ))
        }
    }
}

/// HermesTools - Collection of built-in tools
pub struct HermesTools {
    tools: Vec<Box<dyn Tool>>,
}

impl HermesTools {
    pub fn new() -> Self {
        Self { tools: Vec::new() }
    }

    pub fn with_defaults() -> Self {
        let mut tools = Self::new();
        tools.add(Box::new(builtin::CalculatorTool::new()));
        tools.add(Box::new(builtin::WebSearchTool::new(None)));
        tools
    }

    pub fn add(&mut self, tool: Box<dyn Tool>) {
        self.tools.push(tool);
    }

    pub fn get_tools(&self) -> &[Box<dyn Tool>] {
        &self.tools
    }
}

impl Default for HermesTools {
    fn default() -> Self {
        Self::with_defaults()
    }
}
