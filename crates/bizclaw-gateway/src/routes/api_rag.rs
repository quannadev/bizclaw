//! Knowledge Base and RAG/NL-Query APIs.
use axum::{Json, extract::State};
use std::sync::Arc;
use crate::server::AppState;

// ---- Knowledge Base API ----
use bizclaw_core::traits::Tool;
// Extracted to routes/knowledge.rs — re-export for backward compatibility
pub use crate::routes::knowledge::{knowledge_search, knowledge_list_docs, knowledge_stats,
        knowledge_nudges, knowledge_mcp_tools, knowledge_mcp_call,
    knowledge_watch_scan, knowledge_signal_stats, knowledge_signal_feedback,
    knowledge_add_doc, knowledge_remove_doc, knowledge_upload_file,
};


// ───── NL Query (Text2SQL RAG) API ─────

/// GET /api/v1/nl-query/status — connections + indexed status
pub async fn nl_query_status(State(_state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let conn_mgr = bizclaw_tools::db_connection::DbConnectionManager::load_default();
    let schema_store = bizclaw_tools::db_semantic::SchemaLayerStore::default();

    let connections: Vec<serde_json::Value> = conn_mgr
        .list()
        .iter()
        .map(|c| {
            serde_json::json!({
                "id": c.id,
                "db_type": c.db_type,
                "description": c.description,
                "read_only": c.read_only,
            })
        })
        .collect();

    let indexed = schema_store.list_indexed();
    let example_store = bizclaw_tools::db_examples::SqlExampleStore::default();

    Json(serde_json::json!({
        "connections": connections,
        "indexed": indexed,
        "example_count": example_store.count(),
    }))
}

/// POST /api/v1/nl-query/ask — ask a NL question
pub async fn nl_query_ask(
    State(_state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let conn_id = body["connection_id"].as_str().unwrap_or("");
    let question = body["question"].as_str().unwrap_or("");

    if conn_id.is_empty() || question.is_empty() {
        return Json(
            serde_json::json!({"ok": false, "error": "connection_id and question required"}),
        );
    }

    let tool = bizclaw_tools::nl_query::NlQueryTool::new();
    let args = serde_json::json!({
        "action": "ask",
        "connection_id": conn_id,
        "question": question,
    });

    match tool.execute(&args.to_string()).await {
        Ok(result) => Json(serde_json::json!({
            "ok": result.success,
            "result": result.output,
        })),
        Err(e) => Json(serde_json::json!({
            "ok": false,
            "error": format!("{e}"),
        })),
    }
}

/// POST /api/v1/nl-query/index — index database schema
pub async fn nl_query_index(
    State(_state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let conn_id = body["connection_id"].as_str().unwrap_or("");
    if conn_id.is_empty() {
        return Json(serde_json::json!({"ok": false, "error": "connection_id required"}));
    }

    let tool = bizclaw_tools::nl_query::NlQueryTool::new();
    let args = serde_json::json!({
        "action": "index",
        "connection_id": conn_id,
    });

    match tool.execute(&args.to_string()).await {
        Ok(result) => Json(serde_json::json!({
            "ok": result.success,
            "result": result.output,
        })),
        Err(e) => Json(serde_json::json!({
            "ok": false,
            "error": format!("{e}"),
        })),
    }
}

/// GET /api/v1/nl-query/rules/:conn_id — list rules
pub async fn nl_query_rules_get(
    axum::extract::Path(conn_id): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    let store = bizclaw_tools::db_examples::BusinessRuleStore::default();
    let rules: Vec<serde_json::Value> = store
        .get_rules(&conn_id)
        .iter()
        .map(|r| {
            serde_json::json!({
                "id": r.id,
                "connection_id": r.connection_id,
                "rule": r.rule,
            })
        })
        .collect();
    Json(serde_json::json!({"rules": rules}))
}

/// POST /api/v1/nl-query/rules — add a rule
pub async fn nl_query_rules_add(Json(body): Json<serde_json::Value>) -> Json<serde_json::Value> {
    let conn_id = body["connection_id"].as_str().unwrap_or("");
    let rule = body["rule"].as_str().unwrap_or("");

    if conn_id.is_empty() || rule.is_empty() {
        return Json(serde_json::json!({"ok": false, "error": "connection_id and rule required"}));
    }

    let store = bizclaw_tools::db_examples::BusinessRuleStore::default();
    match store.add_rule(conn_id, rule) {
        Ok(id) => Json(serde_json::json!({"ok": true, "id": id})),
        Err(e) => Json(serde_json::json!({"ok": false, "error": e})),
    }
}

/// GET /api/v1/nl-query/examples/:conn_id — list examples
pub async fn nl_query_examples_get(
    axum::extract::Path(conn_id): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    let store = bizclaw_tools::db_examples::SqlExampleStore::default();
    let examples: Vec<serde_json::Value> = store
        .list_recent(&conn_id, 50)
        .iter()
        .map(|e| {
            serde_json::json!({
                "id": e.id,
                "question": e.question,
                "sql": e.sql,
                "tables_used": e.tables_used,
                "created_at": e.created_at,
            })
        })
        .collect();
    Json(serde_json::json!({"examples": examples}))
}

