use async_trait::async_trait;
use bizclaw_core::error::Result;
use bizclaw_core::traits::Tool;
use bizclaw_core::types::{ToolDefinition, ToolResult};
use serde_json::{Value, json};
use tracing::{info, warn};

/// Extract Insight Tool — implements OpenGnothia's Cumulative Memory pattern.
/// Allows the agent to explicitly extract facts learned during the conversation
/// and write them persistently into the MEOMORY.md architecture.
pub struct ExtractInsightTool {
    memory_impl: bizclaw_memory::brain::BrainWorkspace,
}

impl ExtractInsightTool {
    pub fn new() -> Self {
        Self {
            memory_impl: bizclaw_memory::brain::BrainWorkspace::default(),
        }
    }
}

impl Default for ExtractInsightTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for ExtractInsightTool {
    fn name(&self) -> &str {
        "extract_insight"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "extract_insight".into(),
            description: "Save a learned fact, user preference, or key decision permanently to the agent's memory (MEMORY.md). \
                Use this whenever you learn something important about the user, the business, or the environment that \
                should be remembered for ALL future interactions. \
                This adheres to the OpenGnothia Cumulative Memory pattern.".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "fact": {
                        "type": "string",
                        "description": "The concise fact, decision, or insight to memorize. (e.g. 'User prefers bullet points', or 'Database uses PostgreSQL')."
                    },
                    "category": {
                        "type": "string",
                        "description": "Category: 'environment', 'user_preference', 'business_rule', 'contact_info', or 'other'.",
                        "enum": ["environment", "user_preference", "business_rule", "contact_info", "other"]
                    }
                },
                "required": ["fact", "category"]
            }),
        }
    }

    async fn execute(&self, arguments: &str) -> Result<ToolResult> {
        let args: Value = match serde_json::from_str(arguments) {
            Ok(v) => v,
            Err(e) => {
                return Ok(ToolResult {
                    tool_call_id: String::new(),
                    output: format!("Invalid arguments: {}", e),
                    success: false,
                });
            }
        };

        let fact = args["fact"].as_str().unwrap_or("").trim();
        let category = args["category"].as_str().unwrap_or("other");

        if fact.is_empty() {
            return Ok(ToolResult {
                tool_call_id: String::new(),
                output: "Field 'fact' cannot be empty".to_string(),
                success: false,
            });
        }

        // Read existing MEMORY.md
        let mut memory_content = self
            .memory_impl
            .read_file("MEMORY.md")
            .unwrap_or_else(|| "# 🧠 Long-Term Memory\n\n".to_string());

        // Format the new insight
        let date = chrono::Local::now().format("%Y-%m-%d").to_string();
        let insight_entry = format!("- **[{}]** ({}) {}", category.to_uppercase(), date, fact);

        // Append to the appropriate section or bottom of file
        let marker = "## 📌 Lịch sử học tập (Learned Insights)";
        if let Some(pos) = memory_content.find(marker) {
            let insert_pos = pos + marker.len();
            memory_content.insert_str(insert_pos, &format!("\n{}", insight_entry));
        } else {
            memory_content.push_str(&format!("\n\n{}\n{}", marker, insight_entry));
        }

        // Write back
        if let Err(e) = self.memory_impl.write_file("MEMORY.md", &memory_content) {
            warn!("Failed to write insight to MEMORY.md: {}", e);
            return Ok(ToolResult {
                tool_call_id: String::new(),
                output: format!("Failed to save memory: {}", e),
                success: false,
            });
        }

        info!("🧠 Insight extracted and saved: {}", fact);

        Ok(ToolResult {
            tool_call_id: String::new(),
            output: format!(
                "Insight successfully saved to persistent MEMORY.md: {}",
                fact
            ),
            success: true,
        })
    }
}
