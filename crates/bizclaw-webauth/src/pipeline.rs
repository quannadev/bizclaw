//! WebAuth Pipeline — orchestrator for browser-based AI providers.
//!
//! The pipeline manages:
//! 1. Chrome/Chromium browser launch and CDP connection
//! 2. Provider initialization and health checking
//! 3. HTTP proxy server lifecycle
//! 4. Periodic auth refresh

use std::path::PathBuf;
use std::process::Command;
use tracing;

use crate::cdp;
use crate::providers;
use crate::proxy::WebAuthProxy;

const LOG_TAG: &str = "[WebAuth-Pipeline]";

/// Main orchestrator for WebAuth.
pub struct WebAuthPipeline {
    /// Chrome debugging port
    cdp_port: u16,
    /// User data directory for Chrome profile
    user_data_dir: PathBuf,
    /// Chrome process handle
    chrome_process: Option<std::process::Child>,
}

impl WebAuthPipeline {
    /// Create a new pipeline with default settings.
    pub fn new() -> Self {
        let user_data_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join("bizclaw")
            .join("webauth-chrome-profile");

        Self {
            cdp_port: 9222,
            user_data_dir,
            chrome_process: None,
        }
    }

    /// Set the CDP debugging port.
    pub fn with_cdp_port(mut self, port: u16) -> Self {
        self.cdp_port = port;
        self
    }

    /// Set the Chrome user data directory.
    pub fn with_user_data_dir(mut self, dir: PathBuf) -> Self {
        self.user_data_dir = dir;
        self
    }

    /// Start the pipeline:
    /// 1. Launch Chrome if needed
    /// 2. Connect CDP
    /// 3. Start HTTP proxy
    ///
    /// Returns the port the proxy is listening on.
    pub async fn start(&mut self, proxy_port: u16) -> Result<u16, String> {
        tracing::info!("{} Starting WebAuth pipeline...", LOG_TAG);

        // 1. Try to connect to existing Chrome, or launch new one
        if cdp::find_chrome_ws_url(self.cdp_port).await.is_err() {
            tracing::info!("{} No Chrome found on port {}, launching...", LOG_TAG, self.cdp_port);
            self.launch_chrome()?;
            // Give Chrome time to start
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
        }

        // 2. Create providers and proxy
        let all_providers = providers::create_all_providers();
        let proxy = WebAuthProxy::new(all_providers, self.cdp_port);

        // 3. Connect CDP
        if let Err(e) = proxy.connect_cdp().await {
            tracing::warn!(
                "{} Could not connect CDP: {}. Proxy will start degraded.",
                LOG_TAG,
                e
            );
        }

        // 4. Start proxy
        let actual_port = proxy.start(proxy_port).await?;

        tracing::info!(
            "{} Pipeline started! Proxy: http://127.0.0.1:{}/v1",
            LOG_TAG,
            actual_port
        );
        tracing::info!(
            "{} To use: set provider to custom:http://127.0.0.1:{}/v1",
            LOG_TAG,
            actual_port
        );

        Ok(actual_port)
    }

    /// Launch a Chrome/Chromium instance with remote debugging.
    fn launch_chrome(&mut self) -> Result<(), String> {
        // Ensure user data dir exists
        std::fs::create_dir_all(&self.user_data_dir)
            .map_err(|e| format!("{} Could not create user data dir: {}", LOG_TAG, e))?;

        let chrome_path = find_chrome_binary()?;

        tracing::info!(
            "{} Launching Chrome: {} (port: {}, profile: {})",
            LOG_TAG,
            chrome_path,
            self.cdp_port,
            self.user_data_dir.display()
        );

        let child = Command::new(&chrome_path)
            .args([
                &format!("--remote-debugging-port={}", self.cdp_port),
                &format!("--user-data-dir={}", self.user_data_dir.display()),
                "--no-first-run",
                "--no-default-browser-check",
                "--disable-background-timer-throttling",
                "--disable-backgrounding-occluded-windows",
                "--disable-renderer-backgrounding",
                // Start with blank page
                "about:blank",
            ])
            .spawn()
            .map_err(|e| format!("{} Could not launch Chrome at '{}': {}", LOG_TAG, chrome_path, e))?;

        tracing::info!("{} Chrome launched (PID: {})", LOG_TAG, child.id());
        self.chrome_process = Some(child);
        Ok(())
    }

    /// Stop the pipeline and cleanup.
    pub fn stop(&mut self) {
        if let Some(ref mut child) = self.chrome_process {
            tracing::info!("{} Stopping Chrome...", LOG_TAG);
            let _ = child.kill();
            let _ = child.wait();
            tracing::info!("{} Chrome stopped", LOG_TAG);
        }
        self.chrome_process = None;
    }

    /// Get the proxy base URL.
    pub fn proxy_url(&self, port: u16) -> String {
        format!("http://127.0.0.1:{}/v1", port)
    }
}

impl Default for WebAuthPipeline {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for WebAuthPipeline {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Find the Chrome/Chromium binary on the system.
fn find_chrome_binary() -> Result<String, String> {
    // macOS paths
    let candidates = [
        "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
        "/Applications/Chromium.app/Contents/MacOS/Chromium",
        "/Applications/Google Chrome Canary.app/Contents/MacOS/Google Chrome Canary",
        "/Applications/Brave Browser.app/Contents/MacOS/Brave Browser",
        "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge",
        // Linux paths
        "/usr/bin/google-chrome",
        "/usr/bin/google-chrome-stable",
        "/usr/bin/chromium",
        "/usr/bin/chromium-browser",
        // Windows
        "C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe",
        "C:\\Program Files (x86)\\Google\\Chrome\\Application\\chrome.exe",
    ];

    for path in &candidates {
        if std::path::Path::new(path).exists() {
            return Ok(path.to_string());
        }
    }

    // Try which/where
    if let Ok(output) = Command::new("which").arg("google-chrome").output() {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() {
                return Ok(path);
            }
        }
    }

    if let Ok(output) = Command::new("which").arg("chromium").output() {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() {
                return Ok(path);
            }
        }
    }

    Err(format!(
        "{} Could not find Chrome/Chromium. Install Google Chrome or set CHROME_PATH.",
        LOG_TAG
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_creation() {
        let pipeline = WebAuthPipeline::new();
        assert_eq!(pipeline.cdp_port, 9222);
        assert!(pipeline.user_data_dir.to_string_lossy().contains("bizclaw"));
    }

    #[test]
    fn test_pipeline_with_port() {
        let pipeline = WebAuthPipeline::new().with_cdp_port(9333);
        assert_eq!(pipeline.cdp_port, 9333);
    }

    #[test]
    fn test_proxy_url() {
        let pipeline = WebAuthPipeline::new();
        assert_eq!(
            pipeline.proxy_url(8080),
            "http://127.0.0.1:8080/v1"
        );
    }

    #[test]
    fn test_find_chrome_binary() {
        // This test will pass on macOS with Chrome installed
        // and fail gracefully on CI without Chrome
        let result = find_chrome_binary();
        // Don't assert success — Chrome might not be installed in CI
        if let Ok(path) = &result {
            assert!(!path.is_empty());
        }
    }
}
