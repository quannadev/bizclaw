//! # BizClaw Tools
//! Built-in tool execution system.

pub mod shell;
pub mod file;
pub mod registry;

use bizclaw_core::traits::Tool;

/// Tool registry — manages available tools.
pub struct ToolRegistry {
    tools: Vec<Box<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self { tools: vec![] }
    }

    pub fn register(&mut self, tool: Box<dyn Tool>) {
        self.tools.push(tool);
    }

    pub fn get(&self, name: &str) -> Option<&dyn Tool> {
        self.tools.iter().find(|t| t.name() == name).map(|t| t.as_ref())
    }

    pub fn list(&self) -> Vec<bizclaw_core::types::ToolDefinition> {
        self.tools.iter().map(|t| t.definition()).collect()
    }

    /// Create registry with default tools.
    pub fn with_defaults() -> Self {
        let mut reg = Self::new();
        reg.register(Box::new(shell::ShellTool::new()));
        reg.register(Box::new(file::FileTool::new()));
        reg
    }
}

impl Default for ToolRegistry {
    fn default() -> Self { Self::with_defaults() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_with_defaults() {
        let reg = ToolRegistry::with_defaults();
        assert!(reg.get("shell").is_some());
        assert!(reg.get("file").is_some());
        assert!(reg.get("nonexistent").is_none());
    }

    #[test]
    fn test_registry_list() {
        let reg = ToolRegistry::with_defaults();
        let defs = reg.list();
        assert!(defs.len() >= 2);
        assert!(defs.iter().any(|d| d.name == "shell"));
        assert!(defs.iter().any(|d| d.name == "file"));
    }

    #[test]
    fn test_registry_empty() {
        let reg = ToolRegistry::new();
        assert!(reg.list().is_empty());
        assert!(reg.get("shell").is_none());
    }
}
