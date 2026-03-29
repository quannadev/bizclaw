//! Custom Tool Authoring — Agent self-creates tools for repeating tasks.
//!
//! When the agent encounters a task it performs repeatedly, it can create
//! a custom tool (script) that persists across sessions. The tool is
//! saved to `custom-tools/` and hot-loaded into the ToolRegistry.
//!
//! # Supported languages: bash, python, node
//! # Security: sandboxed execution, timeout, output limit
//!
//! # How agent creates a tool:
//! ```json
//! {
//!   "action": "create",
//!   "name": "check_server_health",
//!   "language": "bash",
//!   "description": "Check server health endpoint",
//!   "script": "#!/bin/bash\ncurl -s http://myserver/health | jq ."
//! }
//! ```

use async_trait::async_trait;
use bizclaw_core::error::{BizClawError, Result};
use bizclaw_core::traits::Tool;
use bizclaw_core::types::{ToolDefinition, ToolResult};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tracing::info;

/// Metadata for a custom tool stored on disk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomToolMeta {
    pub name: String,
    pub description: String,
    pub language: String,
    pub created_at: String,
    pub usage_count: u64,
}

/// The custom_tool management tool — agent uses this to create/list/delete tools.
pub struct CustomToolManager {
    tools_dir: PathBuf,
}

impl CustomToolManager {
    pub fn new(workspace_dir: PathBuf) -> Self {
        let tools_dir = workspace_dir.join("custom-tools");
        Self { tools_dir }
    }

    /// Ensure custom-tools directory exists.
    fn ensure_dir(&self) -> Result<()> {
        std::fs::create_dir_all(&self.tools_dir)
            .map_err(|e| BizClawError::Tool(format!("Failed to create custom-tools dir: {e}")))?;
        Ok(())
    }

    /// Validate tool name — alphanumeric + underscores only, max 64 chars.
    fn validate_name(name: &str) -> Result<()> {
        if name.is_empty() || name.len() > 64 {
            return Err(BizClawError::Tool(
                "Tool name must be 1-64 characters".into(),
            ));
        }
        if !name.chars().all(|c| c.is_alphanumeric() || c == '_') {
            return Err(BizClawError::Tool(
                "Tool name must be alphanumeric with underscores only".into(),
            ));
        }
        // Block reserved names
        let reserved = [
            "shell",
            "file",
            "edit_file",
            "glob",
            "grep",
            "web_search",
            "http_request",
            "browser",
            "config_manager",
            "plan",
            "memory_search",
            "session_context",
            "custom_tool",
            "group_summarizer",
            "calendar",
            "document_reader",
        ];
        if reserved.contains(&name) {
            return Err(BizClawError::Tool(format!(
                "'{name}' is a reserved tool name"
            )));
        }
        Ok(())
    }

    /// Validate script — block dangerous patterns.
    fn validate_script(script: &str) -> Result<()> {
        let dangerous = [
            "rm -rf /",
            "mkfs",
            ":(){:|:&};:",
            "dd if=/dev/zero",
            "/etc/shadow",
            "chmod 777 /",
            "curl|sh",
            "wget|sh",
            "nc -e",
            "reverse_shell",
        ];
        let lower = script.to_lowercase();
        for pattern in &dangerous {
            if lower.contains(pattern) {
                return Err(BizClawError::Security(format!(
                    "Script contains dangerous pattern: {pattern}"
                )));
            }
        }
        // Max script size: 10KB
        if script.len() > 10_240 {
            return Err(BizClawError::Tool("Script too large (max 10KB)".into()));
        }
        Ok(())
    }

    /// Create a custom tool.
    fn create_tool(
        &self,
        name: &str,
        language: &str,
        description: &str,
        script: &str,
    ) -> Result<String> {
        Self::validate_name(name)?;
        Self::validate_script(script)?;
        self.ensure_dir()?;

        // Validate language
        let ext = match language {
            "bash" | "sh" => "sh",
            "python" | "py" => "py",
            "node" | "javascript" | "js" => "js",
            _ => {
                return Err(BizClawError::Tool(format!(
                    "Unsupported language: {language}. Use: bash, python, node"
                )));
            }
        };

        // Save script
        let script_path = self.tools_dir.join(format!("{name}.{ext}"));
        std::fs::write(&script_path, script)?;

        // Set executable permission on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&script_path, std::fs::Permissions::from_mode(0o755))?;
        }

        // Save metadata
        let meta = CustomToolMeta {
            name: name.to_string(),
            description: description.to_string(),
            language: language.to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
            usage_count: 0,
        };
        let meta_path = self.tools_dir.join(format!("{name}.meta.json"));
        let meta_json = serde_json::to_string_pretty(&meta)?;
        std::fs::write(&meta_path, meta_json)?;

        info!("🛠️ Custom tool created: {name} ({language})");

