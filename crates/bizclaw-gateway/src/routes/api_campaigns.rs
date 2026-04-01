use axum::{
    extract::{Path, State},
    Json,
};
use bizclaw_core::error::{BizClawError, Result};
use bizclaw_core::types::AuthUser;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::server::AppState;

#[derive(Debug, Serialize, Deserialize)]
pub struct Campaign {
    pub id: String,
    pub name: String,
    pub channels: Vec<String>,
    pub segment: String,
    pub message: String,
    pub status: String,
    pub sent: i64,
    pub delivered: i64,
    pub read: i64,
    pub created_at: String,
    pub schedule_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateCampaignRequest {
    pub name: String,
    pub channels: Vec<String>,
    pub segment: String,
    pub message: String,
    pub status: String,
    pub schedule_at: Option<String>,
}

pub async fn list_campaigns(
    State(state): State<Arc<AppState>>,
    axum::extract::Extension(user): axum::extract::Extension<AuthUser>,
) -> Result<Json<serde_json::Value>> {
    let conn = state.db.get_conn().map_err(|e| BizClawError::Api(format!("DB error: {e}")))?;
    
    // Ensure table exists
    conn.execute(
        "CREATE TABLE IF NOT EXISTS campaigns (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            channels TEXT NOT NULL,
            segment TEXT NOT NULL,
            message TEXT NOT NULL,
            status TEXT NOT NULL,
            sent INTEGER DEFAULT 0,
            delivered INTEGER DEFAULT 0,
            read_count INTEGER DEFAULT 0,
            created_at TEXT NOT NULL,
            schedule_at TEXT
        )",
        [],
    ).map_err(|e| BizClawError::Api(format!("DB error: {e}")))?;

    let mut stmt = conn.prepare("SELECT id, name, channels, segment, message, status, sent, delivered, read_count, created_at, schedule_at FROM campaigns ORDER BY created_at DESC").unwrap();
    
    let mut campaigns = Vec::new();
    let iter = stmt.query_map([], |row| {
        let channels_str: String = row.get(2)?;
        let channels: Vec<String> = serde_json::from_str(&channels_str).unwrap_or_default();
        Ok(Campaign {
            id: row.get(0)?,
            name: row.get(1)?,
            channels,
            segment: row.get(3)?,
            message: row.get(4)?,
            status: row.get(5)?,
            sent: row.get(6)?,
            delivered: row.get(7)?,
            read: row.get(8)?,
            created_at: row.get(9)?,
            schedule_at: row.get(10)?,
        })
    }).unwrap();

    for c in iter {
        if let Ok(cam) = c {
            campaigns.push(cam);
        }
    }

    Ok(Json(serde_json::json!({ "campaigns": campaigns })))
}

pub async fn create_campaign(
    State(state): State<Arc<AppState>>,
    axum::extract::Extension(user): axum::extract::Extension<AuthUser>,
    Json(req): Json<CreateCampaignRequest>,
) -> Result<Json<serde_json::Value>> {
    let conn = state.db.get_conn().map_err(|e| BizClawError::Api(format!("DB error: {e}")))?;
    let id = Uuid::new_v4().to_string();
    let created_at = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let channels_json = serde_json::to_string(&req.channels).unwrap_or_default();

    conn.execute(
        "INSERT INTO campaigns (id, name, channels, segment, message, status, created_at, schedule_at) 
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![id, req.name, channels_json, req.segment, req.message, req.status, created_at, req.schedule_at],
    ).map_err(|e| BizClawError::Api(format!("Failed to insert campaign: {e}")))?;

    let mut activity_log = state.activity_log.lock().unwrap();
    activity_log.push(crate::openai_compat::ActivityEvent {
        id: Uuid::new_v4().to_string(),
        timestamp: created_at,
        source: "campaigns".into(),
        action: "create".into(),
        details: format!("Tạo chiến dịch mới: {}", req.name),
        status: "success".into(),
    });

    Ok(Json(serde_json::json!({ "id": id, "status": "success" })))
}

pub async fn delete_campaign(
    State(state): State<Arc<AppState>>,
    axum::extract::Extension(user): axum::extract::Extension<AuthUser>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>> {
    let conn = state.db.get_conn().map_err(|e| BizClawError::Api(format!("DB error: {e}")))?;
    conn.execute("DELETE FROM campaigns WHERE id = ?1", params![id])
        .map_err(|e| BizClawError::Api(format!("DB error: {e}")))?;

    Ok(Json(serde_json::json!({ "status": "deleted" })))
}

pub async fn run_campaign(
    State(state): State<Arc<AppState>>,
    axum::extract::Extension(user): axum::extract::Extension<AuthUser>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>> {
    let conn = state.db.get_conn().map_err(|e| BizClawError::Api(format!("DB error: {e}")))?;
    
    // Update status to running
    conn.execute("UPDATE campaigns SET status = 'running' WHERE id = ?1", params![id])
        .map_err(|e| BizClawError::Api(format!("DB error: {e}")))?;

    // Ideally, we start a background tokio::spawn routine here to use AutoMessageTool
    // For now, we update it and the UI optimistic update takes over
    
    Ok(Json(serde_json::json!({ "status": "running" })))
}
