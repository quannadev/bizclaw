//! Skill Gating — auto-detect system capabilities before loading skills.
//!
//! Ported from smallnest/goclaw's gating system.
//! Skills can declare requirements (binaries, env vars, features) in their metadata.
//! The gating system checks if requirements are met before loading the skill.
//!
//! ## SKILL.md Gating Format
//! ```yaml
//! ---
//! name: docker-deploy
//! description: Deploy with Docker
//! gating:
//!   bins: [docker, docker-compose]
//!   env: [DOCKER_HOST]
//!   features: [container-runtime]
//! ---
//! ```

use std::collections::HashMap;
use std::process::Command;
use tracing::debug;

/// Gating requirements for a skill.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct GatingRequirements {
    /// Required binaries (checked via `which`/`where`).
    #[serde(default)]
    pub bins: Vec<String>,
    /// Required environment variables (must be set and non-empty).
    #[serde(default)]
    pub env: Vec<String>,
    /// Required features (custom feature flags).
    #[serde(default)]
    pub features: Vec<String>,
    /// Required files (must exist).
    #[serde(default)]
    pub files: Vec<String>,
    /// Minimum OS (linux, macos, windows). Empty = any.
    #[serde(default)]
    pub os: Vec<String>,
}

impl GatingRequirements {
    /// Check if all requirements are empty (no gating needed).
    pub fn is_empty(&self) -> bool {
        self.bins.is_empty()
            && self.env.is_empty()
            && self.features.is_empty()
            && self.files.is_empty()
            && self.os.is_empty()
    }
}

/// Result of a gating check.
#[derive(Debug, Clone, serde::Serialize)]
pub struct GatingResult {
    /// Whether all requirements are met.
    pub passed: bool,
    /// Which requirements failed.
    pub failures: Vec<GatingFailure>,
    /// Cached capability info.
    pub capabilities: HashMap<String, bool>,
}

/// A single gating failure.
#[derive(Debug, Clone, serde::Serialize)]
pub struct GatingFailure {
    pub requirement_type: String,
    pub name: String,
    pub reason: String,
}

/// System capability checker — caches `which` results.
pub struct GatingChecker {
    /// Cached binary existence checks.
    bin_cache: HashMap<String, bool>,
    /// Custom feature flags (set by config or runtime).
    features: HashMap<String, bool>,
}

impl GatingChecker {
    /// Create a new checker.
    pub fn new() -> Self {
        Self {
            bin_cache: HashMap::new(),
            features: HashMap::new(),
        }
    }

    /// Create with custom feature flags.
    pub fn with_features(features: Vec<String>) -> Self {
        let mut checker = Self::new();
        for f in features {
            checker.features.insert(f, true);
        }
        checker
    }

    /// Register a custom feature flag.
    pub fn set_feature(&mut self, name: impl Into<String>, enabled: bool) {
        self.features.insert(name.into(), enabled);
    }

    /// Check if a binary exists on PATH.
    pub fn has_binary(&mut self, name: &str) -> bool {
        if let Some(&cached) = self.bin_cache.get(name) {
            return cached;
        }

        let exists = check_binary_exists(name);
        self.bin_cache.insert(name.to_string(), exists);

        if exists {
            debug!("🔍 Gating: binary '{}' → found", name);
        } else {
            debug!("🔍 Gating: binary '{}' → NOT found", name);
        }

        exists
    }

    /// Check if an environment variable is set and non-empty.
    pub fn has_env(&self, name: &str) -> bool {
        std::env::var(name)
            .ok()
            .filter(|v| !v.is_empty())
            .is_some()
    }

    /// Check if a feature flag is enabled.
    pub fn has_feature(&self, name: &str) -> bool {
        self.features.get(name).copied().unwrap_or(false)
    }

    /// Check if a file exists.
    pub fn has_file(&self, path: &str) -> bool {
        let expanded = shellexpand::tilde(path).to_string();
        std::path::Path::new(&expanded).exists()
    }

    /// Check if the current OS matches.
    pub fn is_os(&self, os: &str) -> bool {
        let current = std::env::consts::OS;
        match os.to_lowercase().as_str() {
            "linux" => current == "linux",
            "macos" | "darwin" | "mac" => current == "macos",
            "windows" | "win" => current == "windows",
            _ => false,
        }
    }

    /// Run a full gating check against requirements.
    pub fn check(&mut self, reqs: &GatingRequirements) -> GatingResult {
        if reqs.is_empty() {
            return GatingResult {
                passed: true,
                failures: Vec::new(),
                capabilities: HashMap::new(),
            };
        }

        let mut failures = Vec::new();
        let mut capabilities = HashMap::new();

        // Check binaries
        for bin in &reqs.bins {
            let exists = self.has_binary(bin);
            capabilities.insert(format!("bin:{}", bin), exists);
            if !exists {
                failures.push(GatingFailure {
                    requirement_type: "binary".into(),
                    name: bin.clone(),
                    reason: format!("Binary '{}' not found on PATH", bin),
                });
            }
        }

        // Check env vars
        for env_var in &reqs.env {
            let exists = self.has_env(env_var);
            capabilities.insert(format!("env:{}", env_var), exists);
            if !exists {
                failures.push(GatingFailure {
                    requirement_type: "env".into(),
                    name: env_var.clone(),
                    reason: format!("Environment variable '{}' not set", env_var),
                });
            }
        }

        // Check features
        for feature in &reqs.features {
            let exists = self.has_feature(feature);
            capabilities.insert(format!("feature:{}", feature), exists);
            if !exists {
                failures.push(GatingFailure {
                    requirement_type: "feature".into(),
                    name: feature.clone(),
                    reason: format!("Feature '{}' not enabled", feature),
                });
            }
        }

        // Check files
        for file in &reqs.files {
            let exists = self.has_file(file);
            capabilities.insert(format!("file:{}", file), exists);
            if !exists {
                failures.push(GatingFailure {
                    requirement_type: "file".into(),
                    name: file.clone(),
                    reason: format!("File '{}' not found", file),
                });
            }
        }

        // Check OS
        if !reqs.os.is_empty() {
            let os_match = reqs.os.iter().any(|os| self.is_os(os));
            capabilities.insert("os".into(), os_match);
            if !os_match {
                failures.push(GatingFailure {
                    requirement_type: "os".into(),
                    name: std::env::consts::OS.into(),
                    reason: format!(
                        "OS '{}' not in required: {:?}",
                        std::env::consts::OS,
                        reqs.os
                    ),
                });
            }
        }

        let passed = failures.is_empty();
        GatingResult {
            passed,
            failures,
            capabilities,
        }
    }

