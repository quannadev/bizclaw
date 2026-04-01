//! Security Policy trait.

use crate::error::Result;
use async_trait::async_trait;

/// Security Policy trait — validates commands and file access.
#[async_trait]
pub trait SecurityPolicy: Send + Sync {
    /// Check if a command is allowed to execute.
    async fn check_command(&self, command: &str) -> Result<bool>;

    /// Check if a file path is accessible.
    async fn check_path(&self, path: &str) -> Result<bool>;

    /// Check if a specific tool is allowed to execute based on granular permissions.
    async fn check_tool(&self, tool_name: &str) -> Result<bool>;

    /// Get the autonomy level.
    fn autonomy_level(&self) -> &str;
}
