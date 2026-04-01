//! Brain Workspace API route handlers.
//!
//! CRUD operations for the Brain memory workspace files
//! (SOUL.md, IDENTITY.md, USER.md, etc.) plus AI-powered personalization.
//! Extracted from routes/mod.rs.

use axum::{Json, extract::State};
use std::sync::Arc;

use super::helpers::internal_error;
use crate::server::AppState;

/// List all brain files in the workspace.
/// If `?tenant=slug` provided, uses per-tenant workspace.
pub async fn brain_list_files(
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<serde_json::Value> {
    let ws = match params.get("tenant") {
        Some(slug) if !slug.is_empty() => bizclaw_memory::brain::BrainWorkspace::for_tenant(slug),
        _ => bizclaw_memory::brain::BrainWorkspace::default(),
    };
    let _ = ws.initialize(); // ensure files exist
    let files = ws.list_files();
    let base_dir = ws.base_dir().display().to_string();
    Json(serde_json::json!({
        "ok": true,
        "files": files,
        "base_dir": base_dir,
        "count": files.len(),
    }))
}

/// Read a specific brain file.
pub async fn brain_read_file(
    axum::extract::Path(filename): axum::extract::Path<String>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<serde_json::Value> {
    let ws = match params.get("tenant") {
        Some(slug) if !slug.is_empty() => bizclaw_memory::brain::BrainWorkspace::for_tenant(slug),
        _ => bizclaw_memory::brain::BrainWorkspace::default(),
    };
    match ws.read_file(&filename) {
        Some(content) => Json(serde_json::json!({
            "ok": true, "filename": filename, "content": content, "size": content.len(),
        })),
        None => {
            Json(serde_json::json!({"ok": false, "error": format!("File not found: {filename}")}))
        }
    }
}

/// Write (create/update) a brain file.
pub async fn brain_write_file(
    axum::extract::Path(filename): axum::extract::Path<String>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let ws = match params.get("tenant") {
        Some(slug) if !slug.is_empty() => bizclaw_memory::brain::BrainWorkspace::for_tenant(slug),
        _ => bizclaw_memory::brain::BrainWorkspace::default(),
    };
    let content = body["content"].as_str().unwrap_or("");
    match ws.write_file(&filename, content) {
        Ok(()) => Json(serde_json::json!({"ok": true, "message": format!("Saved: {filename}")})),
        Err(e) => internal_error("gateway", e),
    }
}

/// Delete a brain file.
pub async fn brain_delete_file(
    axum::extract::Path(filename): axum::extract::Path<String>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<serde_json::Value> {
    let ws = match params.get("tenant") {
        Some(slug) if !slug.is_empty() => bizclaw_memory::brain::BrainWorkspace::for_tenant(slug),
        _ => bizclaw_memory::brain::BrainWorkspace::default(),
    };
    match ws.delete_file(&filename) {
        Ok(true) => {
            Json(serde_json::json!({"ok": true, "message": format!("Deleted: {filename}")}))
        }
        Ok(false) => Json(serde_json::json!({"ok": false, "error": "File not found"})),
        Err(e) => internal_error("gateway", e),
    }
}

/// Brain Personalization — AI generates SOUL.md, IDENTITY.md, USER.md from user description.
pub async fn brain_personalize(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let about_user = body["about_user"].as_str().unwrap_or("");
    let agent_vibe = body["agent_vibe"]
        .as_str()
        .unwrap_or("helpful and professional");
    let agent_name = body["agent_name"].as_str().unwrap_or("BizClaw Agent");
    let language = body["language"].as_str().unwrap_or("vi");
    let tenant = body["tenant"].as_str().unwrap_or("");

    if about_user.is_empty() {
        return Json(
            serde_json::json!({"ok": false, "error": "Please describe yourself (about_user)"}),
        );
    }

    // Build the AI prompt
    let prompt = format!(
        r#"You are a configuration assistant. Based on the user's description below, generate personalized brain files for an AI agent.

User describes themselves: "{about_user}"
Desired agent personality/vibe: "{agent_vibe}"
Agent name: "{agent_name}"
Language: "{language}"

Generate EXACTLY these 3 files. Output as JSON with keys "soul", "identity", "user". Each value is the markdown content for that file.

SOUL.md should define the agent's personality, tone, and behavioral rules based on the desired vibe.
IDENTITY.md should define the agent's name, role, and style.
USER.md should capture key facts about the user for personalization.

Output ONLY valid JSON, no markdown fences."#
    );

    // Send to agent
    let mut agent_lock = state.agent.lock().await;
    let response = match agent_lock.as_mut() {
        Some(agent) => match agent.process(&prompt).await {
            Ok(r) => r,
            Err(e) => {
                tracing::error!("[security] Agent processing error: {e}");
                return Json(
                    serde_json::json!({"ok": false, "error": "AI processing error — please try again"}),
                );
            }
        },
        None => {
            return Json(
                serde_json::json!({"ok": false, "error": "Agent not available — configure provider first"}),
            );
        }
    };
    drop(agent_lock);

    // Parse AI response as JSON
    let clean = response
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();
    let parsed: serde_json::Value = match serde_json::from_str(clean) {
        Ok(v) => v,
        Err(_) => {
            // Fallback: try to extract JSON from response
            let start = clean.find('{').unwrap_or(0);
            let end = clean.rfind('}').map(|i| i + 1).unwrap_or(clean.len());
            match serde_json::from_str(&clean[start..end]) {
                Ok(v) => v,
                Err(e) => {
                    return Json(serde_json::json!({
                        "ok": false,
                        "error": format!("Failed to parse AI response: {e}"),
                        "raw": response,
                    }));
                }
            }
        }
    };

    // Save to workspace
    let ws = if tenant.is_empty() {
        bizclaw_memory::brain::BrainWorkspace::default()
    } else {
        bizclaw_memory::brain::BrainWorkspace::for_tenant(tenant)
    };
    let _ = ws.initialize();

    let mut saved = Vec::new();
    for (key, filename) in &[
        ("soul", "SOUL.md"),
        ("identity", "IDENTITY.md"),
        ("user", "USER.md"),
    ] {
        if let Some(content) = parsed[key].as_str()
            && ws.write_file(filename, content).is_ok()
        {
            saved.push(*filename);
        }
    }

    tracing::info!("🎨 Brain personalized: {} files saved", saved.len());
    Json(serde_json::json!({
        "ok": true,
        "saved": saved,
        "files": {
            "soul": parsed["soul"].as_str().unwrap_or(""),
            "identity": parsed["identity"].as_str().unwrap_or(""),
            "user": parsed["user"].as_str().unwrap_or(""),
        },
    }))
}
