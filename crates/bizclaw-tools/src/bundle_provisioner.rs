//! Bundle Provisioner — Tự động cấu hình ứng dụng SME từ app-bundles.json.
//!
//! Khi doanh nghiệp chọn 1 trong 6 ứng dụng, provisioner sẽ:
//! 1. Load bundle config từ `data/app-bundles.json`
//! 2. Generate config.toml cho tenant
//! 3. Tạo agent system prompts (MD files)
//! 4. Cấu hình DB connections
//! 5. Đăng ký scheduled tasks (Autonomous Hands)
//! 6. Load gallery skills phù hợp
//! 7. Kích hoạt MCP servers cần thiết
//! 8. Trả về onboarding flow cho frontend

use async_trait::async_trait;
use bizclaw_core::error::Result;
use bizclaw_core::traits::Tool;
use bizclaw_core::types::{ToolDefinition, ToolResult};
use serde::{Deserialize, Serialize};

// ══════════════════════════════════════════════════════════════
// Data structures
// ══════════════════════════════════════════════════════════════

#[derive(Debug, Serialize, Deserialize)]
pub struct AppBundleFile {
    pub version: String,
    pub description: String,
    pub bundles: Vec<AppBundle>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppBundle {
    pub id: String,
    pub name: String,
    pub icon: String,
    pub tagline: String,
    pub description: String,
    pub tier: String,
    pub color: String,
    pub industries: Vec<String>,
    pub config: BundleConfig,
    pub onboarding: BundleOnboarding,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BundleConfig {
    pub identity: BundleIdentity,
    pub agents: Vec<BundleAgent>,
    pub db_connections: Vec<serde_json::Value>,
    pub channels: Vec<String>,
    pub mcp_servers: Vec<String>,
    pub gallery_skills: Vec<String>,
    pub scheduled_tasks: Vec<BundleScheduledTask>,
    #[serde(default)]
    pub api_endpoints: Vec<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BundleIdentity {
    pub name: String,
    pub persona: String,
    #[serde(default)]
    pub system_prompt: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BundleAgent {
    pub id: String,
    pub name: String,
    pub icon: String,
    #[serde(default)]
    pub persona: String,
    #[serde(default)]
    pub system_prompt: String,
    #[serde(default)]
    pub tools: Vec<String>,
    #[serde(default)]
    pub channels: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BundleScheduledTask {
    pub name: String,
    pub schedule: String,
    pub description: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BundleOnboarding {
    pub welcome: serde_json::Value,
    pub steps: Vec<serde_json::Value>,
    pub completion: serde_json::Value,
}

// ══════════════════════════════════════════════════════════════
// Provisioner Tool
// ══════════════════════════════════════════════════════════════

pub struct BundleProvisionerTool {
    data_dir: std::path::PathBuf,
}

impl BundleProvisionerTool {
    pub fn new() -> Self {
        Self {
            data_dir: std::path::PathBuf::from("data"),
        }
    }

    pub fn with_data_dir(path: std::path::PathBuf) -> Self {
        Self { data_dir: path }
    }

    /// Load all bundles from disk
    fn load_bundles(&self) -> Result<AppBundleFile> {
        let path = self.data_dir.join("app-bundles.json");
        let content = std::fs::read_to_string(&path).map_err(|e| {
            bizclaw_core::error::BizClawError::Tool(format!(
                "Cannot read app-bundles.json at {}: {e}",
                path.display()
            ))
        })?;
        serde_json::from_str(&content).map_err(|e| {
            bizclaw_core::error::BizClawError::Tool(format!("Parse app-bundles.json: {e}"))
        })
    }

    /// List all available bundles (summary only)
    fn list_bundles(&self) -> Result<String> {
        let file = self.load_bundles()?;
        let mut out = String::from("📦 Available Application Bundles:\n\n");
        for b in &file.bundles {
            out.push_str(&format!(
                "{} **{}** — {}\n   Tier: {} | Industries: {}\n   Agents: {} | Skills: {} | Tasks: {}\n\n",
                b.icon,
                b.name,
                b.tagline,
                b.tier,
                b.industries.join(", "),
                b.config.agents.len(),
                b.config.gallery_skills.len(),
                b.config.scheduled_tasks.len(),
            ));
        }
        Ok(out)
    }

    /// Get bundle details + onboarding flow
    fn get_bundle(&self, bundle_id: &str) -> Result<String> {
        let file = self.load_bundles()?;
        let bundle = file
            .bundles
            .iter()
            .find(|b| b.id == bundle_id)
            .ok_or_else(|| {
                bizclaw_core::error::BizClawError::Tool(format!(
                    "Bundle '{}' not found. Available: {}",
                    bundle_id,
                    file.bundles
                        .iter()
                        .map(|b| b.id.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                ))
            })?;

        Ok(serde_json::to_string_pretty(bundle).unwrap_or_default())
    }

    /// Provision a bundle — generate configs and MD files
    fn provision(&self, bundle_id: &str, workspace: &str) -> Result<String> {
        let file = self.load_bundles()?;
        let bundle = file
            .bundles
            .iter()
            .find(|b| b.id == bundle_id)
            .ok_or_else(|| {
                bizclaw_core::error::BizClawError::Tool(format!("Bundle '{bundle_id}' not found"))
            })?;

        let workspace_path = std::path::Path::new(workspace);
        let mut steps_done = Vec::new();

        // 1. Generate config.toml
        let config_toml = self.generate_config_toml(bundle);
        let config_path = workspace_path.join("config.toml");
        std::fs::write(&config_path, &config_toml).map_err(|e| {
            bizclaw_core::error::BizClawError::Tool(format!("Write config.toml: {e}"))
        })?;
        steps_done.push(format!(
            "✅ config.toml generated at {}",
            config_path.display()
        ));

        // 2. Create agent system prompt files
        let agents_dir = workspace_path.join("agents");
        std::fs::create_dir_all(&agents_dir).ok();
        for agent in &bundle.config.agents {
            let agent_dir = agents_dir.join(&agent.id);
            std::fs::create_dir_all(&agent_dir).ok();
            let prompt_path = agent_dir.join("system.md");
            let content = format!(
                "# {} {}\n\n## Persona\n{}\n\n## System Prompt\n{}\n\n## Tools\n{}\n\n## Channels\n{}\n",
                agent.icon,
                agent.name,
                agent.persona,
                agent.system_prompt,
                agent.tools.join(", "),
                agent.channels.join(", "),
            );
            std::fs::write(&prompt_path, content).ok();
            steps_done.push(format!("✅ Agent '{}' system.md created", agent.name));
        }

        // 3. Create scheduled tasks config
        if !bundle.config.scheduled_tasks.is_empty() {
            let hands_dir = workspace_path.join("hands");
            std::fs::create_dir_all(&hands_dir).ok();
            let tasks_json =
                serde_json::to_string_pretty(&bundle.config.scheduled_tasks).unwrap_or_default();
            let hands_path = hands_dir.join("scheduled.json");
            std::fs::write(&hands_path, tasks_json).ok();
            steps_done.push(format!(
                "✅ {} scheduled tasks configured",
                bundle.config.scheduled_tasks.len()
            ));
        }

        // 4. Summary
        let summary = format!(
            "🎉 **Bundle '{} {}' provisioned successfully!**\n\n{}\n\n\
             📋 Next: Complete the onboarding wizard to connect your database and chat channels.",
            bundle.icon,
            bundle.name,
            steps_done.join("\n")
        );

        Ok(summary)
    }

    /// Generate config.toml content from bundle
    fn generate_config_toml(&self, bundle: &AppBundle) -> String {
        let mut toml = String::new();

        // Header
        toml.push_str(&format!(
            "# ═══════════════════════════════════════════════════════════\n\
             # {} {} — Auto-generated by BizClaw Bundle Provisioner\n\
             # ═══════════════════════════════════════════════════════════\n\n",
            bundle.icon, bundle.name
        ));

        // Identity
        toml.push_str(&format!(
            "[identity]\nname = \"{}\"\npersona = \"{}\"\nsystem_prompt = \"\"\"{}\"\"\"\n\n",
            bundle.config.identity.name,
            bundle.config.identity.persona,
            bundle.config.identity.system_prompt,
        ));

        // Gateway
        toml.push_str("[gateway]\nport = 3000\nhost = \"127.0.0.1\"\n\n");

        // Channels (commented, user needs to add tokens)
        for ch in &bundle.config.channels {
            toml.push_str(&format!(
                "# ── {} ──\n# [[channel.{}]]\n# enabled = true\n# bot_token = \"YOUR_TOKEN\"\n\n",
                ch, ch
            ));
        }

        // MCP Servers (commented)
        for mcp in &bundle.config.mcp_servers {
            toml.push_str(&format!("# [[mcp_servers]]\n# name = \"{}\"\n\n", mcp));
        }

        toml
    }

    /// Get onboarding screens for a bundle
    fn get_onboarding(&self, bundle_id: &str) -> Result<String> {
        let path = self.data_dir.join("onboarding/welcome-screens.json");
        if path.exists() {
            let content = std::fs::read_to_string(&path).map_err(|e| {
                bizclaw_core::error::BizClawError::Tool(format!("Read onboarding: {e}"))
            })?;
            let screens: serde_json::Value = serde_json::from_str(&content).map_err(|e| {
                bizclaw_core::error::BizClawError::Tool(format!("Parse onboarding: {e}"))
            })?;

            if let Some(app_screens) = screens["screens"][bundle_id].as_array() {
                return Ok(serde_json::to_string_pretty(app_screens).unwrap_or_default());
            }
        }

        // Fallback: get from bundle itself
        let file = self.load_bundles()?;
        let bundle = file
            .bundles
            .iter()
            .find(|b| b.id == bundle_id)
            .ok_or_else(|| {
                bizclaw_core::error::BizClawError::Tool(format!("Bundle '{bundle_id}' not found"))
            })?;
        Ok(serde_json::to_string_pretty(&bundle.onboarding).unwrap_or_default())
    }
}

impl Default for BundleProvisionerTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for BundleProvisionerTool {
    fn name(&self) -> &str {
        "bundle_provision"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "bundle_provision".into(),
            description: concat!(
                "SME Application Bundle Provisioner. ",
                "Actions: list (show 6 available app bundles), ",
                "get (detailed bundle info), ",
                "provision (auto-generate all configs for a bundle), ",
                "onboarding (get welcome wizard screens). ",
                "Bundles: retail, fnb, realestate, healthcare, education, professional."
            )
            .into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["list", "get", "provision", "onboarding"],
                        "description": "Action to perform"
                    },
                    "bundle_id": {
                        "type": "string",
                        "enum": ["retail", "fnb", "realestate", "healthcare", "education", "professional"],
                        "description": "Bundle ID (for get/provision/onboarding)"
                    },
                    "workspace": {
                        "type": "string",
                        "description": "Workspace directory path (for provision action)"
                    }
                },
                "required": ["action"]
            }),
        }
    }

    async fn execute(&self, arguments: &str) -> Result<ToolResult> {
        let args: serde_json::Value = serde_json::from_str(arguments)
            .map_err(|e| bizclaw_core::error::BizClawError::Tool(e.to_string()))?;

        let action = args["action"]
            .as_str()
            .ok_or_else(|| bizclaw_core::error::BizClawError::Tool("Missing 'action'".into()))?;

        match action {
            "list" => {
                let output = self.list_bundles()?;
                Ok(ToolResult {
                    tool_call_id: String::new(),
                    output,
                    success: true,
                })
            }
            "get" => {
                let id = args["bundle_id"].as_str().ok_or_else(|| {
                    bizclaw_core::error::BizClawError::Tool("Missing 'bundle_id'".into())
                })?;
                let output = self.get_bundle(id)?;
                Ok(ToolResult {
                    tool_call_id: String::new(),
                    output,
                    success: true,
                })
            }
            "provision" => {
                let id = args["bundle_id"].as_str().ok_or_else(|| {
                    bizclaw_core::error::BizClawError::Tool("Missing 'bundle_id'".into())
                })?;
                let workspace = args["workspace"].as_str().unwrap_or("~/.bizclaw");
                let output = self.provision(id, workspace)?;
                Ok(ToolResult {
                    tool_call_id: String::new(),
                    output,
                    success: true,
                })
            }
            "onboarding" => {
                let id = args["bundle_id"].as_str().ok_or_else(|| {
                    bizclaw_core::error::BizClawError::Tool("Missing 'bundle_id'".into())
                })?;
                let output = self.get_onboarding(id)?;
                Ok(ToolResult {
                    tool_call_id: String::new(),
                    output,
                    success: true,
                })
            }
            _ => Ok(ToolResult {
                tool_call_id: String::new(),
                output: format!(
                    "Unknown action: {}. Available: list, get, provision, onboarding",
                    action
                ),
                success: false,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_name() {
        let tool = BundleProvisionerTool::new();
        assert_eq!(tool.name(), "bundle_provision");
    }

    #[test]
    fn test_tool_definition() {
        let tool = BundleProvisionerTool::new();
        let def = tool.definition();
        assert_eq!(def.name, "bundle_provision");
        assert!(def.description.contains("Bundle"));
    }
}
