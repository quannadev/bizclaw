//! Sandbox Configuration

use serde::{Deserialize, Serialize};

/// Sandbox provider type
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum SandboxProvider {
    #[default]
    CubeSandbox,
    E2B,
    Local,
}

/// Sandbox configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    pub provider: SandboxProvider,
    pub api_url: String,
    pub api_key: Option<String>,
    pub timeout_secs: u64,
    pub max_retries: u32,
    pub enable_network: bool,
    pub enable_filesystem: bool,
    pub enable_clipboard: bool,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            provider: SandboxProvider::CubeSandbox,
            api_url: "https://api.cubesandbox.io".to_string(),
            api_key: None,
            timeout_secs: 30,
            max_retries: 3,
            enable_network: false,
            enable_filesystem: true,
            enable_clipboard: true,
        }
    }
}

impl SandboxConfig {
    /// Create CubeSandbox config
    pub fn cube_sandbox(api_key: impl Into<String>) -> Self {
        Self {
            provider: SandboxProvider::CubeSandbox,
            api_key: Some(api_key.into()),
            ..Default::default()
        }
    }

    /// Create E2B config (drop-in replacement)
    pub fn e2b(api_key: impl Into<String>) -> Self {
        Self {
            provider: SandboxProvider::E2B,
            api_url: "https://api.e2b.dev".to_string(),
            api_key: Some(api_key.into()),
            ..Default::default()
        }
    }

    /// Create local sandbox config (for development)
    pub fn local() -> Self {
        Self {
            provider: SandboxProvider::Local,
            api_url: "http://localhost:8080".to_string(),
            ..Default::default()
        }
    }
}
