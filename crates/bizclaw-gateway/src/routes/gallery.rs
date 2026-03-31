//! Gallery API route handlers — manage skill templates.
//!
//! Extracted from mod.rs to reduce God File size.
//! Handles: gallery_list, gallery_create, gallery_delete, gallery_upload_md, gallery_get_md.

use axum::{Json, extract::State};
use std::sync::Arc;

use super::internal_error;
use crate::server::AppState;

/// List all gallery skills (built-in + user-created).
pub async fn gallery_list(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let gallery_path = state
        .config_path
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .join("gallery.json");

    // Load built-in skills from embedded data
    let builtin: Vec<serde_json::Value> =
        serde_json::from_str(include_str!("../../../../data/gallery-skills.json"))
            .unwrap_or_default();

    // Load user-created skills
    let user_skills: Vec<serde_json::Value> = if gallery_path.exists() {
        std::fs::read_to_string(&gallery_path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    } else {
        Vec::new()
    };

    // Check which skills have attached MD files
    let skills_dir = state
        .config_path
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .join("skills");

    let mut all_skills: Vec<serde_json::Value> = builtin
        .into_iter()
        .map(|mut s| {
            s.as_object_mut()
                .map(|o| o.insert("source".into(), "builtin".into()));
            s
        })
        .collect();

    for mut s in user_skills {
        s.as_object_mut()
            .map(|o| o.insert("source".into(), "user".into()));
        all_skills.push(s);
    }

    // Check for attached MD files
    for skill in &mut all_skills {
        if let Some(id) = skill["id"].as_str() {
            let md_path = skills_dir.join(format!("{}.md", id));
            if md_path.exists() {
                skill
                    .as_object_mut()
                    .map(|o| o.insert("has_md".into(), true.into()));
            }
        }
    }

    Json(serde_json::json!({
        "ok": true,
        "skills": all_skills,
        "total": all_skills.len(),
    }))
}

/// Create a custom gallery skill.
pub async fn gallery_create(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let gallery_path = state
        .config_path
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .join("gallery.json");

    let mut skills: Vec<serde_json::Value> = if gallery_path.exists() {
        std::fs::read_to_string(&gallery_path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    } else {
        Vec::new()
    };

    let id = body["id"].as_str().unwrap_or("custom").to_string();

    // Check for duplicate
    if skills.iter().any(|s| s["id"].as_str() == Some(&id)) {
        return Json(
            serde_json::json!({"ok": false, "error": format!("Skill '{}' already exists", id)}),
        );
    }

    skills.push(body.clone());

    if let Ok(json) = serde_json::to_string_pretty(&skills) {
        let _ = std::fs::write(&gallery_path, json);
    }

    Json(serde_json::json!({"ok": true, "id": id, "total": skills.len()}))
}

/// Delete a custom gallery skill.
pub async fn gallery_delete(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    let gallery_path = state
        .config_path
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .join("gallery.json");

    let mut skills: Vec<serde_json::Value> = if gallery_path.exists() {
        std::fs::read_to_string(&gallery_path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    } else {
        Vec::new()
    };

    let before = skills.len();
    skills.retain(|s| s["id"].as_str() != Some(&id));
    let removed = before != skills.len();

    if removed {
        if let Ok(json) = serde_json::to_string_pretty(&skills) {
            let _ = std::fs::write(&gallery_path, json);
        }
        // Also remove any attached MD file
        let skills_dir = state
            .config_path
            .parent()
            .unwrap_or(std::path::Path::new("."))
            .join("skills");
        let md_path = skills_dir.join(format!("{}.md", id));
        let _ = std::fs::remove_file(md_path);
    }

    Json(serde_json::json!({"ok": removed, "id": id}))
}

/// Upload an MD file for a gallery skill.
pub async fn gallery_upload_md(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
    body: String,
) -> Json<serde_json::Value> {
    let skills_dir = state
        .config_path
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .join("skills");
    let _ = std::fs::create_dir_all(&skills_dir);

    let md_path = skills_dir.join(format!("{}.md", id));
    match std::fs::write(&md_path, &body) {
        Ok(_) => {
            tracing::info!("📄 Uploaded skill MD: {}.md ({} bytes)", id, body.len());
            Json(serde_json::json!({
                "ok": true,
                "id": id,
                "size": body.len(),
                "path": md_path.display().to_string(),
            }))
        }
        Err(e) => internal_error("gateway", e),
    }
}

/// Get the MD content for a gallery skill.
pub async fn gallery_get_md(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    let skills_dir = state
        .config_path
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .join("skills");
    let md_path = skills_dir.join(format!("{}.md", id));

    if md_path.exists() {
        let content = std::fs::read_to_string(&md_path).unwrap_or_default();
        Json(serde_json::json!({"ok": true, "id": id, "content": content}))
    } else {
        Json(serde_json::json!({"ok": false, "error": "MD file not found"}))
    }
}
