//! Hand manifest — HAND.toml configuration format.
//!
//! Manifest-driven hand activation and lifecycle management.
//!
//! ## OpenFang / FangHub Compatibility
//! BizClaw HAND.toml is a superset of OpenFang's manifest format.
//! Hands can be published to ClawHub or FangHub marketplace.
//! Extra fields (like `dashboard` and `author`) are optional extensions.

use serde::{Deserialize, Serialize};

/// Schedule type for a Hand.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HandSchedule {
    /// Run at a specific cron expression (e.g., "0 6 * * *" = 6 AM daily).
    Cron(String),
    /// Run every N seconds.
    Interval(u64),
    /// Run once on activation.
    Once,
    /// Manual trigger only (via CLI or API).
    Manual,
}

impl std::fmt::Display for HandSchedule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cron(expr) => write!(f, "cron({expr})"),
            Self::Interval(secs) => {
                if *secs >= 3600 {
                    write!(f, "every {}h", secs / 3600)
                } else if *secs >= 60 {
                    write!(f, "every {}min", secs / 60)
                } else {
                    write!(f, "every {secs}s")
                }
            }
            Self::Once => write!(f, "once"),
            Self::Manual => write!(f, "manual"),
        }
    }
}

/// Phase definition within a Hand's playbook.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseManifest {
    pub name: String,
    pub description: String,
    /// Tools this phase is allowed to use.
    #[serde(default)]
    pub allowed_tools: Vec<String>,
    /// Max execution time for this phase (seconds).
    #[serde(default = "default_phase_timeout")]
    pub timeout_secs: u64,
    /// Whether this phase requires human approval before executing.
    #[serde(default)]
    pub requires_approval: bool,
}

fn default_phase_timeout() -> u64 {
    300 // 5 minutes
}

/// Tool requirements for a Hand (OpenFang-compatible).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ToolsManifest {
    /// Tools that MUST be available for this Hand to function.
    #[serde(default)]
    pub required: Vec<String>,
    /// Tools that enhance this Hand but aren't required.
    #[serde(default)]
    pub optional: Vec<String>,
}

/// Dashboard configuration for a Hand (OpenFang-compatible).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DashboardManifest {
    /// Metric names this Hand reports (shown on dashboard).
    #[serde(default)]
    pub metrics: Vec<String>,
    /// Optional widget type for dashboard display.
    #[serde(default)]
    pub widget: String,
}

/// Author metadata for marketplace publishing.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AuthorManifest {
    /// Author name.
    #[serde(default)]
    pub name: String,
    /// Author URL or profile.
    #[serde(default)]
    pub url: String,
}

/// Hand manifest — loaded from HAND.toml.
///
/// Compatible with OpenFang's HAND.toml format + BizClaw extensions.
/// Can be published to ClawHub or FangHub marketplace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandManifest {
    /// Unique hand identifier (e.g., "research", "monitor").
    pub name: String,
    /// Human-readable label.
    pub label: String,
    /// Icon emoji.
    #[serde(default = "default_icon")]
    pub icon: String,
    /// Description of what this hand does.
    pub description: String,
    /// Version string.
    #[serde(default = "default_version")]
    pub version: String,
    /// Execution schedule.
    pub schedule: HandSchedule,
    /// Phases in the multi-phase playbook.
    pub phases: Vec<PhaseManifest>,
    /// LLM provider to use (empty = use default).
    #[serde(default)]
    pub provider: String,
    /// LLM model to use (empty = use default).
    #[serde(default)]
    pub model: String,
    /// Maximum total execution time (seconds).
    #[serde(default = "default_max_runtime")]
    pub max_runtime_secs: u64,
    /// Whether this hand is enabled.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Notification channels for results (e.g., ["telegram", "email"]).
    #[serde(default)]
    pub notify_channels: Vec<String>,

    // ── OpenFang-compatible extensions ──
    /// Tool requirements (required + optional).
    #[serde(default)]
    pub tools: ToolsManifest,
    /// Dashboard metrics and widget configuration.
    #[serde(default)]
    pub dashboard: DashboardManifest,
    /// Author metadata for marketplace publishing.
    #[serde(default)]
    pub author: AuthorManifest,
    /// Tags for ClawHub / FangHub discovery.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Minimum BizClaw version required.
    #[serde(default)]
    pub min_version: String,
    /// License identifier (e.g., "MIT", "Apache-2.0").
    #[serde(default)]
    pub license: String,
}

