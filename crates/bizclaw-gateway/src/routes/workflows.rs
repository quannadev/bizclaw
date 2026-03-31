//! Workflows, Skills, and Tools API route handlers.
//!
//! Extracted from routes/mod.rs to reduce god-file complexity.

use axum::{Json, extract::State};
use std::sync::Arc;

use crate::server::AppState;
use super::helpers::internal_error;

// ═══ Workflows API ═══

/// List available workflow templates.
pub async fn workflows_list(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    // Built-in templates
    let mut workflows = vec![
        serde_json::json!({"id":"content_pipeline","name":"Content Pipeline","description":"Draft → Review → Polish","tags":["content","writing"],"builtin":true,"steps":[
            {"name":"Draft","type":"Sequential","agent_role":"Writer","prompt":""},
            {"name":"Review","type":"Sequential","agent_role":"Editor","prompt":""},
            {"name":"Polish","type":"Sequential","agent_role":"Proofreader","prompt":""},
        ]}),
        serde_json::json!({"id":"expert_consensus","name":"Expert Consensus","description":"3 experts analyze in parallel → Merge","tags":["analysis","multi-agent"],"builtin":true,"steps":[
            {"name":"Expert Analysis","type":"FanOut","agent_role":"3 Experts (parallel)","prompt":""},
            {"name":"Merge Results","type":"Collect","agent_role":"Synthesizer","prompt":""},
        ]}),
        serde_json::json!({"id":"quality_pipeline","name":"Quality Gate","description":"Generate → Loop evaluate until APPROVED","tags":["quality","loop"],"builtin":true,"steps":[
            {"name":"Generate","type":"Sequential","agent_role":"Creator","prompt":""},
            {"name":"Evaluate","type":"Loop","agent_role":"Evaluator (until APPROVED)","prompt":""},
        ]}),
        serde_json::json!({"id":"research_pipeline","name":"Research Pipeline","description":"Search → Analyze → Synthesize → Report","tags":["research","data"],"builtin":true,"steps":[
            {"name":"Search","type":"Sequential","agent_role":"Researcher","prompt":""},
            {"name":"Analyze","type":"Sequential","agent_role":"Analyst","prompt":""},
            {"name":"Synthesize","type":"Sequential","agent_role":"Writer","prompt":""},
            {"name":"Report","type":"Transform","agent_role":"Formatter","prompt":""},
        ]}),
        serde_json::json!({"id":"translation_pipeline","name":"Translation Pipeline","description":"Translate → Quality verification","tags":["language","translation"],"builtin":true,"steps":[
            {"name":"Translate","type":"Sequential","agent_role":"Translator","prompt":""},
            {"name":"Verify Quality","type":"Conditional","agent_role":"QA Checker","prompt":""},
        ]}),
        serde_json::json!({"id":"code_review","name":"Code Review Pipeline","description":"3 reviewers in parallel → Security → Summary","tags":["code","security"],"builtin":true,"steps":[
            {"name":"Code Analysis","type":"FanOut","agent_role":"3 Reviewers (parallel)","prompt":""},
            {"name":"Security Check","type":"Sequential","agent_role":"Security Auditor","prompt":""},
            {"name":"Summary","type":"Collect","agent_role":"Lead Reviewer","prompt":""},
        ]}),
    ];

    // Load user-created workflows from disk
    let wf_dir = state
        .config_path
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .join("workflows");
    if wf_dir.exists()
        && let Ok(entries) = std::fs::read_dir(&wf_dir)
    {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "json").unwrap_or(false)
                && let Ok(content) = std::fs::read_to_string(&path)
                && let Ok(wf) = serde_json::from_str::<serde_json::Value>(&content)
            {
                workflows.push(wf);
            }
        }
    }

    Json(serde_json::json!({"ok": true, "workflows": workflows}))
}

