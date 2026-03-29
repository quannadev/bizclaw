//! MCP (Model Context Protocol) server for the Knowledge Base.
//!
//! Exposes the BizClaw knowledge base as an MCP tool that can be consumed by
//! external agents like Claude Code, Cursor, Continue, etc.
//!
//! ## Design
//! - Compatible with `/v1/chat/completions` OpenAI-compatible API
//! - Intent-level APIs, not raw data access
//! - Portable, file-based context that works across tools
//!
//! ## Architecture
//! ```text
//! External Agent (Claude Code, Cursor, etc.)
//!   ↓ MCP protocol (stdio or HTTP)
//!   ↓
//! BizClaw MCP Server
//!   ├── Tool: knowledge_search(query, filters) → search results
//!   ├── Tool: knowledge_list() → document list
//!   ├── Tool: knowledge_stats() → KB statistics
//!   ├── Tool: knowledge_nudges(message) → proactive suggestions
//!   └── Resource: knowledge://docs/{name} → document content
//! ```
//!
//! ## Usage with Claude Code
//! ```bash
//! # Add to ~/.claude/mcp.json
//! {
//!   "servers": {
//!     "bizclaw-kb": {
//!       "type": "http",
//!       "url": "http://localhost:3579/mcp/knowledge"
//!     }
//!   }
//! }
//! ```

use serde::{Deserialize, Serialize};

/// MCP tool definition for the knowledge base.
#[derive(Debug, Clone, Serialize)]
pub struct McpToolDef {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

/// MCP resource definition.
#[derive(Debug, Clone, Serialize)]
pub struct McpResourceDef {
    pub uri: String,
    pub name: String,
    pub description: String,
    pub mime_type: String,
}

/// MCP tool call request.
#[derive(Debug, Deserialize)]
pub struct McpToolCall {
    pub name: String,
    pub arguments: serde_json::Value,
}

/// MCP tool call response.
#[derive(Debug, Serialize)]
pub struct McpToolResponse {
    pub content: Vec<McpContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
}

/// MCP content item.
#[derive(Debug, Clone, Serialize)]
pub struct McpContent {
    #[serde(rename = "type")]
    pub content_type: String,
    pub text: String,
}

/// Build the list of MCP tools exposed by the knowledge base.
pub fn knowledge_tools() -> Vec<McpToolDef> {
    vec![
        McpToolDef {
            name: "knowledge_search".into(),
            description: "Search the BizClaw knowledge base for relevant documents and chunks. \
                          Supports filtered search by document name, MIME type, and owner."
                .into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Search query text"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum results (default 5, max 20)",
                        "default": 5
                    },
                    "filters": {
                        "type": "object",
                        "description": "Optional filters",
                        "properties": {
                            "doc_names": {
                                "type": "array",
                                "items": {"type": "string"},
                                "description": "Filter by document names"
                            },
                            "mimetypes": {
                                "type": "array",
                                "items": {"type": "string"},
                                "description": "Filter by MIME types (e.g., 'application/pdf')"
                            },
                            "owners": {
                                "type": "array",
                                "items": {"type": "string"},
                                "description": "Filter by document owners"
                            },
                            "score_threshold": {
                                "type": "number",
                                "description": "Minimum relevance score"
                            }
                        }
                    }
                },
                "required": ["query"]
            }),
        },
        McpToolDef {
            name: "knowledge_list".into(),
            description: "List all documents in the knowledge base with metadata \
                          (name, type, size, owner, chunk count)."
                .into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {}
            }),
        },
        McpToolDef {
            name: "knowledge_stats".into(),
            description: "Get knowledge base statistics: document count, chunk count, \
                          embedding coverage, file types breakdown."
                .into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {}
            }),
        },
        McpToolDef {
            name: "knowledge_nudges".into(),
            description: "Get proactive suggestions based on a message. \
                          Returns relevant documents, insights, and follow-up questions."
                .into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "message": {
                        "type": "string",
                        "description": "User message or topic to generate nudges for"
                    },
                    "context": {
                        "type": "string",
                        "description": "Optional conversation context"
                    }
                },
                "required": ["message"]
            }),
        },
    ]
}

