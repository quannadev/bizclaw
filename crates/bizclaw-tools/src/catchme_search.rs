use async_trait::async_trait;
use bizclaw_catchme::store::CatchMeStore;
use bizclaw_core::error::{BizClawError, Result};
use bizclaw_core::traits::Tool;
use bizclaw_core::types::{ToolDefinition, ToolResult};
use serde_json::json;
use tracing::info;

pub struct CatchMeSearchTool {
    db_path: String,
}

impl CatchMeSearchTool {
    pub fn new(db_path: String) -> Self {
        Self { db_path }
    }
}

#[async_trait]
impl Tool for CatchMeSearchTool {
    fn name(&self) -> &str {
        "catchme_search"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: self.name().to_string(),
            description: "Search local digital footprint (screen, clipboard, keyboard) using CatchMe tree memory.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Natural language query about the user's past actions (e.g., 'What was I coding yesterday?')"
                    }
                },
                "required": ["query"]
            }),
        }
    }

    async fn execute(&self, arguments: &str) -> Result<ToolResult> {
        let args: serde_json::Value = serde_json::from_str(arguments)
            .map_err(|e| BizClawError::Tool(format!("Invalid arguments: {e}")))?;

        let query = args
            .get("query")
            .and_then(|q| q.as_str())
            .unwrap_or_default()
            .to_string();

        info!("Running CatchMe search: {}", query);

        // Here we'd typically initialize CatchMeStore and query the activity_tree
        let _store = match CatchMeStore::new(&self.db_path) {
            Ok(s) => s,
            Err(e) => {
                return Err(BizClawError::Tool(format!(
                    "Failed to open CatchMe database: {}",
                    e
                )));
            }
        };

        // Stub response (logic to traverse the Tree Memory is complex and requires LLM integration)
        let output = format!(
            "CatchMe Search Tool invoked with query: '{}'.\n\n(Note: Tree-travesal search algorithm is currently stubbed out in this Rust port.)",
            query
        );

        Ok(ToolResult {
            tool_call_id: String::new(),
            output,
            success: true,
        })
    }
}