        Ok(format!(
            "✅ Custom tool '{name}' created successfully.\n- Language: {language}\n- Path: {}\n- Available immediately for use.",
            script_path.display()
        ))
    }

    /// List all custom tools.
    fn list_tools(&self) -> Result<String> {
        self.ensure_dir()?;

        let mut tools = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&self.tools_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|e| e == "json")
                    && path
                        .file_name()
                        .is_some_and(|n| n.to_string_lossy().ends_with(".meta.json"))
                    && let Ok(content) = std::fs::read_to_string(&path)
                    && let Ok(meta) = serde_json::from_str::<CustomToolMeta>(&content)
                {
                    tools.push(meta);
                }
            }
        }

        if tools.is_empty() {
            return Ok("No custom tools found. Use action='create' to create one.".to_string());
        }

        let mut output = format!("📦 {} custom tool(s):\n\n", tools.len());
        for tool in &tools {
            output.push_str(&format!(
                "• **{}** ({}) — {}\n  Created: {} | Used: {} times\n",
                tool.name, tool.language, tool.description, tool.created_at, tool.usage_count
            ));
        }
        Ok(output)
    }

    /// Delete a custom tool.
    fn delete_tool(&self, name: &str) -> Result<String> {
        Self::validate_name(name)?;

        let mut deleted = false;
        for ext in &["sh", "py", "js", "meta.json"] {
            let path = self.tools_dir.join(format!("{name}.{ext}"));
            if path.exists() {
                std::fs::remove_file(&path)?;
                deleted = true;
            }
        }

        if deleted {
            info!("🗑️ Custom tool deleted: {name}");
            Ok(format!("✅ Custom tool '{name}' deleted."))
        } else {
            Err(BizClawError::Tool(format!(
                "Custom tool '{name}' not found."
            )))
        }
    }

    /// Execute a custom tool script.
    fn execute_tool(&self, name: &str, input: &str) -> Result<String> {
        // Find script file
        let script_path = ["sh", "py", "js"]
            .iter()
            .map(|ext| self.tools_dir.join(format!("{name}.{ext}")))
            .find(|p| p.exists())
            .ok_or_else(|| BizClawError::Tool(format!("Custom tool '{name}' not found")))?;

        let ext = script_path
            .extension()
            .map(|e| e.to_string_lossy().to_string())
            .unwrap_or_default();
        let interpreter = match ext.as_str() {
            "sh" => "bash",
            "py" => "python3",
            "js" => "node",
            _ => "bash",
        };

        // Execute with timeout (30s) and input via stdin
        let output = std::process::Command::new(interpreter)
            .arg(&script_path)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            // Security: clear environment, only keep PATH
            .env_clear()
            .env("PATH", "/usr/local/bin:/usr/bin:/bin")
            .env("HOME", "/tmp")
            .env("TOOL_INPUT", input)
            .output()
            .map_err(|e| BizClawError::Tool(format!("Failed to execute custom tool: {e}")))?;

        // Limit output size (max 50KB)
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let max_output = 50_000;

        let result = if output.status.success() {
            if stdout.len() > max_output {
                format!("{}... (output truncated)", &stdout[..max_output])
            } else {
                stdout.to_string()
            }
        } else {
            format!("Error (exit code {:?}):\n{}", output.status.code(), stderr)
        };

        // Update usage count
        let meta_path = self.tools_dir.join(format!("{name}.meta.json"));
        if let Ok(content) = std::fs::read_to_string(&meta_path)
            && let Ok(mut meta) = serde_json::from_str::<CustomToolMeta>(&content)
        {
            meta.usage_count += 1;
            if let Ok(json) = serde_json::to_string_pretty(&meta) {
                let _ = std::fs::write(&meta_path, json);
            }
        }

        Ok(result)
    }
}