/// Build the list of MCP resources (document URIs).
pub fn knowledge_resources(documents: &[crate::store::DocumentInfo]) -> Vec<McpResourceDef> {
    documents
        .iter()
        .map(|d| McpResourceDef {
            uri: format!("knowledge://docs/{}", d.name),
            name: d.name.clone(),
            description: format!(
                "{} ({}, {} chunks, {})",
                d.name,
                d.mimetype,
                d.chunk_count,
                format_bytes(d.file_size)
            ),
            mime_type: if d.mimetype.is_empty() {
                "text/plain".into()
            } else {
                d.mimetype.clone()
            },
        })
        .collect()
}

/// Handle an MCP tool call against the knowledge store.
///
/// Returns the tool response as JSON string for the MCP protocol.
pub fn handle_tool_call(
    store: &crate::store::KnowledgeStore,
    nudge_engine: &mut crate::nudges::NudgeEngine,
    call: &McpToolCall,
) -> McpToolResponse {
    match call.name.as_str() {
        "knowledge_search" => handle_search(store, &call.arguments),
        "knowledge_list" => handle_list(store),
        "knowledge_stats" => handle_stats(store),
        "knowledge_nudges" => handle_nudges(store, nudge_engine, &call.arguments),
        _ => McpToolResponse {
            content: vec![McpContent {
                content_type: "text".into(),
                text: format!("Unknown tool: {}", call.name),
            }],
            is_error: Some(true),
        },
    }
}

