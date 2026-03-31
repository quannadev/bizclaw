//! Shared utility functions used by route sub-modules.
//!
//! Extracted from routes/mod.rs to enable code sharing across
//! knowledge.rs, workflows.rs, and future sub-modules.

use axum::Json;

/// Return sanitized error — logs real error server-side, sends generic message to client.
pub fn internal_error(context: &str, e: impl std::fmt::Display) -> Json<serde_json::Value> {
    tracing::error!("[{context}] {e}");
    Json(serde_json::json!({"ok": false, "error": "An internal error occurred"}))
}
