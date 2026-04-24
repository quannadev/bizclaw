//! CubeSandbox Executor Implementation
//!
//! E2B-compatible API for seamless migration

use crate::{
    config::SandboxConfig,
    Language, ExecutionResult, FixResult, SandboxInstance, SandboxError, SandboxProvider,
};
use reqwest::Client;
use serde_json::json;
use std::time::{Duration, Instant};
use tracing::{debug, error, info, warn};

/// Main sandbox executor
pub struct SandboxExecutor {
    config: SandboxConfig,
    client: Client,
}

impl SandboxExecutor {
    /// Create new executor with config
    pub fn new(config: SandboxConfig) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .build()
            .expect("Failed to create HTTP client");

        Self { config, client }
    }

    /// Create from environment variables
    pub fn from_env() -> Result<Self, SandboxError> {
        let api_key = std::env::var("CUBESANDBOX_API_KEY")
            .or_else(|_| std::env::var("E2B_API_KEY"))
            .map_err(|_| SandboxError::InvalidRequest("Missing CUBESANDBOX_API_KEY or E2B_API_KEY".into()));

        match api_key {
            Ok(key) => {
                let config = SandboxConfig::cube_sandbox(key);
                Ok(Self::new(config))
            }
            Err(e) => Err(e),
        }
    }

    /// Execute code in sandbox
    pub async fn execute(
        &self,
        code: &str,
        language: Language,
    ) -> Result<ExecutionResult, SandboxError> {
        let start = Instant::now();
        info!("Executing {} code ({} bytes)", language.as_str(), code.len());

        let result = match self.config.provider {
            SandboxProvider::CubeSandbox | SandboxProvider::E2B => {
                self.execute_remote(code, language).await
            }
            SandboxProvider::Local => {
                self.execute_local(code, language).await
            }
        };

        match result {
            Ok(mut result) => {
                result.runtime_ms = start.elapsed().as_millis() as u64;
                debug!("Execution completed in {}ms, exit_code={}", result.runtime_ms, result.exit_code);
                Ok(result)
            }
            Err(e) => Err(e),
        }
    }

    /// Execute with auto-fix loop
    pub async fn execute_with_fix(
        &self,
        initial_code: &str,
        language: Language,
        max_attempts: u32,
    ) -> Result<FixResult, SandboxError> {
        let mut current_code = initial_code.to_string();
        let mut attempts = 0;
        let mut last_error: Option<String> = None;

        loop {
            attempts += 1;
            info!("Fix attempt {}/{}", attempts, max_attempts);

            match self.execute(&current_code, language.clone()).await {
                Ok(result) if result.success => {
                    return Ok(FixResult {
                        original_code: initial_code.to_string(),
                        fixed_code: current_code,
                        attempts,
                        final_result: Some(result),
                        error: None,
                    });
                }
                Ok(result) => {
                    warn!("Execution failed: {}", result.stderr);
                    last_error = Some(result.stderr.clone());
                }
                Err(e) => {
                    warn!("Execution error: {}", e);
                    last_error = Some(e.to_string());
                }
            }

            if attempts >= max_attempts {
                break;
            }

            current_code = self.fix_code(&current_code, last_error.as_deref().unwrap_or("Unknown error")).await?;
        }

        Ok(FixResult {
            original_code: initial_code.to_string(),
            fixed_code: current_code,
            attempts,
            final_result: None,
            error: last_error,
        })
    }

    /// Auto-fix code based on error (placeholder - integrate with LLM)
    async fn fix_code(&self, code: &str, error: &str) -> Result<String, SandboxError> {
        info!("Asking LLM to fix code... (placeholder)");
        
        #[allow(unused_variables)]
        let prompt = format!(
            r#"Fix this code. Error: {}
            
Original code:
```{}
```

Return ONLY the fixed code, no explanation.
"#,
            error, code
        );

        Ok(code.to_string())
    }

    /// Execute on remote sandbox (CubeSandbox/E2B compatible)
    async fn execute_remote(
        &self,
        code: &str,
        language: Language,
    ) -> Result<ExecutionResult, SandboxError> {
        let api_key = self.config.api_key.as_ref()
            .ok_or_else(|| SandboxError::InvalidRequest("API key required".into()))?;

        let request = json!({
            "code": code,
            "language": language.as_str(),
            "enable_network": self.config.enable_network,
            "enable_filesystem": self.config.enable_filesystem,
        });

        let response = self.client
            .post(format!("{}/v1/sandbox/run", self.config.api_url))
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| SandboxError::ConnectionFailed(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            error!("Sandbox error: {} - {}", status, body);
            return Err(SandboxError::ExecutionFailed(format!("{}: {}", status, body)));
        }

        let result: serde_json::Value = response.json().await
            .map_err(|e| SandboxError::ExecutionFailed(e.to_string()))?;

        Ok(ExecutionResult {
            id: result["id"].as_str().unwrap_or("unknown").to_string(),
            stdout: result["stdout"].as_str().unwrap_or("").to_string(),
            stderr: result["stderr"].as_str().unwrap_or("").to_string(),
            exit_code: result["exit_code"].as_i64().unwrap_or(-1) as i32,
            runtime_ms: result["runtime_ms"].as_u64().unwrap_or(0),
            memory_mb: result["memory_mb"].as_f64().map(|m| m as f32),
            network_enabled: result["network_enabled"].as_bool().unwrap_or(false),
            success: result["exit_code"].as_i64().unwrap_or(-1) == 0,
        })
    }

    /// Execute locally (for development)
    async fn execute_local(
        &self,
        code: &str,
        language: Language,
    ) -> Result<ExecutionResult, SandboxError> {
        warn!("Local sandbox mode - executing directly (NOT SAFE for production!)");

        Ok(ExecutionResult {
            id: format!("local-{}", uuid::Uuid::new_v4()),
            stdout: format!("[Local mode] Would execute {} code:\n{}", language.as_str(), code),
            stderr: String::new(),
            exit_code: 0,
            runtime_ms: 0,
            memory_mb: None,
            network_enabled: false,
            success: true,
        })
    }

    /// Create persistent sandbox instance
    #[allow(dead_code)]
    pub async fn create_instance(
        &self,
        language: Language,
    ) -> Result<SandboxInstance, SandboxError> {
        let api_key = self.config.api_key.as_ref()
            .ok_or_else(|| SandboxError::InvalidRequest("API key required".into()))?;

        let request = json!({
            "language": language.as_str(),
        });

        let response = self.client
            .post(format!("{}/v1/sandbox/create", self.config.api_url))
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&request)
            .send()
            .await
            .map_err(|e| SandboxError::ConnectionFailed(e.to_string()))?;

        let result: serde_json::Value = response.json().await
            .map_err(|e| SandboxError::ExecutionFailed(e.to_string()))?;

        Ok(SandboxInstance {
            id: result["sandbox_id"].as_str().unwrap_or("unknown").to_string(),
            created_at: chrono::Utc::now().timestamp(),
            language,
            hostname: result["hostname"].as_str().unwrap_or("local").to_string(),
            network_id: result["network_id"].as_str().map(String::from),
        })
    }
}

impl Default for SandboxExecutor {
    fn default() -> Self {
        Self::new(SandboxConfig::default())
    }
}