    /// Get a summary of system capabilities (for dashboard).
    pub fn system_capabilities(&mut self) -> HashMap<String, bool> {
        let common_bins = [
            "git", "docker", "python3", "node", "cargo", "go", "curl", "wget",
            "ffmpeg", "chromium", "chrome",
        ];

        let mut caps = HashMap::new();
        for bin in &common_bins {
            caps.insert(format!("bin:{}", bin), self.has_binary(bin));
        }

        caps.insert(
            format!("os:{}", std::env::consts::OS),
            true,
        );

        caps
    }
}

impl Default for GatingChecker {
    fn default() -> Self {
        Self::new()
    }
}

/// Check if a binary exists on PATH using `which` (Unix) or `where` (Windows).
fn check_binary_exists(name: &str) -> bool {
    #[cfg(unix)]
    {
        Command::new("which")
            .arg(name)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
    #[cfg(windows)]
    {
        Command::new("where")
            .arg(name)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
    #[cfg(not(any(unix, windows)))]
    {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_gating_passes() {
        let mut checker = GatingChecker::new();
        let reqs = GatingRequirements::default();
        let result = checker.check(&reqs);
        assert!(result.passed);
        assert!(result.failures.is_empty());
    }

    #[test]
    fn test_binary_check() {
        let mut checker = GatingChecker::new();
        // `ls` should exist on Unix
        #[cfg(unix)]
        assert!(checker.has_binary("ls"));
        // Nonexistent binary
        assert!(!checker.has_binary("zzzz_nonexistent_binary_012345"));
    }

    #[test]
    fn test_env_check() {
        let checker = GatingChecker::new();
        // PATH is always set
        assert!(checker.has_env("PATH"));
        // Random var shouldn't be set
        assert!(!checker.has_env("BIZCLAW_TEST_NONEXISTENT_VAR_XYZ"));
    }

    #[test]
    fn test_feature_check() {
        let mut checker = GatingChecker::with_features(vec!["gpu".into(), "docker".into()]);
        assert!(checker.has_feature("gpu"));
        assert!(checker.has_feature("docker"));
        assert!(!checker.has_feature("kubernetes"));
    }

    #[test]
    fn test_os_check() {
        let checker = GatingChecker::new();
        #[cfg(target_os = "macos")]
        {
            assert!(checker.is_os("macos"));
            assert!(checker.is_os("darwin"));
            assert!(checker.is_os("mac"));
            assert!(!checker.is_os("linux"));
        }
        #[cfg(target_os = "linux")]
        {
            assert!(checker.is_os("linux"));
            assert!(!checker.is_os("macos"));
        }
    }

    #[test]
    fn test_gating_with_failures() {
        let mut checker = GatingChecker::new();
        let reqs = GatingRequirements {
            bins: vec!["zzzz_impossible_binary".into()],
            env: vec!["BIZCLAW_NONEXISTENT_VAR".into()],
            features: vec![],
            files: vec![],
            os: vec![],
        };

        let result = checker.check(&reqs);
        assert!(!result.passed);
        assert_eq!(result.failures.len(), 2);
        assert_eq!(result.failures[0].requirement_type, "binary");
        assert_eq!(result.failures[1].requirement_type, "env");
    }

    #[test]
    fn test_gating_with_pass() {
        let mut checker = GatingChecker::new();
        let reqs = GatingRequirements {
            bins: vec![],
            env: vec!["PATH".into()],
            features: vec![],
            files: vec![],
            os: vec![],
        };

        let result = checker.check(&reqs);
        assert!(result.passed);
    }

    #[test]
    fn test_file_check() {
        let checker = GatingChecker::new();
        #[cfg(unix)]
        {
            assert!(checker.has_file("/etc/hosts"));
            assert!(!checker.has_file("/definitely/not/a/real/file"));
        }
    }

    #[test]
    fn test_binary_caching() {
        let mut checker = GatingChecker::new();
        let name = "zzzz_cache_test_binary";

        // First call
        let result1 = checker.has_binary(name);
        // Second call should use cache
        let result2 = checker.has_binary(name);
        assert_eq!(result1, result2);
        assert!(checker.bin_cache.contains_key(name));
    }

    #[test]
    fn test_system_capabilities() {
        let mut checker = GatingChecker::new();
        let caps = checker.system_capabilities();
        // Should have entries for common binaries
        assert!(caps.contains_key("bin:git") || caps.contains_key("bin:curl"));
        assert!(caps.contains_key(&format!("os:{}", std::env::consts::OS)));
    }
}