fn handle_search(
    store: &crate::store::KnowledgeStore,
    args: &serde_json::Value,
) -> McpToolResponse {
    let query = args["query"].as_str().unwrap_or("");
    let limit = args["limit"].as_u64().unwrap_or(5) as usize;

    let filter = if let Some(filters) = args.get("filters") {
        serde_json::from_value::<crate::search::SearchFilter>(filters.clone()).unwrap_or_default()
    } else {
        crate::search::SearchFilter::default()
    };

    let results = store.search_filtered(query, limit, &filter);

    if results.is_empty() {
        return McpToolResponse {
            content: vec![McpContent {
                content_type: "text".into(),
                text: format!("No results found for: \"{}\"", query),
            }],
            is_error: None,
        };
    }

    let formatted = results
        .iter()
        .enumerate()
        .map(|(i, r)| {
            format!(
                "[{}] 📄 {} (chunk {}, score: {:.2})\n{}",
                i + 1,
                r.doc_name,
                r.chunk_idx,
                r.score.abs(),
                r.content
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n---\n\n");

    McpToolResponse {
        content: vec![McpContent {
            content_type: "text".into(),
            text: format!(
                "Found {} results for \"{}\":\n\n{}",
                results.len(),
                query,
                formatted
            ),
        }],
        is_error: None,
    }
}

fn handle_list(store: &crate::store::KnowledgeStore) -> McpToolResponse {
    let docs = store.list_documents();

    if docs.is_empty() {
        return McpToolResponse {
            content: vec![McpContent {
                content_type: "text".into(),
                text: "Knowledge base is empty. No documents uploaded yet.".into(),
            }],
            is_error: None,
        };
    }

    let formatted = docs
        .iter()
        .map(|d| {
            format!(
                "• {} — {} ({}, {} chunks, {})",
                d.name,
                d.mimetype,
                d.source,
                d.chunk_count,
                format_bytes(d.file_size)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    McpToolResponse {
        content: vec![McpContent {
            content_type: "text".into(),
            text: format!("{} documents:\n\n{}", docs.len(), formatted),
        }],
        is_error: None,
    }
}

fn handle_stats(store: &crate::store::KnowledgeStore) -> McpToolResponse {
    let stats = store.detailed_stats();

    McpToolResponse {
        content: vec![McpContent {
            content_type: "text".into(),
            text: serde_json::to_string_pretty(&stats).unwrap_or_else(|_| "{}".into()),
        }],
        is_error: None,
    }
}

fn handle_nudges(
    store: &crate::store::KnowledgeStore,
    nudge_engine: &mut crate::nudges::NudgeEngine,
    args: &serde_json::Value,
) -> McpToolResponse {
    let message = args["message"].as_str().unwrap_or("");
    let context = args["context"].as_str();

    // Search for relevant content first
    let results = store.search(message, 5);

    // Generate nudges
    let nudges = nudge_engine.generate_nudges(message, &results, context);

    if nudges.is_empty() {
        return McpToolResponse {
            content: vec![McpContent {
                content_type: "text".into(),
                text: "No suggestions at this time.".into(),
            }],
            is_error: None,
        };
    }

    let formatted = nudges
        .iter()
        .map(|n| {
            let source = n
                .source_doc
                .as_deref()
                .map(|s| format!(" (from {})", s))
                .unwrap_or_default();
            format!("{}{}", n.text, source)
        })
        .collect::<Vec<_>>()
        .join("\n");

    McpToolResponse {
        content: vec![McpContent {
            content_type: "text".into(),
            text: format!("💡 Suggestions:\n\n{}", formatted),
        }],
        is_error: None,
    }
}

/// Format bytes to human-readable string.
fn format_bytes(bytes: i64) -> String {
    if bytes < 1024 {
        format!("{}B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1}KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1}MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_knowledge_tools_defined() {
        let tools = knowledge_tools();
        assert_eq!(tools.len(), 4);
        assert!(tools.iter().any(|t| t.name == "knowledge_search"));
        assert!(tools.iter().any(|t| t.name == "knowledge_list"));
        assert!(tools.iter().any(|t| t.name == "knowledge_stats"));
        assert!(tools.iter().any(|t| t.name == "knowledge_nudges"));
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(500), "500B");
        assert_eq!(format_bytes(1536), "1.5KB");
        assert_eq!(format_bytes(1_048_576), "1.0MB");
    }

    #[test]
    fn test_knowledge_resources() {
        let docs = vec![crate::store::DocumentInfo {
            id: 1,
            name: "test.md".into(),
            source: "api".into(),
            chunk_count: 5,
            mimetype: "text/markdown".into(),
            owner: "admin".into(),
            file_size: 2048,
            created_at: "2025-01-01".into(),
        }];

        let resources = knowledge_resources(&docs);
        assert_eq!(resources.len(), 1);
        assert_eq!(resources[0].uri, "knowledge://docs/test.md");
        assert_eq!(resources[0].mime_type, "text/markdown");
    }

    #[test]
    fn test_handle_search_empty_store() {
        let store = crate::store::KnowledgeStore::open(std::path::Path::new(":memory:")).unwrap();
        let response = handle_search(&store, &serde_json::json!({"query": "test"}));
        assert!(response.content[0].text.contains("No results"));
    }

    #[test]
    fn test_handle_list_empty_store() {
        let store = crate::store::KnowledgeStore::open(std::path::Path::new(":memory:")).unwrap();
        let response = handle_list(&store);
        assert!(response.content[0].text.contains("empty"));
    }

    #[test]
    fn test_handle_stats() {
        let store = crate::store::KnowledgeStore::open(std::path::Path::new(":memory:")).unwrap();
        let response = handle_stats(&store);
        assert!(response.content[0].text.contains("documents"));
    }

    #[test]
    fn test_handle_unknown_tool() {
        let store = crate::store::KnowledgeStore::open(std::path::Path::new(":memory:")).unwrap();
        let mut engine = crate::nudges::NudgeEngine::new(crate::nudges::NudgeConfig::default());
        let call = McpToolCall {
            name: "unknown_tool".into(),
            arguments: serde_json::json!({}),
        };
        let response = handle_tool_call(&store, &mut engine, &call);
        assert!(response.is_error == Some(true));
    }
}