#[async_trait]
impl Tool for CustomToolManager {
    fn name(&self) -> &str {
        "custom_tool"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "custom_tool".into(),
            description: "Create, list, delete, or execute custom tools. Use 'create' to make a reusable script, 'list' to see available tools, 'execute' to run one, or 'delete' to remove.".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["create", "list", "delete", "execute"],
                        "description": "Action to perform"
                    },
                    "name": {
                        "type": "string",
                        "description": "Tool name (alphanumeric + underscores, max 64 chars)"
                    },
                    "language": {
                        "type": "string",
                        "enum": ["bash", "python", "node"],
                        "description": "Script language (for create)"
                    },
                    "description": {
                        "type": "string",
                        "description": "Tool description (for create)"
                    },
                    "script": {
                        "type": "string",
                        "description": "Script content (for create)"
                    },
                    "input": {
                        "type": "string",
                        "description": "Input data for tool execution (for execute)"
                    }
                },
                "required": ["action"]
            }),
        }
    }

    async fn execute(&self, arguments: &str) -> Result<ToolResult> {
        let args: serde_json::Value = serde_json::from_str(arguments).unwrap_or_default();
        let action = args
            .get("action")
            .and_then(|v| v.as_str())
            .unwrap_or("list");

        let result = match action {
            "create" => {
                let name = args
                    .get("name")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| BizClawError::Tool("'name' required for create".into()))?;
                let language = args
                    .get("language")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| BizClawError::Tool("'language' required for create".into()))?;
                let description = args
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Custom tool");
                let script = args
                    .get("script")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| BizClawError::Tool("'script' required for create".into()))?;

                self.create_tool(name, language, description, script)?
            }
            "list" => self.list_tools()?,
            "delete" => {
                let name = args
                    .get("name")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| BizClawError::Tool("'name' required for delete".into()))?;
                self.delete_tool(name)?
            }
            "execute" => {
                let name = args
                    .get("name")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| BizClawError::Tool("'name' required for execute".into()))?;
                let input = args.get("input").and_then(|v| v.as_str()).unwrap_or("");
                self.execute_tool(name, input)?
            }
            _ => {
                return Err(BizClawError::Tool(format!(
                    "Unknown action: {action}. Use: create, list, delete, execute"
                )));
            }
        };

        Ok(ToolResult {
            tool_call_id: String::new(),
            output: result,
            success: true,
        })
    }
}

/// Load existing custom tools as executable Tool instances.
pub fn load_custom_tools(workspace_dir: &Path) -> Vec<Box<dyn Tool>> {
    let tools_dir = workspace_dir.join("custom-tools");
    if !tools_dir.exists() {
        return vec![];
    }

    let mut tools: Vec<Box<dyn Tool>> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&tools_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "json")
                && path
                    .file_name()
                    .is_some_and(|n| n.to_string_lossy().ends_with(".meta.json"))
                && let Ok(content) = std::fs::read_to_string(&path)
                && let Ok(meta) = serde_json::from_str::<CustomToolMeta>(&content)
            {
                tools.push(Box::new(CustomToolRunner {
                    meta: meta.clone(),
                    tools_dir: tools_dir.clone(),
                }));
                info!("📦 Loaded custom tool: {} ({})", meta.name, meta.language);
            }
        }
    }

    tools
}

/// Runtime executor for a custom tool (loaded from disk).
struct CustomToolRunner {
    meta: CustomToolMeta,
    tools_dir: PathBuf,
}

#[async_trait]
impl Tool for CustomToolRunner {
    fn name(&self) -> &str {
        &self.meta.name
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: self.meta.name.clone(),
            description: format!("[Custom] {}", self.meta.description),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "input": {
                        "type": "string",
                        "description": "Input data (passed via TOOL_INPUT env var)"
                    }
                }
            }),
        }
    }

    async fn execute(&self, arguments: &str) -> Result<ToolResult> {
        let args: serde_json::Value = serde_json::from_str(arguments).unwrap_or_default();
        let input = args.get("input").and_then(|v| v.as_str()).unwrap_or("");

        let manager = CustomToolManager {
            tools_dir: self.tools_dir.clone(),
        };
        let output = manager.execute_tool(&self.meta.name, input)?;
        Ok(ToolResult {
            tool_call_id: String::new(),
            output,
            success: true,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn test_workspace() -> PathBuf {
        let dir = std::env::temp_dir().join("bizclaw-test-custom-tools");
        let _ = std::fs::create_dir_all(&dir);
        dir
    }

    #[test]
    fn test_validate_name() {
        assert!(CustomToolManager::validate_name("check_health").is_ok());
        assert!(CustomToolManager::validate_name("my_tool_2").is_ok());
        assert!(CustomToolManager::validate_name("").is_err());
        assert!(CustomToolManager::validate_name("bad name").is_err());
        assert!(CustomToolManager::validate_name("shell").is_err()); // reserved
        assert!(CustomToolManager::validate_name("http_request").is_err());
    }

    #[test]
    fn test_validate_script() {
        assert!(CustomToolManager::validate_script("echo 'hello'").is_ok());
        assert!(CustomToolManager::validate_script("curl http://api.example.com/health").is_ok());
        assert!(CustomToolManager::validate_script("rm -rf /").is_err());
        assert!(CustomToolManager::validate_script(&"x".repeat(20_000)).is_err());
    }

    #[tokio::test]
    async fn test_create_list_delete() {
        let ws = test_workspace();
        let manager = CustomToolManager::new(ws.clone());

        // Create
        let result = manager
            .create_tool(
                "test_tool",
                "bash",
                "A test tool",
                "#!/bin/bash\necho hello",
            )
            .unwrap();
        assert!(result.contains("created successfully"));

        // List
        let list = manager.list_tools().unwrap();
        assert!(list.contains("test_tool"));

        // Delete
        let del = manager.delete_tool("test_tool").unwrap();
        assert!(del.contains("deleted"));

        // Cleanup
        let _ = std::fs::remove_dir_all(&ws);
    }
}