/// Create a new workflow.
pub async fn workflows_create(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let name = body["name"].as_str().unwrap_or("Untitled Workflow");
    let description = body["description"].as_str().unwrap_or("");
    let tags: Vec<String> = body["tags"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    let steps = body["steps"].clone();
    let input_prompt = body["input_prompt"].as_str().unwrap_or("");

    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let id = format!("wf-{:x}-{:x}", ts as u64, std::process::id());

    let workflow = serde_json::json!({
        "id": id,
        "name": name,
        "description": description,
        "tags": tags,
        "steps": steps,
        "input_prompt": input_prompt,
        "builtin": false,
        "created_at": chrono::Utc::now().to_rfc3339(),
    });

    // Save to disk
    let wf_dir = state
        .config_path
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .join("workflows");
    let _ = std::fs::create_dir_all(&wf_dir);
    let path = wf_dir.join(format!("{}.json", id));
    if let Ok(json) = serde_json::to_string_pretty(&workflow) {
        let _ = std::fs::write(&path, json);
    }

    tracing::info!("✅ Workflow created: {} ({})", name, id);
    Json(serde_json::json!({"ok": true, "id": id, "workflow": workflow}))
}

/// Update an existing workflow.
pub async fn workflows_update(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let wf_dir = state
        .config_path
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .join("workflows");
    let path = wf_dir.join(format!("{}.json", id));

    if !path.exists() {
        return Json(serde_json::json!({"ok": false, "error": "Workflow not found"}));
    }

    let mut workflow: serde_json::Value = std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or(serde_json::json!({}));

    // Merge updates
    if let Some(name) = body["name"].as_str() {
        workflow["name"] = serde_json::json!(name);
    }
    if let Some(desc) = body["description"].as_str() {
        workflow["description"] = serde_json::json!(desc);
    }
    if body.get("steps").is_some() {
        workflow["steps"] = body["steps"].clone();
    }
    if body.get("tags").is_some() {
        workflow["tags"] = body["tags"].clone();
    }
    if let Some(ip) = body["input_prompt"].as_str() {
        workflow["input_prompt"] = serde_json::json!(ip);
    }
    workflow["updated_at"] = serde_json::json!(chrono::Utc::now().to_rfc3339());

    if let Ok(json) = serde_json::to_string_pretty(&workflow) {
        let _ = std::fs::write(&path, json);
    }

    tracing::info!("✅ Workflow updated: {}", id);
    Json(serde_json::json!({"ok": true, "workflow": workflow}))
}

/// Delete a workflow.
pub async fn workflows_delete(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    let wf_dir = state
        .config_path
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .join("workflows");
    let path = wf_dir.join(format!("{}.json", id));

    if path.exists() {
        let _ = std::fs::remove_file(&path);
        tracing::info!("🗑️ Workflow deleted: {}", id);
        Json(serde_json::json!({"ok": true}))
    } else {
        Json(
            serde_json::json!({"ok": false, "error": "Workflow not found or is a built-in template"}),
        )
    }
}

/// Run a workflow — execute steps sequentially through the agent.
pub async fn workflows_run(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let workflow_id = body["workflow_id"].as_str().unwrap_or("");
    let input = body["input"].as_str().unwrap_or("");

    if workflow_id.is_empty() {
        return Json(serde_json::json!({"ok": false, "error": "workflow_id is required"}));
    }

    // Find the workflow (check user files first, then built-in templates)
    let wf_dir = state
        .config_path
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .join("workflows");
    let user_path = wf_dir.join(format!("{}.json", workflow_id));

    let workflow: Option<serde_json::Value> = if user_path.exists() {
        std::fs::read_to_string(&user_path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
    } else {
        // Check built-in templates
        let list_resp = workflows_list(State(state.clone())).await;
        let wfs = list_resp.0["workflows"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        wfs.into_iter()
            .find(|w| w["id"].as_str() == Some(workflow_id))
    };

    let workflow = match workflow {
        Some(wf) => wf,
        None => {
            return Json(
                serde_json::json!({"ok": false, "error": format!("Workflow '{}' not found", workflow_id)}),
            );
        }
    };

    let steps = workflow["steps"].as_array().cloned().unwrap_or_default();
    let wf_name = workflow["name"].as_str().unwrap_or(workflow_id);

    tracing::info!(
        "▶ Running workflow '{}' ({} steps), input: {:?}",
        wf_name,
        steps.len(),
        input
    );

    let mut results: Vec<serde_json::Value> = Vec::new();
    let mut current_input = input.to_string();

    for (i, step) in steps.iter().enumerate() {
        let step_name = step["name"].as_str().unwrap_or("Step");
        let agent_role = step["agent_role"].as_str().unwrap_or("Agent");
        let step_prompt = step["prompt"].as_str().unwrap_or("");

        let prompt = if step_prompt.is_empty() {
            format!(
                "[Workflow: {} | Step {}: {} | Role: {}]\n\nPrevious context:\n{}\n\nPlease complete this step as the {} role.",
                wf_name,
                i + 1,
                step_name,
                agent_role,
                current_input,
                agent_role
            )
        } else {
            format!("{}\n\nInput:\n{}", step_prompt, current_input)
        };

        tracing::info!(
            "  → Step {}/{}: {} ({})",
            i + 1,
            steps.len(),
            step_name,
            agent_role
        );

        let response = {
            let mut orch = state.orchestrator.lock().await;
            let target = agent_role.to_lowercase().replace(" ", "_");
            
            // Try explicit routing, fallback to default agent
            let res = orch.send_to(&target, &prompt).await;
            let res = match res {
                Ok(r) => Ok(r),
                Err(bizclaw_core::error::BizClawError::AgentNotFound(_)) => {
                    tracing::warn!("Agent '{}' not found, falling back to default orchestration", target);
                    orch.send(&prompt).await
                }
                Err(e) => Err(e),
            };

            match res {
                Ok(r) => r,
                Err(e) => format!("Error in step '{}': {}", step_name, e),
            }
        };

        results.push(serde_json::json!({
            "step": i + 1,
            "name": step_name,
            "agent_role": agent_role,
            "output": response,
        }));

        current_input = response;
    }

    tracing::info!(
        "✅ Workflow '{}' completed ({} steps)",
        wf_name,
        results.len()
    );

    Json(serde_json::json!({
        "ok": true,
        "workflow": wf_name,
        "steps_completed": results.len(),
        "results": results,
        "final_output": current_input,
    }))
}

// ═══ Skills API ═══

fn skills_dir(state: &AppState) -> std::path::PathBuf {
    state
        .config_path
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .join("skills")
}

fn installed_skills_path(state: &AppState) -> std::path::PathBuf {
    state
        .config_path
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .join("skills-installed.json")
}

fn load_installed_set(state: &AppState) -> std::collections::HashSet<String> {
    let path = installed_skills_path(state);
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str::<Vec<String>>(&s).ok())
        .unwrap_or_default()
        .into_iter()
        .collect()
}

fn save_installed_set(state: &AppState, set: &std::collections::HashSet<String>) {
    let path = installed_skills_path(state);
    let list: Vec<&String> = set.iter().collect();
    if let Ok(json) = serde_json::to_string_pretty(&list) {
        let _ = std::fs::write(&path, json);
    }
}

/// List available skills (built-in + user-created).
pub async fn skills_list(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let installed = load_installed_set(&state);

    let builtin = vec![
        serde_json::json!({"id":"rust-expert","name":"Rust Expert","icon":"🦀","category":"coding","tags":["rust","systems","performance"],"version":"1.0.0","description":"Rust expert: ownership, async, performance tuning","builtin":true,
            "system_prompt":"You are a Rust programming expert. Help with ownership, borrowing, lifetimes, async/await, error handling, and performance optimization. Write idiomatic, safe Rust code."}),
        serde_json::json!({"id":"python-analyst","name":"Python Analyst","icon":"🐍","category":"data","tags":["python","pandas","visualization"],"version":"1.0.0","description":"Python data analysis: pandas, numpy, visualization","builtin":true,
            "system_prompt":"You are a Python data analyst. Expert in pandas, numpy, matplotlib, seaborn, scikit-learn. Help with data cleaning, analysis, visualization, and machine learning."}),
        serde_json::json!({"id":"web-developer","name":"Web Developer","icon":"🌐","category":"coding","tags":["react","typescript","css"],"version":"1.0.0","description":"Full-stack web: React, TypeScript, CSS, Node.js","builtin":true,
            "system_prompt":"You are a full-stack web developer. Expert in React, TypeScript, CSS, Node.js, Next.js. Build responsive, accessible, performant web applications."}),
        serde_json::json!({"id":"devops-engineer","name":"DevOps Engineer","icon":"🔧","category":"devops","tags":["docker","k8s","ci-cd"],"version":"1.0.0","description":"DevOps: Docker, K8s, CI/CD, Terraform","builtin":true,
            "system_prompt":"You are a DevOps engineer. Expert in Docker, Kubernetes, CI/CD pipelines, Terraform, monitoring, and cloud infrastructure."}),
        serde_json::json!({"id":"content-writer","name":"Content Writer","icon":"✍️","category":"writing","tags":["blog","seo","marketing"],"version":"1.0.0","description":"Content writing: blog, marketing, SEO","builtin":true,
            "system_prompt":"You are a professional content writer. Expert in blog posts, marketing copy, SEO optimization, and engaging storytelling."}),
        serde_json::json!({"id":"security-auditor","name":"Security Auditor","icon":"🔒","category":"security","tags":["owasp","pentest","review"],"version":"1.0.0","description":"Security auditing: OWASP Top 10, code review","builtin":true,
            "system_prompt":"You are a security auditor. Expert in OWASP Top 10, penetration testing, code review for vulnerabilities, and security best practices."}),
        serde_json::json!({"id":"sql-expert","name":"SQL Expert","icon":"🗄️","category":"data","tags":["postgresql","sqlite","optimization"],"version":"1.0.0","description":"SQL expert: PostgreSQL, optimization, window functions","builtin":true,
            "system_prompt":"You are an SQL expert. Expert in PostgreSQL, SQLite, query optimization, window functions, CTEs, and database design."}),
        serde_json::json!({"id":"api-designer","name":"API Designer","icon":"🔌","category":"coding","tags":["rest","openapi","auth"],"version":"1.0.0","description":"API design: REST, OpenAPI, authentication","builtin":true,
            "system_prompt":"You are an API designer. Expert in REST, OpenAPI/Swagger, authentication (OAuth2, JWT), rate limiting, and API versioning."}),
        serde_json::json!({"id":"vietnamese-business","name":"Vietnamese Business","icon":"🇻🇳","category":"business","tags":["tax","labor","accounting"],"version":"1.0.0","description":"Vietnamese business: tax law, labor, accounting","builtin":true,
            "system_prompt":"Bạn là chuyên gia kinh doanh Việt Nam. Tư vấn về luật thuế, luật lao động, kế toán, bảo hiểm xã hội, hợp đồng, và quy định doanh nghiệp."}),
        serde_json::json!({"id":"git-workflow","name":"Git Workflow","icon":"📦","category":"devops","tags":["git","branching","review"],"version":"1.0.0","description":"Git workflow: branching, commits, code review","builtin":true,
            "system_prompt":"You are a Git workflow expert. Help with branching strategies, commit conventions, code review, merge conflicts, and CI/CD integration."}),
    ];

    let mut skills: Vec<serde_json::Value> = builtin
        .into_iter()
        .map(|mut s| {
            let id = s["id"].as_str().unwrap_or("").to_string();
            s["installed"] = serde_json::json!(installed.contains(&id));
            s
        })
        .collect();

    // Load user-created skills
    let dir = skills_dir(&state);
    if dir.exists()
        && let Ok(entries) = std::fs::read_dir(&dir)
    {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "json").unwrap_or(false)
                && let Ok(content) = std::fs::read_to_string(&path)
                && let Ok(mut sk) = serde_json::from_str::<serde_json::Value>(&content)
            {
                let id = sk["id"].as_str().unwrap_or("").to_string();
                sk["installed"] = serde_json::json!(installed.contains(&id));
                skills.push(sk);
            }
        }
    }

    Json(serde_json::json!({"ok": true, "skills": skills}))
}

