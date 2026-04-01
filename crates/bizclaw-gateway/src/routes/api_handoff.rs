use axum::{
    extract::{Path, State},
    Json,
};
use bizclaw_core::error::{BizClawError, Result};

use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::server::AppState;

#[derive(Debug, Serialize, Deserialize)]
pub struct HandoffTicket {
    pub id: String,
    pub customer: String,
    pub channel: String,
    pub reason: String,
    pub message: String,
    pub status: String,
    pub context_summary: String,
    pub ai_attempts: i64,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HandoffSettings {
    pub enabled: bool,
    pub auto_handoff: bool,
    pub triggers: Vec<String>,
    pub timeout_minutes: i64,
}

pub async fn list_handoff_queue(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let conn = match state.db.lock_conn() {
        Ok(c) => c,
        Err(e) => return Json(serde_json::json!({"error": format!("DB error: {e}")}))
    };
    
    // Ensure table exists
    conn.execute(
        "CREATE TABLE IF NOT EXISTS handoff_queue (
            id TEXT PRIMARY KEY,
            customer TEXT NOT NULL,
            channel TEXT NOT NULL,
            reason TEXT NOT NULL,
            message TEXT NOT NULL,
            status TEXT NOT NULL,
            context_summary TEXT NOT NULL,
            ai_attempts INTEGER DEFAULT 0,
            created_at TEXT NOT NULL
        )",
        [],
    ).unwrap();

    let mut stmt = conn.prepare("SELECT id, customer, channel, reason, message, status, context_summary, ai_attempts, created_at FROM handoff_queue ORDER BY created_at DESC").unwrap();
    
    let mut queue = Vec::new();
    let iter = stmt.query_map([], |row| {
        Ok(HandoffTicket {
            id: row.get(0)?,
            customer: row.get(1)?,
            channel: row.get(2)?,
            reason: row.get(3)?,
            message: row.get(4)?,
            status: row.get(5)?,
            context_summary: row.get(6)?,
            ai_attempts: row.get(7)?,
            created_at: row.get(8)?,
        })
    }).unwrap();

    for t in iter {
        if let Ok(ticket) = t {
            queue.push(ticket);
        }
    }

    Json(serde_json::json!({ "queue": queue }))
}

pub async fn resolve_handoff(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    let conn = match state.db.lock_conn() {
        Ok(c) => c,
        Err(e) => return Json(serde_json::json!({"error": format!("DB error: {e}")}))
    };
    conn.execute("UPDATE handoff_queue SET status = 'resolved' WHERE id = ?1", params![id]).unwrap();

    let mut activity_log = state.activity_log.lock().unwrap();
    activity_log.push(crate::openai_compat::ActivityEvent {
        timestamp: chrono::Utc::now(),
        event_type: "handoff_resolved".into(),
        agent: "Human".into(),
        detail: format!("Nhân viên đã xử lý ticket Handoff: {}", id),
    });

    Json(serde_json::json!({ "status": "resolved" }))
}