fn default_icon() -> String {
    "🤖".into()
}
fn default_version() -> String {
    "1.0.0".into()
}
fn default_max_runtime() -> u64 {
    1800 // 30 minutes
}
fn default_true() -> bool {
    true
}

impl HandManifest {
    /// Parse a HAND.toml manifest from string content.
    pub fn from_toml(content: &str) -> Result<Self, String> {
        toml::from_str(content).map_err(|e| format!("Parse HAND.toml: {e}"))
    }

    /// Load manifest from a HAND.toml file path.
    pub fn load(path: &std::path::Path) -> Result<Self, String> {
        let content =
            std::fs::read_to_string(path).map_err(|e| format!("Read {}: {e}", path.display()))?;
        Self::from_toml(&content)
    }

    /// Check if all required tools are available in a given tool list.
    pub fn check_tools(&self, available: &[String]) -> Vec<String> {
        self.tools
            .required
            .iter()
            .filter(|t| !available.contains(t))
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_manifest() {
        let toml_str = r#"
name = "research"
label = "Research Hand"
icon = "🔍"
description = "Autonomous competitive research and knowledge graph building"
schedule = { cron = "0 6 * * *" }
provider = "gemini"
model = "gemini-2.5-flash-preview-05-20"
notify_channels = ["telegram"]

[[phases]]
name = "gather"
description = "Search and collect relevant information"
allowed_tools = ["web_search", "http_request"]
timeout_secs = 600

[[phases]]
name = "analyze"
description = "Analyze gathered information and extract insights"
allowed_tools = ["shell"]
timeout_secs = 300

[[phases]]
name = "report"
description = "Generate and deliver structured report"
allowed_tools = ["file", "session_context"]
timeout_secs = 120
"#;
        let manifest = HandManifest::from_toml(toml_str).unwrap();
        assert_eq!(manifest.name, "research");
        assert_eq!(manifest.phases.len(), 3);
        assert_eq!(manifest.phases[0].name, "gather");
        assert!(matches!(manifest.schedule, HandSchedule::Cron(_)));
    }

    #[test]
    fn test_schedule_display() {
        assert_eq!(HandSchedule::Interval(300).to_string(), "every 5min");
        assert_eq!(HandSchedule::Interval(7200).to_string(), "every 2h");
        assert_eq!(HandSchedule::Manual.to_string(), "manual");
    }

    #[test]
    fn test_openfang_compatible_manifest() {
        let toml_str = r#"
name = "seo-monitor"
label = "SEO Monitor"
icon = "📊"
description = "Monitor search rankings and report changes"
version = "1.2.0"
schedule = { interval = 3600 }
tags = ["seo", "monitoring", "marketing"]
min_version = "1.0.5"
license = "MIT"

[tools]
required = ["web_search", "http_request"]
optional = ["shell", "file"]

[dashboard]
metrics = ["rankings_checked", "changes_detected", "uptime"]
widget = "line_chart"

[author]
name = "BizClaw Team"
url = "https://bizclaw.vn"

[[phases]]
name = "crawl"
description = "Fetch current search rankings"
allowed_tools = ["web_search", "http_request"]
timeout_secs = 900
"#;
        let manifest = HandManifest::from_toml(toml_str).unwrap();
        assert_eq!(manifest.name, "seo-monitor");
        assert_eq!(manifest.tools.required, vec!["web_search", "http_request"]);
        assert_eq!(manifest.tools.optional, vec!["shell", "file"]);
        assert_eq!(manifest.dashboard.metrics.len(), 3);
        assert_eq!(manifest.dashboard.widget, "line_chart");
        assert_eq!(manifest.author.name, "BizClaw Team");
        assert_eq!(manifest.tags, vec!["seo", "monitoring", "marketing"]);
        assert_eq!(manifest.license, "MIT");
        assert_eq!(manifest.min_version, "1.0.5");
    }

    #[test]
    fn test_check_tools() {
        let toml_str = r#"
name = "test"
label = "Test"
description = "Test hand"
schedule = "manual"

[tools]
required = ["web_search", "shell", "file"]

[[phases]]
name = "run"
description = "Execute"
"#;
        let manifest = HandManifest::from_toml(toml_str).unwrap();
        let available = vec!["web_search".to_string(), "file".to_string()];
        let missing = manifest.check_tools(&available);
        assert_eq!(missing, vec!["shell"]);

        let all_available = vec![
            "web_search".to_string(),
            "shell".to_string(),
            "file".to_string(),
        ];
        assert!(manifest.check_tools(&all_available).is_empty());
    }
}