/// Create a new custom skill.
pub async fn skills_create(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let name = body["name"].as_str().unwrap_or("Untitled Skill");
    let icon = body["icon"].as_str().unwrap_or("🧩");
    let category = body["category"].as_str().unwrap_or("custom");
    let description = body["description"].as_str().unwrap_or("");
    let system_prompt = body["system_prompt"].as_str().unwrap_or("");
    let tags: Vec<String> = body["tags"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let id = format!("skill-{:x}-{:x}", ts as u64, std::process::id());

    let skill = serde_json::json!({
        "id": id,
        "name": name,
        "icon": icon,
        "category": category,
        "description": description,
        "system_prompt": system_prompt,
        "tags": tags,
        "version": "1.0.0",
        "builtin": false,
        "created_at": chrono::Utc::now().to_rfc3339(),
    });

    let dir = skills_dir(&state);
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join(format!("{}.json", id));
    if let Ok(json) = serde_json::to_string_pretty(&skill) {
        let _ = std::fs::write(&path, json);
    }

    tracing::info!("✅ Skill created: {} ({})", name, id);
    Json(serde_json::json!({"ok": true, "id": id, "skill": skill}))
}

/// Update a custom skill.
pub async fn skills_update(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let dir = skills_dir(&state);
    let path = dir.join(format!("{}.json", id));
    if !path.exists() {
        return Json(serde_json::json!({"ok": false, "error": "Skill not found or is built-in"}));
    }

    let mut skill: serde_json::Value = std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or(serde_json::json!({}));

    if let Some(v) = body["name"].as_str() {
        skill["name"] = serde_json::json!(v);
    }
    if let Some(v) = body["icon"].as_str() {
        skill["icon"] = serde_json::json!(v);
    }
    if let Some(v) = body["category"].as_str() {
        skill["category"] = serde_json::json!(v);
    }
    if let Some(v) = body["description"].as_str() {
        skill["description"] = serde_json::json!(v);
    }
    if let Some(v) = body["system_prompt"].as_str() {
        skill["system_prompt"] = serde_json::json!(v);
    }
    if body.get("tags").is_some() {
        skill["tags"] = body["tags"].clone();
    }
    skill["updated_at"] = serde_json::json!(chrono::Utc::now().to_rfc3339());

    if let Ok(json) = serde_json::to_string_pretty(&skill) {
        let _ = std::fs::write(&path, json);
    }

    tracing::info!("✅ Skill updated: {}", id);
    Json(serde_json::json!({"ok": true, "skill": skill}))
}

/// Delete a custom skill.
pub async fn skills_delete(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    let dir = skills_dir(&state);
    let path = dir.join(format!("{}.json", id));
    if path.exists() {
        let _ = std::fs::remove_file(&path);
        tracing::info!("🗑️ Skill deleted: {}", id);
        Json(serde_json::json!({"ok": true}))
    } else {
        Json(serde_json::json!({"ok": false, "error": "Skill not found or is built-in"}))
    }
}

/// Install a skill (toggle installed state).
pub async fn skills_install(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let skill_id = body["skill"].as_str().unwrap_or("");
    if skill_id.is_empty() {
        return Json(serde_json::json!({"ok": false, "error": "skill ID is required"}));
    }
    let mut installed = load_installed_set(&state);
    installed.insert(skill_id.to_string());
    save_installed_set(&state, &installed);
    tracing::info!("✅ Skill installed: {}", skill_id);
    Json(serde_json::json!({"ok": true}))
}

/// Uninstall a skill.
pub async fn skills_uninstall(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let skill_id = body["skill"].as_str().unwrap_or("");
    if skill_id.is_empty() {
        return Json(serde_json::json!({"ok": false, "error": "skill ID is required"}));
    }
    let mut installed = load_installed_set(&state);
    installed.remove(skill_id);
    save_installed_set(&state, &installed);
    tracing::info!("🗑️ Skill uninstalled: {}", skill_id);
    Json(serde_json::json!({"ok": true}))
}

/// Get details of a single skill.
pub async fn skills_detail(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    // Check user skills first
    let dir = skills_dir(&state);
    let path = dir.join(format!("{}.json", id));
    if path.exists()
        && let Ok(content) = std::fs::read_to_string(&path)
        && let Ok(skill) = serde_json::from_str::<serde_json::Value>(&content)
    {
        return Json(serde_json::json!({"ok": true, "skill": skill}));
    }
    // Check built-in
    let list_resp = skills_list(State(state.clone())).await;
    let skills = list_resp.0["skills"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    if let Some(skill) = skills.into_iter().find(|s| s["id"].as_str() == Some(&id)) {
        return Json(serde_json::json!({"ok": true, "skill": skill}));
    }
    Json(serde_json::json!({"ok": false, "error": "Skill not found"}))
}

// ═══════════════════════════════════════════════════════════════════════
// TOOLS CRUD — custom tools stored in ~/.bizclaw/tools/
// ═══════════════════════════════════════════════════════════════════════

fn tools_dir(state: &AppState) -> std::path::PathBuf {
    let dir = state
        .config_path
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .join("tools");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

/// List all tools (built-in + custom)
pub async fn tools_list(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let mut tools: Vec<serde_json::Value> = vec![
        serde_json::json!({"name":"shell","icon":"🖥️","desc":"Execute system commands (sandboxed)","enabled":true,"builtin":true}),
        serde_json::json!({"name":"file","icon":"📁","desc":"Read/write/list files","enabled":true,"builtin":true}),
        serde_json::json!({"name":"edit_file","icon":"✏️","desc":"Precise text replacements in files","enabled":true,"builtin":true}),
        serde_json::json!({"name":"glob","icon":"🔍","desc":"Find files by pattern","enabled":true,"builtin":true}),
        serde_json::json!({"name":"grep","icon":"🔎","desc":"Search file contents with regex","enabled":true,"builtin":true}),
        serde_json::json!({"name":"http_request","icon":"🌐","desc":"Call external APIs","enabled":true,"builtin":true}),
        serde_json::json!({"name":"web_search","icon":"🔍","desc":"DuckDuckGo, SearXNG","enabled":true,"builtin":true}),
        serde_json::json!({"name":"plan","icon":"📋","desc":"Task decomposition with dependencies","enabled":true,"builtin":true}),
        serde_json::json!({"name":"session_context","icon":"📊","desc":"Session info: provider, tokens, tools","enabled":true,"builtin":true}),
        serde_json::json!({"name":"config_manager","icon":"⚙️","desc":"Read/write config.toml at runtime","enabled":true,"builtin":true}),
        serde_json::json!({"name":"memory_search","icon":"🧠","desc":"Search past conversations via FTS5","enabled":true,"builtin":true}),
        serde_json::json!({"name":"doc_reader","icon":"📄","desc":"PDF, DOCX, Excel, CSV extraction","enabled":true,"builtin":true}),
        // Zalo & Messaging tools
        serde_json::json!({"name":"zalo_tool","icon":"💬","desc":"Zalo Personal/OA: gửi tin, đọc nhóm, kết bạn, tóm tắt","enabled":true,"builtin":true}),
        serde_json::json!({"name":"group_summarizer","icon":"📝","desc":"Tóm tắt nội dung nhóm chat & gửi báo cáo","enabled":true,"builtin":true}),
        // Database & API tools
        serde_json::json!({"name":"db_query","icon":"🗄️","desc":"Execute SQL queries (SQLite, PostgreSQL, MySQL)","enabled":true,"builtin":true}),
        serde_json::json!({"name":"db_schema","icon":"📐","desc":"Inspect database schema, tables, columns","enabled":true,"builtin":true}),
        serde_json::json!({"name":"api_connector","icon":"🔗","desc":"REST API connector with auth & headers","enabled":true,"builtin":true}),
        // Calendar & Browser
        serde_json::json!({"name":"calendar","icon":"📅","desc":"Google Calendar: tạo/xem/xóa sự kiện","enabled":true,"builtin":true}),
        serde_json::json!({"name":"browser","icon":"🌍","desc":"Browser automation: screenshot, navigate, click","enabled":true,"builtin":true}),
        // Text2SQL RAG — Natural Language to SQL
        serde_json::json!({"name":"nl_query","icon":"🧠","desc":"Hỏi database bằng tiếng Việt → AI tự viết SQL (RAG pipeline)","enabled":true,"builtin":true}),
    ];
    // Load custom tools
    let dir = tools_dir(&state);
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            if entry.path().extension().is_some_and(|e| e == "json")
                && let Ok(content) = std::fs::read_to_string(entry.path())
                && let Ok(tool) = serde_json::from_str::<serde_json::Value>(&content)
            {
                tools.push(tool);
            }
        }
    }
    // Load disabled state
    let disabled_path = state
        .config_path
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .join("tools-disabled.json");
    let disabled: Vec<String> = std::fs::read_to_string(&disabled_path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    for tool in tools.iter_mut() {
        if let Some(name) = tool["name"].as_str()
            && disabled.contains(&name.to_string())
        {
            tool["enabled"] = serde_json::Value::Bool(false);
        }
    }
    Json(serde_json::json!({"tools": tools}))
}

/// Create a custom tool
pub async fn tools_create(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let name = body["name"].as_str().unwrap_or("").trim();
    if name.is_empty() {
        return Json(serde_json::json!({"ok": false, "error": "Tool name required"}));
    }
    let id = name
        .to_lowercase()
        .replace(|c: char| !c.is_alphanumeric() && c != '-' && c != '_', "-");
    let tool = serde_json::json!({
        "name": id,
        "icon": body["icon"].as_str().unwrap_or("🔧"),
        "desc": body["desc"].as_str().unwrap_or(""),
        "enabled": true,
        "builtin": false,
        "command": body["command"].as_str().unwrap_or(""),
        "args": body["args"].as_str().unwrap_or(""),
        "created_at": chrono::Utc::now().to_rfc3339(),
    });
    let dir = tools_dir(&state);
    let path = dir.join(format!("{}.json", id));
    if let Err(e) = std::fs::write(
        &path,
        serde_json::to_string_pretty(&tool).unwrap_or_default(),
    ) {
        return internal_error("tools_create", e);
    }
    tracing::info!("🔧 Custom tool created: {}", id);
    Json(serde_json::json!({"ok": true, "tool": tool}))
}

/// Toggle tool enabled/disabled
pub async fn tools_toggle(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(name): axum::extract::Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let enabled = body["enabled"].as_bool().unwrap_or(true);
    let disabled_path = state
        .config_path
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .join("tools-disabled.json");
    let mut disabled: Vec<String> = std::fs::read_to_string(&disabled_path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    if enabled {
        disabled.retain(|n| n != &name);
    } else if !disabled.contains(&name) {
        disabled.push(name.clone());
    }
    let _ = std::fs::write(
        &disabled_path,
        serde_json::to_string(&disabled).unwrap_or_default(),
    );
    tracing::info!(
        "🔧 Tool {}: {}",
        name,
        if enabled { "enabled" } else { "disabled" }
    );
    Json(serde_json::json!({"ok": true, "enabled": enabled}))
}

/// Delete a custom tool
pub async fn tools_delete(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(name): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    let dir = tools_dir(&state);
    let path = dir.join(format!("{}.json", name));
    if path.exists() {
        if let Err(e) = std::fs::remove_file(&path) {
            return internal_error("tools_delete", e);
        }
        tracing::info!("🗑️ Custom tool deleted: {}", name);
        Json(serde_json::json!({"ok": true}))
    } else {
        Json(serde_json::json!({"ok": false, "error": "Cannot delete built-in tool"}))
    }
}
