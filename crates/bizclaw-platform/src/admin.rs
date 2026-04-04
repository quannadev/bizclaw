//! Admin HTTP server — REST API for the admin control plane.
//!
//! Phase 2: Uses tokio::sync::Mutex for db/manager to eliminate
//! poison-on-panic risk from std::sync::Mutex.

use crate::db::PlatformDb;
use crate::tenant::TenantManager;
use axum::extract::DefaultBodyLimit;
use axum::middleware;
use axum::{
    Extension, Json, Router,
    extract::{Path, State},
    routing::{delete, get, post, put},
};
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::cors::{Any, CorsLayer};

/// Shared application state for the admin server.
pub struct AdminState {
    pub db: Mutex<PlatformDb>,
    pub manager: Mutex<TenantManager>,
    pub jwt_secret: String,
    pub bizclaw_bin: String,
    pub base_port: u16,
    /// Domain name for this platform instance (e.g. "bizclaw.vn" or "viagent.vn")
    pub domain: String,
    /// Rate limiter: email → (attempt_count, first_attempt_time)
    pub login_attempts:
        std::sync::Mutex<std::collections::HashMap<String, (u32, std::time::Instant)>>,
    /// Rate limiter for registration: email → (attempt_count, first_attempt_time)
    pub register_attempts:
        std::sync::Mutex<std::collections::HashMap<String, (u32, std::time::Instant)>>,
    /// PostgreSQL DB for enterprise features (optional — only when DATABASE_URL is set).
    /// Falls back gracefully: enterprise endpoints return 503 if None.
    pub pg_db: Option<crate::db_pg::PgDb>,
    /// Startup time for uptime calculation.
    pub startup_time: std::time::Instant,
    /// AI Gateway for centralized inference (Shared AI Brain).
    pub ai_gateway: Option<crate::ai_gateway::SharedAiGateway>,
}

/// JWT auth middleware — validates Authorization: Bearer <token>.
async fn require_auth(
    State(state): State<Arc<AdminState>>,
    req: axum::http::Request<axum::body::Body>,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let auth_header = req
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if let Some(token) = auth_header.strip_prefix("Bearer ")
        && let Ok(claims) = crate::auth::validate_token(token, &state.jwt_secret)
    {
        let mut req = req;
        req.extensions_mut().insert(claims);
        return next.run(req).await;
    }

    axum::response::Response::builder()
        .status(axum::http::StatusCode::UNAUTHORIZED)
        .header("Content-Type", "application/json")
        .body(axum::body::Body::from(
            serde_json::json!({"ok": false, "error": "Unauthorized — invalid or missing JWT token"}).to_string()
        ))
        .expect("401 response")
}

/// Admin API server.
pub struct AdminServer;

impl AdminServer {
    /// Build the admin router.
    pub fn router(state: Arc<AdminState>) -> Router {
        // Protected routes — require valid JWT
        let protected = Router::new()
            // Dashboard data
            .route("/api/admin/stats", get(get_stats))
            .route("/api/admin/activity", get(get_activity))
            // Tenants
            .route("/api/admin/tenants", get(list_tenants))
            .route("/api/admin/tenants", post(create_tenant))
            .route("/api/admin/tenants/{id}", get(get_tenant))
            .route("/api/admin/tenants/{id}", delete(delete_tenant))
            .route("/api/admin/tenants/{id}/start", post(start_tenant))
            .route("/api/admin/tenants/{id}/stop", post(stop_tenant))
            .route("/api/admin/tenants/{id}/restart", post(restart_tenant))
            // pairing route removed — SaaS uses JWT login
            // Channel Configuration
            .route("/api/admin/tenants/{id}/channels", get(list_channels))
            .route("/api/admin/tenants/{id}/channels", post(upsert_channel))
            .route(
                "/api/admin/tenants/{id}/channels/{channel_id}",
                delete(delete_channel),
            )
            .route(
                "/api/admin/tenants/{id}/channels/zalo/qr",
                post(zalo_get_qr),
            )
            // Ollama / Brain Engine
            .route("/api/admin/ollama/models", get(ollama_list_models))
            .route("/api/admin/ollama/pull", post(ollama_pull_model))
            .route("/api/admin/ollama/delete", post(ollama_delete_model))
            .route("/api/admin/ollama/health", get(ollama_health))
            // AI Gateway — Shared AI Brain for multi-tenant inference
            .route("/v1/ai/chat/completions", post(crate::ai_gateway::ai_chat_completions))
            .route("/api/v1/ai/status", get(crate::ai_gateway::ai_gateway_status))
            .route("/api/v1/ai/usage", get(crate::ai_gateway::ai_usage_stats))
            .route("/api/v1/ai/usage/{tenant_id}", get(crate::ai_gateway::ai_tenant_usage))
            // Tenant Config (key-value settings)
            .route("/api/admin/tenants/{id}/configs", get(list_tenant_configs))
            .route("/api/admin/tenants/{id}/configs", post(set_tenant_configs))
            // Tenant Agents
            .route("/api/admin/tenants/{id}/agents", get(list_tenant_agents))
            .route("/api/admin/tenants/{id}/agents", post(upsert_tenant_agent))
            .route(
                "/api/admin/tenants/{id}/agents/{name}",
                delete(delete_tenant_agent),
            )
            // Users
            .route("/api/admin/users", get(list_users))
            .route("/api/admin/users", post(create_user_handler))
            .route("/api/admin/users/{id}", delete(delete_user_handler))
            .route("/api/admin/users/{id}/tenant", put(assign_tenant_handler))
            .route(
                "/api/admin/users/{id}/password/reset",
                put(admin_reset_user_password),
            )
            .route(
                "/api/admin/users/{id}/status",
                put(update_user_status_handler),
            )
            .route("/api/admin/users/{id}/role", put(update_user_role_handler))
            // Profile
            .route(
                "/api/admin/users/me/password",
                put(crate::self_serve::change_password_handler),
            )
            // ── ENTERPRISE: Multi-user RBAC per Tenant ─────────────────────
            .route("/api/admin/tenants/{id}/members", get(list_members))
            .route(
                "/api/admin/tenants/{id}/members/invite",
                post(invite_member),
            )
            .route(
                "/api/admin/tenants/{id}/members/{uid}/role",
                put(update_member_role),
            )
            .route(
                "/api/admin/tenants/{id}/members/{uid}",
                delete(remove_member),
            )
            // ── ENTERPRISE: Human Handoff ───────────────────────────────────
            .route("/api/admin/tenants/{id}/handoffs", get(list_handoffs))
            .route(
                "/api/admin/tenants/{id}/handoffs/{hid}/claim",
                post(claim_handoff),
            )
            .route(
                "/api/admin/tenants/{id}/handoffs/{hid}/reply",
                post(reply_handoff),
            )
            .route(
                "/api/admin/tenants/{id}/handoffs/{hid}/resolve",
                post(resolve_handoff),
            )
            .route(
                "/api/admin/tenants/{id}/handoffs/{hid}/messages",
                get(list_handoff_messages),
            )
            // ── ENTERPRISE: BI Analytics ────────────────────────────────────
            .route(
                "/api/admin/tenants/{id}/analytics/summary",
                get(analytics_summary),
            )
            .route(
                "/api/admin/tenants/{id}/analytics/tokens",
                get(analytics_tokens),
            )
            // ── ENTERPRISE: Budget Quota Control ───────────────────────────
            .route("/api/admin/tenants/{id}/quotas", get(list_quotas))
            .route("/api/admin/tenants/{id}/quotas/{resource}", put(set_quota))
            // ── MISSION CONTROL: Kanban Task Board ─────────────────────────
            .route("/api/admin/tasks", get(mc_list_tasks).post(mc_create_task))
            .route("/api/admin/tasks/board", get(mc_kanban_board))
            .route("/api/admin/tasks/pending-review", get(mc_pending_reviews))
            .route(
                "/api/admin/tasks/{tid}",
                get(mc_get_task).put(mc_update_task).delete(mc_delete_task),
            )
            .route(
                "/api/admin/tasks/{tid}/comments",
                get(mc_list_comments).post(mc_add_comment),
            )
            .route("/api/admin/tasks/{tid}/review", post(mc_submit_review))
            // ── MISSION CONTROL: Agent Session Monitor ─────────────────────
            .route("/api/admin/sessions", get(mc_list_sessions))
            .route("/api/admin/sessions/heartbeat", post(mc_heartbeat))
            .route(
                "/api/admin/sessions/{key}/terminate",
                post(mc_terminate_session),
            )
            // ── MISSION CONTROL: GitHub Issues Sync ────────────────────────
            .route(
                "/api/admin/tenants/{id}/github-syncs",
                get(mc_list_github_syncs).post(mc_upsert_github_sync),
            )
            .route(
                "/api/admin/tenants/{id}/github-syncs/{repo}/trigger",
                post(mc_trigger_github_sync),
            )
            // ── CLOUD: Remote Server Provisioner ────────────────────────────
            .route(
                "/api/admin/servers",
                get(srv_list_servers).post(srv_provision),
            )
            .route(
                "/api/admin/servers/{sid}",
                get(srv_get_server).delete(srv_delete_server),
            )
            .route("/api/admin/servers/{sid}/health", get(srv_health_check))
            // ── CLOUD SaaS INTEGRATION ──────────────────────────────────────
            .route("/api/v1/cloud/tenants", get(crate::api_cloud::cloud_list_tenants).post(crate::api_cloud::cloud_create_tenant))
            .route("/api/v1/cloud/tenants/{id}", delete(crate::api_cloud::cloud_delete_tenant))
            .route("/api/v1/cloud/tenants/{id}/suspend", post(crate::api_cloud::cloud_suspend_tenant))
            .route("/api/v1/cloud/tenants/{id}/resume", post(crate::api_cloud::cloud_resume_tenant))
            .route("/api/v1/cloud/tenants/{id}/renew", post(crate::api_cloud::cloud_renew_tenant))
            .route("/api/v1/cloud/stats", get(crate::api_cloud::cloud_stats))
            .route("/api/v1/cloud/config", get(crate::api_cloud::cloud_get_config).post(crate::api_cloud::cloud_save_config))
            .route("/api/v1/cloud/test", post(crate::api_cloud::cloud_test_connection))
            .route(
                "/api/admin/servers/{sid}/command",
                post(srv_execute_command),
            )
            // ── MISSION CONTROL: Webhooks ───────────────────────────────────
            .route("/api/admin/tenants/{id}/webhooks", get(mc_list_webhooks))
            // ── SECRET MANAGEMENT: Key Status (masked) ──────────────────────
            .route("/api/admin/keys/status", get(key_status_handler))
            // ── BILLING: SePay & Payment ──────────────────────────────
            .route("/api/billing/status", get(billing_status_handler))
            .route("/api/billing/config", put(set_billing_config_handler))
            .route("/api/billing/vietqr", get(vietqr_handler))
            // ── SME TEMPLATES ─────────────────────────────────────────
            .route("/api/admin/templates", get(list_templates_handler))
            // ── OAUTH: Social Connect (protected endpoints) ──────────────
            .route(
                "/api/oauth/status/{tenant_id}",
                get(crate::oauth::oauth_status),
            )
            .route(
                "/api/oauth/disconnect/{provider}",
                delete(crate::oauth::oauth_disconnect),
            )
            // ── MAMA AI: Orchestrator + Cost-Aware Provider Routing ───
            .route("/api/mama/providers", get(crate::mama::list_providers))
            .route("/api/mama/plan", post(crate::mama::create_plan))
            .route("/api/mama/skills", get(crate::mama::search_skills_api))
            .route("/api/mama/status", get(crate::mama::mama_status))
            .route("/api/mama/onboard", post(crate::mama::smart_onboard))
            .route("/api/mama/detect-key", post(crate::mama::detect_key))
            // ── MAMA AI: Workflows + Teams (Phase 1 Integration) ─────
            .route("/api/mama/workflows", get(crate::mama::list_workflows))
            .route("/api/mama/workflows/run", post(crate::mama::run_workflow))
            .route("/api/mama/teams", get(crate::mama::list_teams))
            .route(
                "/api/mama/team/{channel}",
                get(crate::mama::team_for_channel),
            )
            // ── MAMA AI: Execute + Budget + Health + Dashboard (Phase 2+3) ──
            .route("/api/mama/execute", post(crate::mama::execute_plan))
            .route("/api/mama/budget", get(crate::mama::budget_summary))
            .route("/api/mama/health", get(crate::mama::health_status))
            .route("/api/mama/dashboard", get(crate::mama::mama_dashboard))
            .route_layer(middleware::from_fn_with_state(state.clone(), require_auth));

        // Public routes — including invitation acceptance
        let public = Router::new()
            .route("/api/admin/login", post(login))
            .route(
                "/api/admin/pairing/validate",
                post(validate_pairing_deprecated),
            )
            .route(
                "/api/admin/register",
                post(crate::self_serve::register_handler),
            )
            .route(
                "/api/admin/password-reset",
                post(crate::self_serve::forgot_password_handler),
            )
            .route(
                "/api/admin/password-reset/confirm",
                post(crate::self_serve::reset_password_handler),
            )
            .route(
                "/api/admin/invitations/{token}/accept",
                post(accept_invitation),
            )
            .route("/pixel-office", get(pixel_office_page))
            .route("/hub", get(hub_skills_page))
            .route("/health", get(health_check_handler))
            // SePay webhook — public (SePay POSTs here, no JWT)
            .route("/api/billing/sepay-webhook", post(sepay_webhook_handler))
            // Pay2S webhook — public (Pay2S POSTs here, bearer auth inside)
            .route("/api/billing/pay2s-webhook", post(pay2s_webhook_handler))
            // OAuth2 callbacks — public (Google/FB redirect here)
            .route(
                "/api/oauth/connect/{provider}",
                get(crate::oauth::oauth_connect),
            )
            .route(
                "/api/oauth/callback/{provider}",
                get(crate::oauth::oauth_callback),
            )
            .route("/", get(admin_dashboard_page));

        // SPA fallback — serve dashboard HTML for all non-API paths
        // so that /tenants, /settings, /ollama etc. all work
        let spa_fallback = Router::new().fallback(get(admin_dashboard_page));

        // CORS — configurable via BIZCLAW_CORS_ORIGINS env var
        // H4 FIX: Default to localhost-only CORS (not Any) for security
        let cors_methods = [
            axum::http::Method::GET,
            axum::http::Method::POST,
            axum::http::Method::PUT,
            axum::http::Method::DELETE,
            axum::http::Method::OPTIONS,
        ];
        let cors = match std::env::var("BIZCLAW_CORS_ORIGINS") {
            Ok(origins) if !origins.is_empty() => {
                let allowed: Vec<_> = origins
                    .split(',')
                    .filter_map(|s| s.trim().parse().ok())
                    .collect();
                CorsLayer::new()
                    .allow_origin(allowed)
                    .allow_methods(cors_methods)
                    .allow_headers(Any)
            }
            _ => {
                // Dev mode: allow Any; Production: restrict to same-origin
                if std::env::var("BIZCLAW_BIND_ALL").unwrap_or_default() == "1" {
                    CorsLayer::new()
                        .allow_origin(Any)
                        .allow_methods(cors_methods)
                        .allow_headers(Any)
                } else {
                    // Production: only allow requests from same origin (no CORS header = same-origin only)
                    CorsLayer::new()
                        .allow_methods(cors_methods)
                        .allow_headers(Any)
                }
            }
        };

        protected
            .merge(public)
            .merge(spa_fallback)
            .layer(axum::middleware::from_fn(platform_security_headers))
            .layer(cors)
            .layer(DefaultBodyLimit::max(1_048_576)) // 1MB max request body
            .with_state(state)
    }

    /// Start the admin server.
    pub async fn start(state: Arc<AdminState>, port: u16) -> bizclaw_core::error::Result<()> {
        let app = Self::router(state.clone());
        // Bind to 127.0.0.1 — only accessible via reverse proxy (Nginx)
        // Set BIZCLAW_BIND_ALL=1 to allow direct external access (dev only)
        let bind_addr = if std::env::var("BIZCLAW_BIND_ALL").unwrap_or_default() == "1" {
            [0, 0, 0, 0]
        } else {
            [127, 0, 0, 1]
        };
        let addr = std::net::SocketAddr::from((bind_addr, port));
        tracing::info!("🏢 Admin platform running at http://localhost:{port}");

        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .map_err(|e| bizclaw_core::error::BizClawError::Gateway(format!("Bind error: {e}")))?;

        let watchdog_state = state.clone();
        tokio::spawn(async move {
            tracing::info!("🐕 Watchdog auto-healing started.");
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
                let db = watchdog_state.db.lock().await;
                if let Ok(tenants) = db.list_tenants() {
                    let mut mgr = watchdog_state.manager.lock().await;
                    for t in tenants {
                        if t.status == "running" && !mgr.is_running(&t.id) {
                            tracing::warn!(
                                "🐕 Watchdog: Tenant {} marked 'running' but process missing. Restarting...",
                                t.slug
                            );
                            // Avoid dropping lock while restarting
                            match mgr.start_tenant(&t, &watchdog_state.bizclaw_bin, &db) {
                                Ok(pid) => {
                                    db.update_tenant_status(&t.id, "running", Some(pid)).ok();
                                    tracing::info!(
                                        "🐕 Watchdog: Restarted tenant {} (pid={})",
                                        t.slug,
                                        pid
                                    );
                                }
                                Err(e) => {
                                    db.update_tenant_status(&t.id, "error", None).ok();
                                    tracing::error!(
                                        "🐕 Watchdog: Failed to restart tenant {}: {}",
                                        t.slug,
                                        e
                                    );
                                }
                            }
                        }
                    }
                }
            }
        });

        // ── Graceful Shutdown ──────────────────────────────────
        let shutdown_state = state.clone();
        let shutdown_signal = async move {
            let ctrl_c = async {
                tokio::signal::ctrl_c().await.ok();
            };
            #[cfg(unix)]
            let sigterm = async {
                tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                    .expect("SIGTERM handler")
                    .recv()
                    .await;
            };
            #[cfg(not(unix))]
            let sigterm = std::future::pending::<()>();

            tokio::select! {
                _ = ctrl_c => {},
                _ = sigterm => {},
            }

            tracing::info!("🛑 Shutdown signal received — stopping all tenants...");
            // Stop all running tenants gracefully
            let db = shutdown_state.db.lock().await;
            if let Ok(tenants) = db.list_tenants() {
                let mut mgr = shutdown_state.manager.lock().await;
                for t in &tenants {
                    if t.status == "running" {
                        let _ = mgr.stop_tenant(&t.id);
                        db.update_tenant_status(&t.id, "stopped", None).ok();
                        tracing::info!("  ↳ Stopped tenant: {}", t.slug);
                    }
                }
            }
            db.log_event("platform_shutdown", "system", "platform", Some("graceful"))
                .ok();
            tracing::info!("✅ All tenants stopped. Platform shutting down.");
        };

        axum::serve(listener, app)
            .with_graceful_shutdown(shutdown_signal)
            .await
            .map_err(|e| {
                bizclaw_core::error::BizClawError::Gateway(format!("Server error: {e}"))
            })?;

        Ok(())
    }
}

// ── Security Headers (C1 FIX) ──────────────────────────

/// Security headers middleware — HSTS, CSP, X-Frame-Options, X-Content-Type-Options.
/// Matches Gateway security headers for parity.
async fn platform_security_headers(
    req: axum::http::Request<axum::body::Body>,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let mut response = next.run(req).await;
    let headers = response.headers_mut();
    headers.insert(
        "X-Content-Type-Options",
        "nosniff".parse().expect("static header"),
    );
    headers.insert(
        "X-Frame-Options",
        "SAMEORIGIN".parse().expect("static header"),
    );
    headers.insert(
        "X-XSS-Protection",
        "1; mode=block".parse().expect("static header"),
    );
    headers.insert(
        "Referrer-Policy",
        "strict-origin-when-cross-origin"
            .parse()
            .expect("static header"),
    );
    // HSTS — only when behind reverse proxy (production)
    if std::env::var("BIZCLAW_BIND_ALL").unwrap_or_default() != "1" {
        headers.insert(
            "Strict-Transport-Security",
            "max-age=31536000; includeSubDomains"
                .parse()
                .expect("static header"),
        );
    }
    // Permissions-Policy — restrict browser features
    headers.insert(
        "Permissions-Policy",
        "camera=(), microphone=(), geolocation=(), payment=(), usb=()"
            .parse()
            .expect("static header"),
    );
    response
}

// ── Health Check ────────────────────────────────────────

/// Public health endpoint for Docker HEALTHCHECK, load balancers, and monitoring.
/// Returns database connectivity, uptime, tenant counts, and version.
async fn health_check_handler(State(state): State<Arc<AdminState>>) -> Json<serde_json::Value> {
    let uptime_secs = state.startup_time.elapsed().as_secs();

    // Check SQLite connectivity
    let db_status = match state.db.lock().await.tenant_stats() {
        Ok((total, running, stopped, error)) => {
            serde_json::json!({
                "connected": true,
                "tenants_total": total,
                "tenants_running": running,
                "tenants_stopped": stopped,
                "tenants_error": error,
            })
        }
        Err(_) => serde_json::json!({ "connected": false }),
    };

    // Check PostgreSQL (enterprise) — optional
    let pg_status = if let Some(pg) = &state.pg_db {
        match pg.pool().acquire().await {
            Ok(_) => serde_json::json!({ "connected": true }),
            Err(_) => serde_json::json!({ "connected": false }),
        }
    } else {
        serde_json::json!({ "connected": false, "reason": "not configured" })
    };

    Json(serde_json::json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
        "uptime_secs": uptime_secs,
        "database": db_status,
        "postgres": pg_status,
        "domain": state.domain,
    }))
}

// ── Key Status (Secret Management) ─────────────────────

/// Protected endpoint: returns masked key status for all integrations.
/// Agent Mama uses this to know which keys are configured WITHOUT seeing raw values.
///
/// GET /api/admin/keys/status
/// Response: { "llm": { "configured": true, "source": "env", "provider": "openai", "masked": "sk-***83d" }, ... }
async fn key_status_handler(State(_state): State<Arc<AdminState>>) -> Json<serde_json::Value> {
    use bizclaw_core::config::{BizClawConfig, mask_key};

    // Load current config (hot-reload aware)
    let config = BizClawConfig::load().unwrap_or_default();

    // LLM key status
    let (llm_ok, llm_src, llm_masked) = config.llm.key_status();

    // Telegram (first instance)
    let tg_status = config.channel.telegram.first().map(|tg| {
        let (ok, src, masked) = tg.key_status();
        serde_json::json!({ "configured": ok, "source": src, "masked": masked, "name": tg.name })
    }).unwrap_or(serde_json::json!({ "configured": false }));

    // Discord (first instance)
    let dc_status = config.channel.discord.first().map(|dc| {
        let (ok, src, masked) = dc.key_status();
        serde_json::json!({ "configured": ok, "source": src, "masked": masked, "name": dc.name })
    }).unwrap_or(serde_json::json!({ "configured": false }));

    // WhatsApp (first instance)
    let wa_status = config
        .channel
        .whatsapp
        .first()
        .map(|wa| {
            let (ok, src, masked) = wa.key_status();
            serde_json::json!({ "configured": ok, "source": src, "masked": masked })
        })
        .unwrap_or(serde_json::json!({ "configured": false }));

    // Zalo OA — check env var first
    let zalo_status = {
        let zalo = config.channel.zalo.first();
        let env_token = std::env::var("BIZCLAW_ZALO_OA_ACCESS_TOKEN")
            .ok()
            .filter(|s| !s.is_empty());
        if let Some(token) = env_token {
            serde_json::json!({ "configured": true, "source": "env", "masked": mask_key(&token) })
        } else if let Some(z) = zalo {
            if !z.oa_access_token.is_empty() {
                serde_json::json!({ "configured": true, "source": "config", "masked": mask_key(&z.oa_access_token) })
            } else {
                serde_json::json!({ "configured": false })
            }
        } else {
            serde_json::json!({ "configured": false })
        }
    };

    // Email
    let email_status = config.channel.email.first().map(|em| {
        if !em.password.is_empty() {
            serde_json::json!({ "configured": true, "source": "config", "masked": mask_key(&em.password), "email": em.email })
        } else {
            serde_json::json!({ "configured": false })
        }
    }).unwrap_or(serde_json::json!({ "configured": false }));

    Json(serde_json::json!({
        "llm": {
            "configured": llm_ok,
            "source": llm_src,
            "provider": config.llm.provider,
            "model": config.llm.model,
            "masked": llm_masked,
        },
        "telegram": tg_status,
        "discord": dc_status,
        "whatsapp": wa_status,
        "zalo": zalo_status,
        "email": email_status,
        "_note": "Keys are never exposed via this API. Use environment variables for SaaS deployments.",
    }))
}

// ── SePay & Pay2S Webhooks & Billing ────────────────────────────

/// Pay2S Webhook Integration ──────────────────────────────────────

#[derive(Debug, serde::Deserialize)]
#[allow(non_snake_case, dead_code)]
struct Pay2STransaction {
    id: String,
    gateway: String,
    transactionDate: String,
    transactionNumber: String,
    accountNumber: String,
    content: String,
    transferType: String,
    transferAmount: u64,
}

#[derive(Debug, serde::Deserialize)]
struct Pay2SWebhookPayload {
    transactions: Vec<Pay2STransaction>,
}

/// POST /api/billing/pay2s-webhook — receive SaaS payments from Pay2S.
async fn pay2s_webhook_handler(
    State(state): State<Arc<AdminState>>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<Pay2SWebhookPayload>,
) -> (axum::http::StatusCode, Json<serde_json::Value>) {
    let db = state.db.lock().await;

    // 1. Verify Authorization (Bearer token)
    let auth_header = headers.get("Authorization").and_then(|h| h.to_str().ok());
    let token = db
        .get_platform_config("pay2s_api_key")
        .unwrap_or_else(|| std::env::var("BIZCLAW_PAY2S_API_KEY").unwrap_or_default());

    if token.is_empty() || auth_header != Some(&format!("Bearer {}", token)) {
        tracing::warn!(
            "🚫 Pay2S webhook auth failed or API KEY not configured in Admin Settings / Environment"
        );
        return (
            axum::http::StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({ "success": false, "message": "Invalid token" })),
        );
    }

    // 2. Process transactions
    for tx in payload.transactions {
        if tx.transferType.to_uppercase() != "IN" {
            continue; // Only incoming transfers
        }

        // [SECURITY: REPLAY ATTACK PREVENTION]
        // Parse date (assumed "YYYY-MM-DD HH:MM:SS"). Reject if older than 48 hours.
        if let Ok(tx_date) =
            chrono::NaiveDateTime::parse_from_str(&tx.transactionDate, "%Y-%m-%d %H:%M:%S")
        {
            let tx_utc = tx_date.and_utc();
            let now = chrono::Utc::now();
            if now.signed_duration_since(tx_utc).num_hours() > 48 {
                tracing::error!(
                    "🚨 SECURITY ALERT: Rejected ancient Pay2S replay attack date {} (Tx ID: {})",
                    tx.transactionDate,
                    tx.id
                );
                continue; // Ignore replay
            }
        }

        let tx_ref = format!("pay2s_{}", tx.id);

        // Record locally for idempotency (so we do not process twice)
        match db.record_payment("pay2s", &tx_ref, tx.transferAmount, &tx.content) {
            Ok(true) => {
                tracing::info!(
                    "💳 Pay2S webhook received: id={}, amount={}, content='{}'",
                    tx.id,
                    tx.transferAmount,
                    tx.content
                );

                // Auto plan activation match
                let content_upper = tx.content.to_uppercase();
                let match_str = if content_upper.contains("BIZCLAW") {
                    &content_upper
                } else if content_upper.contains("BC") {
                    // Fallback shorter prefix
                    &content_upper
                } else {
                    ""
                };

                if !match_str.is_empty() {
                    let tenant_ref = match_str
                        .replace("BIZCLAW-", "")
                        .replace("BIZCLAW ", "")
                        .replace("BC-", "")
                        .replace("BC ", "")
                        .trim()
                        .to_string();

                    if !tenant_ref.is_empty() && tenant_ref != "TEST" {
                        let amount = tx.transferAmount;
                        let new_plan = if amount >= 7_999_000 {
                            "business" // ~8tr/tháng
                        } else if amount >= 4_999_000 {
                            "pro" // ~5tr/tháng
                        } else {
                            ""
                        };

                        if !new_plan.is_empty() {
                            if let Ok(Some(tenant)) = db.find_tenant_by_ref(&tenant_ref) {
                                if let Ok(()) = db.update_tenant_plan(&tenant.id, new_plan) {
                                    tracing::info!(
                                        "✅ Auto plan activated via Pay2S: tenant={} ({}), plan={}, amount={}đ",
                                        tenant.slug,
                                        tenant.id,
                                        new_plan,
                                        amount
                                    );
                                    let _ = db.complete_payment(&tx_ref, &tenant.id);

                                    // Activate tenant
                                    if tenant.status == "pending" {
                                        let _ = db.update_tenant_status(&tenant.id, "stopped", None);
                                    }

                                    // Auto-activate the owner user if they are pending
                                    if let Some(owner_id) = tenant.owner_id {
                                        if let Ok(Some(user)) = db.get_user_by_id(&owner_id) {
                                            if user.status == "pending" {
                                                tracing::info!("🔓 Auto-activating pending user {} after payment", user.email);
                                                let _ = db.update_user_status(&user.id, "active");
                                                let _ = db.update_user_role(&user.id, "admin");
                                            }
                                        }
                                    }

                                    let _ = db.log_event(
                                        "plan_activated",
                                        "pay2s",
                                        &tenant.id,
                                        Some(&format!(
                                            "Auto-activated plan '{}' via Pay2S payment {}đ (ref: {})",
                                            new_plan, amount, tenant_ref
                                        )),
                                    );
                                }
                            } else {
                                tracing::warn!(
                                    "⚠️ Pay2S payment ref '{}' did not match any tenant",
                                    tenant_ref
                                );
                                let _ = db.fail_payment(&tx_ref, "unmatched");
                            }
                        } else {
                            tracing::info!(
                                "💰 Pay2S payment {}đ received (ref: {}) — below plan threshold",
                                amount,
                                tenant_ref
                            );
                            let _ = db.fail_payment(&tx_ref, "amount_too_small");
                        }
                    }
                }
            }
            Ok(false) => {
                tracing::debug!("Pay2S: duplicate webhook id={}, returning success", tx_ref);
            }
            Err(e) => {
                tracing::error!("Pay2S: Failed to record payment DB logic: {}", e);
            }
        }
    }

    // Always 200 OK so Pay2S stops retrying
    (
        axum::http::StatusCode::OK,
        Json(serde_json::json!({ "success": true })),
    )
}

/// SePay Webhook Integration ──────────────────────────────────────

/// SePay webhook payload — matches exact SePay documentation.
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
struct SepayWebhookPayload {
    /// Unique transaction ID (use for deduplication)
    id: i64,
    /// Bank name (e.g., "Vietcombank")
    #[serde(default)]
    gateway: String,
    /// Transaction timestamp
    #[serde(default)]
    transaction_date: String,
    /// Bank account number
    #[serde(default)]
    account_number: String,
    /// Payment code
    #[serde(default)]
    code: Option<String>,
    /// Transfer description/content
    #[serde(default)]
    content: String,
    /// "in" (incoming) or "out" (outgoing)
    transfer_type: String,
    /// Transaction amount in VND
    transfer_amount: f64,
    /// Account balance after transaction
    #[serde(default)]
    accumulated: f64,
    /// Sub-account identifier
    #[serde(default)]
    sub_account: Option<String>,
    /// Bank transaction reference
    #[serde(default)]
    reference_code: Option<String>,
}

/// POST /api/billing/sepay-webhook — receive SaaS subscription payments from SePay.
///
/// Public endpoint (no JWT). SePay calls this when a customer pays for a
/// BizClaw SaaS plan via bank transfer. Matches amount to plan tier and
/// auto-activates the tenant's plan.
/// Security: validated via `Authorization: Apikey <KEY>` header.
///
/// Following SePay webhook spec: https://docs.sepay.vn/
async fn sepay_webhook_handler(
    State(state): State<Arc<AdminState>>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<SepayWebhookPayload>,
) -> (axum::http::StatusCode, Json<serde_json::Value>) {
    // 1. Log incoming webhook FIRST (for debugging)
    tracing::info!(
        "💳 SePay webhook received: id={}, type={}, amount={}, ref={:?}",
        payload.id,
        payload.transfer_type,
        payload.transfer_amount,
        payload.reference_code,
    );

    let db = state.db.lock().await;

    // 2. Verify authorization (API key from Admin Settings or Env var)
    let expected_key = db
        .get_platform_config("sepay_api_key")
        .unwrap_or_else(|| std::env::var("BIZCLAW_SEPAY_API_KEY").unwrap_or_default());

    if expected_key.is_empty() {
        tracing::error!(
            "🚨 SECURITY ALERT: BIZCLAW_SEPAY_API_KEY is completely missing in Admin Configs! Dropping all SePay webhooks!"
        );
        return (
            axum::http::StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({ "error": "System not configured" })),
        );
    }

    let auth_header = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let valid = auth_header == format!("Apikey {}", expected_key)
        || auth_header == format!("Bearer {}", expected_key);

    if !valid {
        tracing::warn!("🚫 SePay webhook auth failed: {:?}", auth_header);
        return (
            axum::http::StatusCode::FORBIDDEN,
            Json(serde_json::json!({ "error": "Unauthorized" })),
        );
    }

    // 3. Only process incoming transfers
    if payload.transfer_type != "in" {
        tracing::debug!("SePay: ignoring outgoing transfer id={}", payload.id);
        return (
            axum::http::StatusCode::OK,
            Json(serde_json::json!({ "success": true, "message": "ignored_outgoing" })),
        );
    }

    // [SECURITY: REPLAY ATTACK PREVENTION]
    // Parse SePay ISO date/datetime. Reject if older than 48 hours.
    if let Ok(tx_date) =
        chrono::NaiveDateTime::parse_from_str(&payload.transaction_date, "%Y-%m-%d %H:%M:%S")
    {
        let tx_utc = tx_date.and_utc();
        let now = chrono::Utc::now();
        if now.signed_duration_since(tx_utc).num_hours() > 48 {
            tracing::error!(
                "🚨 SECURITY ALERT: Rejected ancient SePay replay attack date {} (Tx ID: {})",
                payload.transaction_date,
                payload.id
            );
            return (
                axum::http::StatusCode::OK,
                Json(serde_json::json!({ "success": true, "message": "Ignored obsolete" })),
            );
        }
    }

    // 4. Idempotency & DB Recording (Local SQLite as Source of Truth)
    let tx_ref = format!("sepay_{}", payload.id);

    // Check if already processed
    match db.record_payment(
        "sepay",
        &tx_ref,
        payload.transfer_amount as u64,
        &payload.content,
    ) {
        Ok(true) => {
            // New payment -> Write to PG if it exists (for dashboard)
            if let Some(pg) = &state.pg_db {
                let _ = sqlx::query(
                    "INSERT INTO sepay_transactions (sepay_id, gateway, transfer_type, transfer_amount, content, reference_code, account_number, transaction_date, created_at) \
                     VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NOW()) \
                     ON CONFLICT (sepay_id) DO NOTHING"
                )
                .bind(payload.id)
                .bind(&payload.gateway)
                .bind(&payload.transfer_type)
                .bind(payload.transfer_amount)
                .bind(&payload.content)
                .bind(&payload.reference_code)
                .bind(&payload.account_number)
                .bind(&payload.transaction_date)
                .execute(pg.pool())
                .await;
            }

            // 5. Auto plan activation
            let content_upper = payload.content.to_uppercase();
            let ref_upper = payload
                .reference_code
                .as_deref()
                .unwrap_or("")
                .to_uppercase();
            let match_str = if content_upper.contains("BIZCLAW") || content_upper.contains("BC") {
                &content_upper
            } else if ref_upper.contains("BIZCLAW") || ref_upper.contains("BC") {
                &ref_upper
            } else {
                ""
            };

            if !match_str.is_empty() {
                let tenant_ref = match_str
                    .replace("BIZCLAW-", "")
                    .replace("BIZCLAW ", "")
                    .replace("BC-", "")
                    .replace("BC ", "")
                    .trim()
                    .to_string();

                if !tenant_ref.is_empty() && tenant_ref != "TEST" {
                    let amount = payload.transfer_amount as u64;
                    let new_plan = if amount >= 7_999_000 {
                        "business" // ~8tr/tháng
                    } else if amount >= 4_999_000 {
                        "pro" // ~5tr/tháng
                    } else {
                        "" // Amount too low
                    };

                    if !new_plan.is_empty() {
                        if let Ok(Some(tenant)) = db.find_tenant_by_ref(&tenant_ref) {
                            if let Ok(()) = db.update_tenant_plan(&tenant.id, new_plan) {
                                tracing::info!(
                                    "✅ Auto plan activated via SePay: tenant={} ({}), plan={}, amount={}đ",
                                    tenant.slug,
                                    tenant.id,
                                    new_plan,
                                    amount
                                );
                                let _ = db.complete_payment(&tx_ref, &tenant.id);

                                // Activate tenant from 'stopped' to 'running' if applicable,
                                // or just 'stopped' so Watchdog can start it. We'll set 'stopped' 
                                // if it's currently something else, to let Mama take over.
                                if tenant.status == "pending" {
                                    let _ = db.update_tenant_status(&tenant.id, "stopped", None);
                                }

                                // Auto-activate the owner user if they are pending
                                if let Some(owner_id) = tenant.owner_id {
                                    if let Ok(Some(user)) = db.get_user_by_id(&owner_id) {
                                        if user.status == "pending" {
                                            tracing::info!("🔓 Auto-activating pending user {} after payment", user.email);
                                            let _ = db.update_user_status(&user.id, "active");
                                            let _ = db.update_user_role(&user.id, "admin");
                                        }
                                    }
                                }

                                let _ = db.log_event(
                                    "plan_activated",
                                    "sepay",
                                    &tenant.id,
                                    Some(&format!(
                                        "Auto-activated plan '{}' via SePay payment {}đ (ref: {})",
                                        new_plan, amount, tenant_ref
                                    )),
                                );
                            }
                        } else {
                            tracing::warn!(
                                "⚠️ SePay payment ref '{}' did not match any tenant",
                                tenant_ref
                            );
                            let _ = db.fail_payment(&tx_ref, "unmatched");
                        }
                    } else {
                        tracing::info!(
                            "💰 SePay payment {}đ received (ref: {}) — below plan threshold",
                            amount,
                            tenant_ref
                        );
                        let _ = db.fail_payment(&tx_ref, "amount_too_small");
                    }
                }
            }
        }
        Ok(false) => {
            tracing::info!(
                "SePay: duplicate webhook id={}, returning success",
                payload.id
            );
            return (
                axum::http::StatusCode::OK,
                Json(serde_json::json!({ "success": true, "message": "already_processed" })),
            );
        }
        Err(e) => {
            tracing::error!("SePay: Failed to record payment logic: {}", e);
        }
    }

    // Billing ghi thẳng vào PostgreSQL — Google Sheets dùng cho AI Agent làm việc

    tracing::info!(
        "✅ SePay webhook processed: id={}, amount={} VND, gateway={}",
        payload.id,
        payload.transfer_amount,
        payload.gateway,
    );

    (
        axum::http::StatusCode::OK,
        Json(serde_json::json!({ "success": true })),
    )
}

#[derive(serde::Deserialize)]
struct BillingConfigReq {
    sepay_api_key: Option<String>,
    pay2s_api_key: Option<String>,
    sepay_bank_name: Option<String>,
    sepay_account_no: Option<String>,
    sepay_account_name: Option<String>,
}

/// PUT /api/billing/config — update billing webhook configurations
async fn set_billing_config_handler(
    State(state): State<Arc<AdminState>>,
    Json(req): Json<BillingConfigReq>,
) -> Json<serde_json::Value> {
    let db = state.db.lock().await;

    if let Some(sepay) = req.sepay_api_key {
        if let Err(e) = db.set_platform_config("sepay_api_key", &sepay) {
            return Json(
                serde_json::json!({"ok": false, "error": format!("Failed to save SePay config: {}", e)}),
            );
        }
    }

    if let Some(pay2s) = req.pay2s_api_key {
        if let Err(e) = db.set_platform_config("pay2s_api_key", &pay2s) {
            return Json(
                serde_json::json!({"ok": false, "error": format!("Failed to save Pay2S config: {}", e)}),
            );
        }
    }

    if let Some(val) = req.sepay_bank_name {
        let _ = db.set_platform_config("SEPAY_BANK_ID", &val);
    }
    if let Some(val) = req.sepay_account_no {
        let _ = db.set_platform_config("SEPAY_ACCOUNT_NO", &val);
    }
    if let Some(val) = req.sepay_account_name {
        let _ = db.set_platform_config("SEPAY_ACCOUNT_NAME", &val);
    }

    Json(serde_json::json!({"ok": true}))
}

/// GET /api/billing/status — billing overview for dashboard.
async fn billing_status_handler(State(state): State<Arc<AdminState>>) -> Json<serde_json::Value> {
    let (sepay_cfg, pay2s_cfg, bank, acc_no, acc_name) = {
        let db = state.db.lock().await;
        let sepay = db
            .get_platform_config("sepay_api_key")
            .unwrap_or_else(|| std::env::var("BIZCLAW_SEPAY_API_KEY").unwrap_or_default());
        let pay2s = db
            .get_platform_config("pay2s_api_key")
            .unwrap_or_else(|| std::env::var("BIZCLAW_PAY2S_API_KEY").unwrap_or_default());
        let b = db.get_platform_config("SEPAY_BANK_ID").unwrap_or_default();
        let a = db.get_platform_config("SEPAY_ACCOUNT_NO").unwrap_or_default();
        let n = db.get_platform_config("SEPAY_ACCOUNT_NAME").unwrap_or_default();
        (sepay, pay2s, b, a, n)
    };

    let mut recent_transactions = serde_json::json!([]);
    let mut total_received: f64 = 0.0;
    let mut transaction_count: i64 = 0;

    if let Some(pg) = &state.pg_db {
        // Get total received amount
        if let Ok(row) = sqlx::query_as::<_, (i64, f64)>(
            "SELECT COUNT(*), COALESCE(SUM(transfer_amount), 0) FROM sepay_transactions WHERE transfer_type = 'in'"
        )
        .fetch_one(pg.pool())
        .await
        {
            transaction_count = row.0;
            total_received = row.1;
        }

        // Get last 10 transactions
        if let Ok(rows) = sqlx::query_as::<_, (i64, String, f64, String, Option<String>, String)>(
            "SELECT sepay_id, gateway, transfer_amount, content, reference_code, transaction_date \
             FROM sepay_transactions \
             WHERE transfer_type = 'in' \
             ORDER BY sepay_id DESC LIMIT 10",
        )
        .fetch_all(pg.pool())
        .await
        {
            recent_transactions = serde_json::json!(
                rows.iter()
                    .map(|r| serde_json::json!({
                        "id": r.0,
                        "gateway": r.1,
                        "amount": r.2,
                        "content": r.3,
                        "reference_code": r.4,
                        "date": r.5,
                    }))
                    .collect::<Vec<_>>()
            );
        }
    }
    let sepay_configured = !sepay_cfg.is_empty();
    let pay2s_configured = !pay2s_cfg.is_empty();

    Json(serde_json::json!({
        "sepay": {
            "configured": sepay_configured,
            "webhook_url": "/api/billing/sepay-webhook",
            "bank_name": bank,
            "account_no": acc_no,
            "account_name": acc_name,
        },
        "pay2s": {
            "configured": pay2s_configured,
            "webhook_url": "/api/billing/pay2s-webhook",
        },
        "summary": {
            "total_transactions": transaction_count,
            "total_received_vnd": total_received,
            "total_received_formatted": format!("{:.0}đ", total_received),
        },
        "recent_transactions": recent_transactions,
        "plans": {
            "free": { "price": 0, "msg_per_day": 100, "channels": 3, "expires": false },
            "pro": { "price": 5000000, "msg_per_day": 2000, "channels": 10, "expires": true },
            "business": { "price": 8000000, "msg_per_day": 999999, "channels": 99, "expires": true },
        }
    }))
}

// ── VietQR Payment Generation ────────────────────────────

/// Query params for VietQR generation.
#[derive(serde::Deserialize)]
struct VietQrParams {
    /// Bank BIN code (e.g., 970436 for Vietcombank, 970418 for BIDV)
    bank: String,
    /// Account number
    account: String,
    /// Amount in VND
    #[serde(default)]
    amount: u64,
    /// Transfer content (auto-prefixed with BIZCLAW- for tracking)
    #[serde(default)]
    content: String,
    /// Account holder name
    #[serde(default)]
    name: String,
}

/// GET /api/billing/vietqr — generate VietQR payment link.
///
/// Returns a VietQR image URL that can be displayed in the dashboard or
/// sent to customers via chat channels for easy bank transfer payment.
async fn vietqr_handler(
    axum::extract::Query(params): axum::extract::Query<VietQrParams>,
) -> Json<serde_json::Value> {
    // VietQR standard URL format
    let content = if params.content.is_empty() {
        "BIZCLAW".to_string()
    } else {
        params.content.clone()
    };

    let qr_url = format!(
        "https://img.vietqr.io/image/{}-{}-compact2.jpg?amount={}&addInfo={}&accountName={}",
        params.bank,
        params.account,
        params.amount,
        urlencoding::encode(&content),
        urlencoding::encode(&params.name),
    );

    // Also provide the deep link for mobile banking apps
    let napas_url = format!(
        "https://api.vietqr.io/v2/generate?bank={}&account={}&amount={}&description={}&accountName={}",
        params.bank,
        params.account,
        params.amount,
        urlencoding::encode(&content),
        urlencoding::encode(&params.name),
    );

    Json(serde_json::json!({
        "qr_image_url": qr_url,
        "napas_url": napas_url,
        "bank": params.bank,
        "account": params.account,
        "amount": params.amount,
        "amount_formatted": format!("{}đ", params.amount.to_string()
            .as_bytes().rchunks(3).rev()
            .map(|c| std::str::from_utf8(c).unwrap_or(""))
            .collect::<Vec<_>>().join(".")),
        "content": content,
    }))
}

// ── SME Vertical Templates ────────────────────────────

/// GET /api/admin/templates — list available vertical templates for setup wizard.
async fn list_templates_handler() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "templates": [
            {
                "id": "online_shop",
                "icon": "🏪",
                "name": "Bán hàng Online",
                "description": "Zalo/Facebook shop, tư vấn + chốt đơn tự động",
                "tags": ["ecommerce", "zalo", "facebook"],
                "locale": "vi"
            },
            {
                "id": "tourism",
                "icon": "🏨",
                "name": "Du lịch & Lưu trú",
                "description": "Homestay, khách sạn, tour — auto booking, quét giá đối thủ",
                "tags": ["tourism", "hotel", "booking", "dalat"],
                "locale": "vi"
            },
            {
                "id": "fnb",
                "icon": "🍜",
                "name": "F&B & Dịch vụ",
                "description": "Quán ăn, café — auto đặt bàn, gửi menu, tổng hợp đơn",
                "tags": ["fnb", "restaurant", "cafe", "dalat"],
                "locale": "vi"
            },
            {
                "id": "specialty",
                "icon": "🍓",
                "name": "Đặc sản & Nông sản",
                "description": "Mứt, trà, cà phê — auto nhận đơn, xuất vận đơn, chăm sóc khách",
                "tags": ["specialty", "agriculture", "dalat", "products"],
                "locale": "vi"
            },
            {
                "id": "clinic",
                "icon": "🏥",
                "name": "Phòng khám",
                "description": "Đặt lịch hẹn, tư vấn giờ khám, nhắc nhở lịch",
                "tags": ["clinic", "healthcare"],
                "locale": "vi"
            },
            {
                "id": "education",
                "icon": "🏫",
                "name": "Giáo dục & Đào tạo",
                "description": "Tư vấn khóa học, đăng ký học, lịch học",
                "tags": ["education", "training"],
                "locale": "vi"
            },
            {
                "id": "office",
                "icon": "🏢",
                "name": "Văn phòng",
                "description": "Tóm tắt email, soạn báo cáo, quản lý lịch trình",
                "tags": ["office", "internal"],
                "locale": "vi"
            }
        ]
    }))
}

// ── Error Sanitization (H2 FIX) ────────────────────────

/// Return a sanitized error response — log the real error server-side,
/// send a generic message to the client. Prevents information disclosure.
fn internal_error(context: &str, e: impl std::fmt::Display) -> Json<serde_json::Value> {
    tracing::error!("[{context}] {e}");
    Json(serde_json::json!({"ok": false, "error": "An internal error occurred"}))
}

// ── Nginx Sync ─────────────────────────────────────

/// Regenerate /etc/nginx/conf.d/{domain}-tenants.conf from the DB
/// and reload nginx so new/removed tenants are routed correctly.
/// Runs in a background thread to avoid blocking the HTTP response.
///
/// ## Environment Variables
/// - `NGINX_SSL_CERT_DIR`: Override SSL cert directory name under `/etc/letsencrypt/live/`.
///   Default: uses the domain name. Example: `NGINX_SSL_CERT_DIR=bizclaw.vn-0001`
/// - `NGINX_CONTAINER_NAME`: Docker container name for nginx. Default: `bizclaw-nginx`
/// - `BIZCLAW_BIND_ALL`: If `1`, upstream connects to Docker service hostname; else `127.0.0.1`
async fn sync_nginx_routing(state: &AdminState) {
    let domain = state.domain.clone();
    let tenants = match state.db.lock().await.list_tenants() {
        Ok(t) => t,
        Err(e) => {
            tracing::warn!("nginx-sync[{domain}]: failed to list tenants: {e}");
            return;
        }
    };

    // Spawn background thread so HTTP response is not blocked
    std::thread::spawn(move || {
        // Use domain prefix for map variable names to avoid conflicts between domains
        let domain_slug = domain.replace('.', "_");
        let mut map_entries = String::new();

        // Reserved subdomains that should NOT be treated as tenant slugs
        let reserved_subdomains = ["apps", "www", "api", "admin", "mail", "smtp", "imap", "ftp"];

        for t in &tenants {
            // M5 FIX: Validate slug contains only safe chars before injecting into nginx config
            let safe_slug: String = t
                .slug
                .chars()
                .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
                .collect();
            if safe_slug.is_empty() || safe_slug != t.slug {
                tracing::warn!(
                    "nginx-sync[{domain}]: skipping tenant '{}' — slug contains unsafe chars",
                    t.slug
                );
                continue;
            }
            // Skip reserved subdomains
            if reserved_subdomains.contains(&safe_slug.as_str()) {
                tracing::warn!(
                    "nginx-sync[{domain}]: skipping tenant '{}' — reserved subdomain",
                    t.slug
                );
                continue;
            }
            map_entries.push_str(&format!("    {}      {};\n", safe_slug, t.port));
        }

        // Escape dots in domain for nginx regex
        let domain_regex = domain.replace('.', "\\.");

        // Docker mode: use container hostname; Non-Docker: use 127.0.0.1
        let upstream_host = if std::env::var("BIZCLAW_BIND_ALL").unwrap_or_default() == "1" {
            // In Docker, nginx connects to the app container via network hostname
            // Use NGINX_UPSTREAM_HOST env or derive from HOSTNAME or fallback to domain-based guess
            std::env::var("NGINX_UPSTREAM_HOST").unwrap_or_else(|_| {
                // Try HOSTNAME env (set by Docker to container name)
                std::env::var("HOSTNAME").unwrap_or_else(|_| "bizclaw".to_string())
            })
        } else {
            "127.0.0.1".to_string()
        };

        // SSL cert directory: configurable via NGINX_SSL_CERT_DIR env var
        // Allows overriding when certbot creates directories like "bizclaw.vn-0001"
        let ssl_cert_dir = std::env::var("NGINX_SSL_CERT_DIR").unwrap_or_else(|_| domain.clone());

        let conf = format!(
            r#"# {domain} Dynamic Tenant Routing (auto-generated)
# Generated at: {timestamp}
# Tenants: {tenant_count}

map $subdomain_{domain_slug} $tenant_port_{domain_slug} {{
    default   0;
{map_entries}}}

server {{
    listen 80;
    listen 443 ssl http2;
    server_name ~^(?<subdomain_{domain_slug}>[^.]+)\.{domain_regex}$;

    ssl_certificate /etc/letsencrypt/live/{ssl_cert_dir}/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/{ssl_cert_dir}/privkey.pem;
    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_ciphers HIGH:!aNULL:!MD5;
    ssl_prefer_server_ciphers on;

    add_header X-Frame-Options "SAMEORIGIN" always;
    add_header X-Content-Type-Options "nosniff" always;
    add_header Strict-Transport-Security "max-age=31536000; includeSubDomains" always;

    # Docker internal DNS — required when proxy_pass uses variables
    resolver 127.0.0.11 valid=30s ipv6=off;

    # Reserved subdomains — let static server blocks handle them
    if ($subdomain_{domain_slug} = "apps") {{
        return 444;
    }}
    if ($subdomain_{domain_slug} = "www") {{
        return 444;
    }}

    if ($tenant_port_{domain_slug} = 0) {{
        return 404;
    }}

    location / {{
        set $tenant_upstream http://{upstream_host}:$tenant_port_{domain_slug};
        proxy_pass $tenant_upstream;
        proxy_http_version 1.1;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_read_timeout 86400;
    }}
}}
"#,
            timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
            tenant_count = tenants.len(),
        );

        // Try multiple paths: shared Docker volume first, then traditional path
        let conf_paths = [
            format!("/etc/nginx/dynamic/{domain_slug}-tenants.conf"),
            format!("/etc/nginx/conf.d/{domain_slug}-tenants.conf"),
        ];

        let mut written = false;
        for conf_path in &conf_paths {
            // Ensure directory exists
            if let Some(parent) = std::path::Path::new(conf_path).parent() {
                std::fs::create_dir_all(parent).ok();
            }
            if std::fs::write(conf_path, &conf).is_ok() {
                tracing::info!("nginx-sync[{domain}]: wrote config to {conf_path}");
                written = true;
                break;
            }
        }

        if !written {
            tracing::warn!("nginx-sync[{domain}]: failed to write to any nginx config path");
            return;
        }

        // Try to reload nginx — multiple strategies for Docker vs bare-metal
        // Strategy 0: docker exec (when bizclaw runs in a separate container from nginx)
        let nginx_container =
            std::env::var("NGINX_CONTAINER_NAME").unwrap_or_else(|_| "bizclaw-nginx".to_string());
        if let Ok(out) = std::process::Command::new("docker")
            .args(["exec", &nginx_container, "nginx", "-s", "reload"])
            .output()
            && out.status.success()
        {
            tracing::info!(
                "nginx-sync[{domain}]: {} tenants synced, nginx reloaded (docker exec {})",
                tenants.len(),
                nginx_container
            );
            return;
        }
        // Strategy 1: nginx -s reload (works if nginx is in same container)
        if let Ok(out) = std::process::Command::new("nginx")
            .args(["-s", "reload"])
            .output()
            && out.status.success()
        {
            tracing::info!(
                "nginx-sync[{domain}]: {} tenants synced, nginx reloaded (nginx -s)",
                tenants.len()
            );
            return;
        }
        // Strategy 2: systemctl reload nginx (bare-metal)
        if let Ok(out) = std::process::Command::new("systemctl")
            .args(["reload", "nginx"])
            .output()
            && out.status.success()
        {
            tracing::info!(
                "nginx-sync[{domain}]: {} tenants synced, nginx reloaded (systemctl)",
                tenants.len()
            );
            return;
        }
        // Strategy 3: send HUP to nginx master process (Docker — if pid is available)
        if let Ok(out) = std::process::Command::new("sh")
            .args([
                "-c",
                "kill -HUP $(cat /var/run/nginx.pid 2>/dev/null) 2>/dev/null",
            ])
            .output()
            && out.status.success()
        {
            tracing::info!(
                "nginx-sync[{domain}]: {} tenants synced, nginx reloaded (HUP)",
                tenants.len()
            );
            return;
        }
        // If none worked, config is written but nginx needs manual reload
        tracing::warn!(
            "nginx-sync[{domain}]: config written ({} tenants) but nginx reload failed — may need manual reload",
            tenants.len()
        );
    });
}

// ── RBAC Helpers ──────────────────────────────────────────

/// Check if claims represent the super-admin (platform owner).
/// Security: use role-based check ONLY — admin email is configured at runtime via env var.
fn is_super_admin(claims: &crate::auth::Claims) -> bool {
    claims.role == "superadmin"
}

/// Check if a user role is super-admin (for protecting the owner account).
fn is_super_admin_user(role: &str) -> bool {
    role == "superadmin"
}

/// Check if a user can ACCESS (view) a specific tenant.
/// - superadmin: any tenant
/// - admin: only tenants where owner_id == claims.sub
/// - viewer: only the tenant assigned via JWT tenant_id
fn can_access_tenant(
    claims: &crate::auth::Claims,
    tenant_id: &str,
    db: &crate::db::PlatformDb,
) -> bool {
    if is_super_admin(claims) {
        return true;
    }
    // Admin can access tenants they own
    if claims.role == "admin" {
        if let Ok(tenant) = db.get_tenant(tenant_id) {
            return tenant.owner_id.as_deref() == Some(&claims.sub);
        }
        return false;
    }
    // Viewer can access their assigned tenant
    if claims.role == "viewer" {
        return claims.tenant_id.as_deref() == Some(tenant_id);
    }
    false
}

/// Check if a user can WRITE (create/edit/delete/start/stop) a tenant.
/// - superadmin: any tenant
/// - admin: only tenants where owner_id == claims.sub
/// - viewer: CANNOT write
fn can_write_tenant(
    claims: &crate::auth::Claims,
    tenant_id: &str,
    db: &crate::db::PlatformDb,
) -> bool {
    if claims.role == "viewer" {
        return false;
    }
    can_access_tenant(claims, tenant_id, db)
}

// ── API Handlers ────────────────────────────────────

async fn get_stats(
    State(state): State<Arc<AdminState>>,
    Extension(claims): Extension<crate::auth::Claims>,
) -> Json<serde_json::Value> {
    if is_super_admin(&claims) {
        let (total, running, stopped, error) =
            state.db.lock().await.tenant_stats().unwrap_or((0, 0, 0, 0));
        let users = state
            .db
            .lock()
            .await
            .list_users()
            .map(|u| u.len() as u32)
            .unwrap_or(0);
        Json(serde_json::json!({
            "total_tenants": total, "running": running, "stopped": stopped,
            "error": error, "users": users
        }))
    } else {
        // Non-super-admin: only count their own tenants
        let my_tenants = state
            .db
            .lock()
            .await
            .list_tenants_by_owner(&claims.sub)
            .unwrap_or_default();
        let running = my_tenants.iter().filter(|t| t.status == "running").count() as u32;
        let stopped = my_tenants.iter().filter(|t| t.status == "stopped").count() as u32;
        let error = my_tenants.iter().filter(|t| t.status == "error").count() as u32;
        Json(serde_json::json!({
            "total_tenants": my_tenants.len() as u32, "running": running, "stopped": stopped,
            "error": error, "users": 1
        }))
    }
}

async fn get_activity(
    State(state): State<Arc<AdminState>>,
    Extension(claims): Extension<crate::auth::Claims>,
) -> Json<serde_json::Value> {
    let events = state.db.lock().await.recent_events(20).unwrap_or_default();
    // For non-super-admin, filter to only their events
    if is_super_admin(&claims) {
        Json(serde_json::json!({ "events": events }))
    } else {
        let filtered: Vec<_> = events
            .into_iter()
            .filter(|e| e.actor_id == claims.sub)
            .collect();
        Json(serde_json::json!({ "events": filtered }))
    }
}

async fn list_tenants(
    State(state): State<Arc<AdminState>>,
    Extension(claims): Extension<crate::auth::Claims>,
) -> Json<serde_json::Value> {
    if is_super_admin(&claims) {
        // Superadmin sees ALL tenants
        let tenants = state.db.lock().await.list_tenants().unwrap_or_default();
        Json(serde_json::json!({ "tenants": tenants }))
    } else if claims.role == "admin" {
        // Admin sees only their own tenants (owner_id match)
        let tenants = state
            .db
            .lock()
            .await
            .list_tenants_by_owner(&claims.sub)
            .unwrap_or_default();
        Json(serde_json::json!({ "tenants": tenants }))
    } else {
        // Viewer sees only the single tenant assigned to them
        let db = state.db.lock().await;
        if let Some(tid) = &claims.tenant_id {
            match db.get_tenant(tid) {
                Ok(tenant) => Json(serde_json::json!({ "tenants": [tenant] })),
                Err(_) => Json(serde_json::json!({ "tenants": [] })),
            }
        } else {
            Json(serde_json::json!({ "tenants": [] }))
        }
    }
}

#[derive(serde::Deserialize)]
struct CreateTenantReq {
    name: String,
    slug: String,
    provider: Option<String>,
    model: Option<String>,
    plan: Option<String>,
}

async fn create_tenant(
    State(state): State<Arc<AdminState>>,
    Extension(claims): Extension<crate::auth::Claims>,
    Json(req): Json<CreateTenantReq>,
) -> Json<serde_json::Value> {
    // Demo mode: block all tenant creation
    if std::env::var("BIZCLAW_DEMO_MODE").unwrap_or_default() == "1" {
        return Json(
            serde_json::json!({"ok": false, "error": "🔒 Demo server — không thể tạo tenant mới. Vui lòng tự cài đặt BizClaw để trải nghiệm đầy đủ."}),
        );
    }

    // Role check: viewer cannot create tenants
    if claims.role == "viewer" {
        return Json(
            serde_json::json!({"ok": false, "error": "Không có quyền tạo tenant. Liên hệ admin để nâng cấp role."}),
        );
    }

    // Sanitize slug: only ASCII alphanumeric + hyphens allowed
    let clean_slug = crate::self_serve::generate_safe_slug(&req.slug);
    let slug = if clean_slug.is_empty() {
        crate::self_serve::generate_safe_slug(&req.name)
    } else {
        clean_slug
    };

    let owner_id = claims.sub.clone();

    // Lock DB for both getting port and creating tenant atomically
    let db = state.db.lock().await;
    let used_ports = db.used_ports().unwrap_or_default();
    let mut port = state.base_port;
    while used_ports.contains(&port) {
        port += 1;
    }

    let create_result = db.create_tenant(
        &req.name,
        &slug,
        port,
        req.provider.as_deref().unwrap_or("openai"),
        req.model.as_deref().unwrap_or("gpt-4o-mini"),
        req.plan.as_deref().unwrap_or("free"),
        Some(&owner_id),
    );

    match create_result {
        Ok(tenant) => {
            db.log_event(
                "tenant_created",
                "admin",
                &tenant.id,
                Some(&format!("slug={}", slug)),
            )
            .ok();

            // Auto-start the tenant so subdomain works immediately
            {
                let mut mgr = state.manager.lock().await;
                match mgr.start_tenant(&tenant, &state.bizclaw_bin, &db) {
                    Ok(pid) => {
                        db.update_tenant_status(&tenant.id, "running", Some(pid))
                            .ok();
                        tracing::info!(
                            "auto-start: tenant '{}' started on port {} (pid={})",
                            slug,
                            port,
                            pid
                        );
                    }
                    Err(e) => {
                        db.update_tenant_status(&tenant.id, "error", None).ok();
                        tracing::error!("auto-start failed for tenant '{}': {}", slug, e);
                    }
                }
            }
            sync_nginx_routing(&state).await;
            Json(serde_json::json!({"ok": true, "tenant": tenant}))
        }
        Err(e) => internal_error("admin", e),
    }
}

async fn get_tenant(
    State(state): State<Arc<AdminState>>,
    Extension(claims): Extension<crate::auth::Claims>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    let db = state.db.lock().await;
    if !can_access_tenant(&claims, &id, &db) {
        return Json(
            serde_json::json!({"ok": false, "error": "Không có quyền truy cập tenant này."}),
        );
    }
    match db.get_tenant(&id) {
        Ok(t) => Json(serde_json::json!({"ok": true, "tenant": t})),
        Err(e) => internal_error("admin", e),
    }
}

async fn delete_tenant(
    State(state): State<Arc<AdminState>>,
    Extension(claims): Extension<crate::auth::Claims>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    // Demo mode: block tenant deletion
    if std::env::var("BIZCLAW_DEMO_MODE").unwrap_or_default() == "1" {
        return Json(
            serde_json::json!({"ok": false, "error": "🔒 Demo server — không thể xóa tenant."}),
        );
    }

    // RBAC: only superadmin or owner-admin can delete
    if !can_write_tenant(&claims, &id, &*state.db.lock().await) {
        return Json(serde_json::json!({"ok": false, "error": "Không có quyền xóa tenant này."}));
    }
    state.manager.lock().await.stop_tenant(&id).ok();
    // IMPORTANT: separate lock scopes to avoid Mutex deadlock.
    // delete_tenant lock must be dropped before log_event acquires it again.
    let delete_result = state.db.lock().await.delete_tenant(&id);
    match delete_result {
        Ok(()) => {
            state
                .db
                .lock()
                .await
                .log_event("tenant_deleted", "admin", &id, None)
                .ok();
            sync_nginx_routing(&state).await;
            Json(serde_json::json!({"ok": true}))
        }
        Err(e) => internal_error("admin", e),
    }
}

async fn start_tenant(
    State(state): State<Arc<AdminState>>,
    Extension(claims): Extension<crate::auth::Claims>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    if !can_write_tenant(&claims, &id, &*state.db.lock().await) {
        return Json(
            serde_json::json!({"ok": false, "error": "Không có quyền khởi động tenant này."}),
        );
    }
    let tenant = match state.db.lock().await.get_tenant(&id) {
        Ok(t) => t,
        Err(e) => return internal_error("admin", e),
    };

    let mut mgr = state.manager.lock().await;
    let db = state.db.lock().await;
    match mgr.start_tenant(&tenant, &state.bizclaw_bin, &db) {
        Ok(pid) => {
            drop(db);
            state
                .db
                .lock()
                .await
                .update_tenant_status(&id, "running", Some(pid))
                .ok();
            state
                .db
                .lock()
                .await
                .log_event("tenant_started", "admin", &id, None)
                .ok();
            sync_nginx_routing(&state).await;
            Json(serde_json::json!({"ok": true, "pid": pid}))
        }
        Err(e) => {
            drop(db);
            state
                .db
                .lock()
                .await
                .update_tenant_status(&id, "error", None)
                .ok();
            internal_error("start_tenant", e)
        }
    }
}

async fn stop_tenant(
    State(state): State<Arc<AdminState>>,
    Extension(claims): Extension<crate::auth::Claims>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    if !can_write_tenant(&claims, &id, &*state.db.lock().await) {
        return Json(serde_json::json!({"ok": false, "error": "Không có quyền dừng tenant này."}));
    }
    state.manager.lock().await.stop_tenant(&id).ok();
    state
        .db
        .lock()
        .await
        .update_tenant_status(&id, "stopped", None)
        .ok();
    state
        .db
        .lock()
        .await
        .log_event("tenant_stopped", "admin", &id, None)
        .ok();
    sync_nginx_routing(&state).await;
    Json(serde_json::json!({"ok": true}))
}

async fn restart_tenant(
    State(state): State<Arc<AdminState>>,
    Extension(claims): Extension<crate::auth::Claims>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    if !can_write_tenant(&claims, &id, &*state.db.lock().await) {
        return Json(
            serde_json::json!({"ok": false, "error": "Không có quyền khởi động lại tenant này."}),
        );
    }
    let tenant = match state.db.lock().await.get_tenant(&id) {
        Ok(t) => t,
        Err(e) => return internal_error("admin", e),
    };

    // IMPORTANT: separate lock scopes to avoid Mutex deadlock
    let restart_result = {
        let mut mgr = state.manager.lock().await;
        let db = state.db.lock().await;
        mgr.restart_tenant(&tenant, &state.bizclaw_bin, &db)
    }; // Both locks dropped here
    match restart_result {
        Ok(pid) => {
            state
                .db
                .lock()
                .await
                .update_tenant_status(&id, "running", Some(pid))
                .ok();
            state
                .db
                .lock()
                .await
                .log_event("tenant_restarted", "admin", &id, None)
                .ok();
            sync_nginx_routing(&state).await;
            Json(serde_json::json!({"ok": true, "pid": pid}))
        }
        Err(e) => {
            state
                .db
                .lock()
                .await
                .update_tenant_status(&id, "error", None)
                .ok();
            internal_error("restart_tenant", e)
        }
    }
}

// Pairing code management removed — SaaS uses JWT from Platform login

async fn list_users(
    State(state): State<Arc<AdminState>>,
    Extension(claims): Extension<crate::auth::Claims>,
) -> Json<serde_json::Value> {
    if is_super_admin(&claims) {
        let users = state.db.lock().await.list_users().unwrap_or_default();
        Json(serde_json::json!({"users": users}))
    } else {
        // Non-super-admin can only see themselves
        let all_users = state.db.lock().await.list_users().unwrap_or_default();
        let my_users: Vec<_> = all_users
            .into_iter()
            .filter(|u| u.id == claims.sub)
            .collect();
        Json(serde_json::json!({"users": my_users}))
    }
}

#[derive(serde::Deserialize)]
struct LoginReq {
    email: String,
    password: String,
}

async fn login(
    State(state): State<Arc<AdminState>>,
    Json(req): Json<LoginReq>,
) -> Json<serde_json::Value> {
    // Rate limiting — max 5 attempts per email per 5 minutes
    {
        let mut attempts = state
            .login_attempts
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        let now = std::time::Instant::now();
        if let Some((count, first_at)) = attempts.get(&req.email) {
            if now.duration_since(*first_at).as_secs() < 300 && *count >= 5 {
                return Json(serde_json::json!({
                    "ok": false,
                    "error": "Too many login attempts. Please wait 5 minutes."
                }));
            }
            // Reset if window expired
            if now.duration_since(*first_at).as_secs() >= 300 {
                attempts.remove(&req.email);
            }
        }
        // Record attempt
        let entry = attempts.entry(req.email.clone()).or_insert((0, now));
        entry.0 += 1;
    }

    tracing::debug!("login: querying user {}", req.email);
    let user = state.db.lock().await.get_user_by_email(&req.email);
    match user {
        Ok(Some((id, hash, role))) => {
            // Run bcrypt in blocking thread to avoid stalling the async runtime
            let password = req.password.clone();
            let hash_clone = hash.clone();
            tracing::debug!("login: verifying password");
            let ok = tokio::task::spawn_blocking(move || {
                crate::auth::verify_password(&password, &hash_clone)
            })
            .await
            .unwrap_or(false);

            if ok {
                tracing::debug!("login: password verified, generating token");
                // Get tenant_id and status for JWT — direct query instead of list_users
                let (tenant_id, user_status) = {
                    let db = state.db.lock().await;
                    match db.get_user_by_id(&id) {
                        Ok(Some(u)) => {
                            let tid = if u.status == "pending" {
                                None
                            } else {
                                u.tenant_id.clone()
                            };
                            (tid, u.status.clone())
                        }
                        _ => (None, "active".into()),
                    }
                };
                if user_status == "pending" {
                    return Json(serde_json::json!({
                        "ok": false,
                        "error": "Tài khoản đang chờ duyệt. Vui lòng liên hệ admin để kích hoạt."
                    }));
                }
                if user_status == "suspended" {
                    return Json(serde_json::json!({
                        "ok": false,
                        "error": "Tài khoản đã bị tạm khóa. Vui lòng liên hệ admin."
                    }));
                }
                match crate::auth::create_token(
                    &id,
                    &req.email,
                    &role,
                    tenant_id.as_deref(),
                    &state.jwt_secret,
                ) {
                    Ok(token) => {
                        state
                            .db
                            .lock()
                            .await
                            .log_event("login_success", "user", &id, None)
                            .ok();
                        Json(serde_json::json!({"ok": true, "token": token, "role": role}))
                    }
                    Err(e) => {
                        tracing::error!("login: Token error: {e}");
                        Json(serde_json::json!({"ok": false, "error": e}))
                    }
                }
            } else {
                tracing::warn!("login: Invalid credentials for {}", req.email);
                Json(serde_json::json!({"ok": false, "error": "Invalid credentials"}))
            }
        }
        Ok(None) => Json(serde_json::json!({"ok": false, "error": "Invalid credentials"})),
        Err(e) => {
            tracing::error!("login: DB error: {e}");
            Json(
                serde_json::json!({"ok": false, "error": "An internal error occurred. Please try again."}),
            )
        }
    }
}

/// Deprecated — pairing code validation endpoint kept for backward compat.
/// Returns deprecation notice regardless of input.
async fn validate_pairing_deprecated(
    State(_state): State<Arc<AdminState>>,
    Json(_body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "ok": false,
        "error": "Pairing code đã bị loại bỏ. Vui lòng đăng nhập qua Platform để nhận JWT token.",
        "deprecated": true,
        "redirect": "/login"
    }))
}

async fn admin_dashboard_page() -> impl axum::response::IntoResponse {
    (
        [
            (
                axum::http::header::CACHE_CONTROL,
                "no-cache, no-store, must-revalidate",
            ),
            (axum::http::header::PRAGMA, "no-cache"),
        ],
        axum::response::Html(include_str!("admin_dashboard.html")),
    )
}

async fn pixel_office_page() -> axum::response::Html<&'static str> {
    axum::response::Html(include_str!("../../../data/pixel_office.html"))
}

async fn hub_skills_page() -> impl axum::response::IntoResponse {
    (
        [
            (axum::http::header::CACHE_CONTROL, "public, max-age=300"),
            (axum::http::header::CONTENT_TYPE, "text/html; charset=utf-8"),
        ],
        axum::response::Html(include_str!("../../bizclaw-gateway/src/dashboard/hub.html")),
    )
}

// ── Channel Configuration Handlers ────────────────────────────────────

async fn list_channels(
    State(state): State<Arc<AdminState>>,
    Extension(claims): Extension<crate::auth::Claims>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    // IMPORTANT: separate lock scopes — can_access_tenant lock must be dropped
    // before list_channels acquires its own lock, otherwise Mutex deadlock.
    {
        let db = state.db.lock().await;
        if !can_access_tenant(&claims, &id, &db) {
            return Json(
                serde_json::json!({"ok": false, "error": "Không có quyền truy cập tenant này."}),
            );
        }
    } // lock dropped here
    match state.db.lock().await.list_channels(&id) {
        Ok(channels) => Json(serde_json::json!({"ok": true, "channels": channels})),
        Err(e) => internal_error("admin", e),
    }
}

#[derive(serde::Deserialize)]
struct UpsertChannelReq {
    channel_type: String,
    enabled: bool,
    config: serde_json::Value,
}

async fn upsert_channel(
    State(state): State<Arc<AdminState>>,
    Extension(claims): Extension<crate::auth::Claims>,
    Path(id): Path<String>,
    Json(req): Json<UpsertChannelReq>,
) -> Json<serde_json::Value> {
    if !can_write_tenant(&claims, &id, &*state.db.lock().await) {
        return Json(
            serde_json::json!({"ok": false, "error": "Không có quyền cấu hình tenant này."}),
        );
    }
    let config_json = serde_json::to_string(&req.config).unwrap_or_default();
    // IMPORTANT: separate lock scopes to avoid Mutex deadlock
    let upsert_result =
        state
            .db
            .lock()
            .await
            .upsert_channel(&id, &req.channel_type, req.enabled, &config_json);
    match upsert_result {
        Ok(channel) => {
            state
                .db
                .lock()
                .await
                .log_event(
                    "channel_configured",
                    "admin",
                    &id,
                    Some(&format!(
                        "type={}, enabled={}",
                        req.channel_type, req.enabled
                    )),
                )
                .ok();
            Json(serde_json::json!({"ok": true, "channel": channel}))
        }
        Err(e) => internal_error("admin", e),
    }
}

async fn delete_channel(
    State(state): State<Arc<AdminState>>,
    Extension(claims): Extension<crate::auth::Claims>,
    Path((tenant_id, channel_id)): Path<(String, String)>,
) -> Json<serde_json::Value> {
    if !can_write_tenant(&claims, &tenant_id, &*state.db.lock().await) {
        return Json(serde_json::json!({"ok": false, "error": "Không có quyền xóa channel."}));
    }
    // IMPORTANT: separate lock scopes to avoid Mutex deadlock
    let del_result = state.db.lock().await.delete_channel(&channel_id);
    match del_result {
        Ok(()) => {
            state
                .db
                .lock()
                .await
                .log_event(
                    "channel_deleted",
                    "admin",
                    &tenant_id,
                    Some(&format!("channel_id={}", channel_id)),
                )
                .ok();
            Json(serde_json::json!({"ok": true}))
        }
        Err(e) => internal_error("admin", e),
    }
}

/// Zalo QR code generation endpoint — returns QR data URL for scanning.
async fn zalo_get_qr(
    State(_state): State<Arc<AdminState>>,
    Path(_id): Path<String>,
) -> Json<serde_json::Value> {
    use bizclaw_channels::zalo::client::auth::{ZaloAuth, ZaloCredentials};

    let creds = ZaloCredentials::default();
    let mut auth = ZaloAuth::new(creds);

    match auth.get_qr_code().await {
        Ok(qr) => Json(serde_json::json!({
            "ok": true,
            "qr_code": qr.image,
            "qr_id": qr.code,
            "imei": auth.credentials().imei,
            "instructions": [
                "1. Mở ứng dụng Zalo trên điện thoại",
                "2. Nhấn biểu tượng QR ở thanh tìm kiếm",
                "3. Quét mã QR này để đăng nhập",
                "4. Xác nhận đăng nhập trên điện thoại"
            ],
            "message": "Quét mã QR bằng Zalo trên điện thoại"
        })),
        Err(e) => {
            tracing::error!("[zalo_qr] {e}");
            Json(serde_json::json!({
                "ok": false,
                "error": "Không thể tạo mã QR Zalo",
                "fallback": "Vui lòng vào chat.zalo.me → F12 → Application → Cookies → Copy toàn bộ và paste vào ô Cookie bên dưới"
            }))
        }
    }
}

// ═══════════════════════════════════════════════════════════
// OLLAMA / BRAIN ENGINE MANAGEMENT
// ═══════════════════════════════════════════════════════════

fn ollama_url() -> String {
    std::env::var("OLLAMA_HOST").unwrap_or_else(|_| "http://localhost:11434".to_string())
}

/// Check if Ollama is running.
async fn ollama_health(State(_state): State<Arc<AdminState>>) -> Json<serde_json::Value> {
    let client = reqwest::Client::new();
    let url = ollama_url();
    match client.get(format!("{url}/api/tags")).send().await {
        Ok(r) if r.status().is_success() => {
            Json(serde_json::json!({"ok": true, "url": url, "status": "running"}))
        }
        Ok(r) => Json(serde_json::json!({
            "ok": false, "url": url,
            "status": format!("unhealthy: {}", r.status())
        })),
        Err(e) => {
            tracing::error!("[ollama_health] {e}");
            Json(serde_json::json!({
                "ok": false, "url": url,
                "status": "not_running",
                "error": "Ollama is not reachable",
                "install_guide": "curl -fsSL https://ollama.ai/install.sh | sh"
            }))
        }
    }
}

/// List installed Ollama models.
async fn ollama_list_models(State(_state): State<Arc<AdminState>>) -> Json<serde_json::Value> {
    let client = reqwest::Client::new();
    let url = ollama_url();
    match client.get(format!("{url}/api/tags")).send().await {
        Ok(r) if r.status().is_success() => {
            let body: serde_json::Value = r.json().await.unwrap_or_default();
            let models: Vec<serde_json::Value> = body["models"]
                .as_array()
                .map(|arr| {
                    arr.iter().map(|m| {
                    let size_bytes = m["size"].as_u64().unwrap_or(0);
                    let size_mb = size_bytes as f64 / 1_048_576.0;
                    serde_json::json!({
                        "name": m["name"].as_str().unwrap_or(""),
                        "size": format!("{:.0} MB", size_mb),
                        "size_bytes": size_bytes,
                        "modified_at": m["modified_at"].as_str().unwrap_or(""),
                        "family": m["details"]["family"].as_str().unwrap_or(""),
                        "parameter_size": m["details"]["parameter_size"].as_str().unwrap_or(""),
                        "quantization": m["details"]["quantization_level"].as_str().unwrap_or(""),
                    })
                }).collect()
                })
                .unwrap_or_default();
            Json(serde_json::json!({"ok": true, "models": models}))
        }
        _ => Json(serde_json::json!({
            "ok": false,
            "models": [],
            "error": "Ollama not running. Install: curl -fsSL https://ollama.ai/install.sh | sh"
        })),
    }
}

/// Pull (download) a model.
async fn ollama_pull_model(
    State(_state): State<Arc<AdminState>>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let model = body["model"].as_str().unwrap_or("tinyllama");
    let client = reqwest::Client::new();
    let url = ollama_url();

    tracing::info!("Pulling Ollama model: {}", model);

    match client
        .post(format!("{url}/api/pull"))
        .json(&serde_json::json!({"name": model, "stream": false}))
        .timeout(std::time::Duration::from_secs(600)) // 10 min timeout
        .send()
        .await
    {
        Ok(r) if r.status().is_success() => {
            let resp: serde_json::Value = r.json().await.unwrap_or_default();
            tracing::info!("Model pulled: {}", model);
            Json(serde_json::json!({
                "ok": true,
                "model": model,
                "status": resp["status"].as_str().unwrap_or("success"),
                "message": format!("Model '{}' pulled successfully!", model)
            }))
        }
        Ok(r) => {
            let text = r.text().await.unwrap_or_default();
            Json(serde_json::json!({"ok": false, "error": text}))
        }
        Err(e) => {
            tracing::error!("[ollama_pull] {e}");
            Json(serde_json::json!({
                "ok": false,
                "error": "Ollama is not reachable",
                "hint": "Ollama might not be installed. Run: curl -fsSL https://ollama.ai/install.sh | sh"
            }))
        }
    }
}

/// Delete a model.
async fn ollama_delete_model(
    State(_state): State<Arc<AdminState>>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let model = body["model"].as_str().unwrap_or("");
    if model.is_empty() {
        return Json(serde_json::json!({"ok": false, "error": "Missing model name"}));
    }

    let client = reqwest::Client::new();
    let url = ollama_url();

    match client
        .delete(format!("{url}/api/delete"))
        .json(&serde_json::json!({"name": model}))
        .send()
        .await
    {
        Ok(r) if r.status().is_success() => {
            Json(serde_json::json!({"ok": true, "message": format!("Model '{}' deleted", model)}))
        }
        Ok(r) => {
            let text = r.text().await.unwrap_or_default();
            Json(serde_json::json!({"ok": false, "error": text}))
        }
        Err(e) => internal_error("admin", e),
    }
}

// ═════════════════════════════════════════════════════════════
// TENANT CONFIG (KEY-VALUE SETTINGS) — DB as source of truth
// ═════════════════════════════════════════════════════════════

/// List all config entries for a tenant.
async fn list_tenant_configs(
    State(state): State<Arc<AdminState>>,
    Extension(claims): Extension<crate::auth::Claims>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    // IMPORTANT: separate lock scopes — can_access_tenant lock must be dropped
    // before list_configs acquires its own lock, otherwise Mutex deadlock.
    {
        let db = state.db.lock().await;
        if !can_access_tenant(&claims, &id, &db) {
            return Json(
                serde_json::json!({"ok": false, "error": "Không có quyền truy cập tenant này."}),
            );
        }
    } // lock dropped here
    match state.db.lock().await.list_configs(&id) {
        Ok(configs) => {
            let mut obj = serde_json::Map::new();
            for cfg in &configs {
                obj.insert(
                    cfg.key.clone(),
                    serde_json::Value::String(cfg.value.clone()),
                );
            }
            Json(serde_json::json!({"ok": true, "configs": obj}))
        }
        Err(e) => internal_error("admin", e),
    }
}

/// Set one or more config values for a tenant.
/// Body: {"configs": {"default_provider": "ollama", "default_model": "llama3.2", ...}}
async fn set_tenant_configs(
    State(state): State<Arc<AdminState>>,
    Extension(claims): Extension<crate::auth::Claims>,
    Path(id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    if !can_write_tenant(&claims, &id, &*state.db.lock().await) {
        return Json(
            serde_json::json!({"ok": false, "error": "Không có quyền cấu hình tenant này."}),
        );
    }
    let configs = match body.get("configs").and_then(|c| c.as_object()) {
        Some(c) => c,
        None => return Json(serde_json::json!({"ok": false, "error": "Missing 'configs' object"})),
    };

    let db = state.db.lock().await;
    let mut saved_count = 0;
    for (key, value) in configs {
        let val_str = match value {
            serde_json::Value::String(s) => s.clone(),
            other => other.to_string(),
        };
        if db.set_config(&id, key, &val_str).is_ok() {
            saved_count += 1;
        }
    }

    // Also update the tenants table provider/model for consistency
    if let Some(provider) = configs.get("default_provider").and_then(|v| v.as_str())
        && let Some(model) = configs.get("default_model").and_then(|v| v.as_str())
    {
        db.update_tenant_provider(&id, provider, model).ok();
    }

    drop(db);
    state
        .db
        .lock()
        .await
        .log_event(
            "config_updated",
            "admin",
            &id,
            Some(&format!("keys={}", saved_count)),
        )
        .ok();

    Json(serde_json::json!({"ok": true, "saved": saved_count}))
}

// ═════════════════════════════════════════════════════════════
// TENANT AGENTS — DB-backed agent persistence
// ═════════════════════════════════════════════════════════════

/// List all agents for a tenant.
async fn list_tenant_agents(
    State(state): State<Arc<AdminState>>,
    Extension(claims): Extension<crate::auth::Claims>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    // IMPORTANT: separate lock scopes — can_access_tenant lock must be dropped
    // before list_agents acquires its own lock, otherwise Mutex deadlock.
    {
        let db = state.db.lock().await;
        if !can_access_tenant(&claims, &id, &db) {
            return Json(
                serde_json::json!({"ok": false, "error": "Không có quyền truy cập tenant này."}),
            );
        }
    } // lock dropped here
    match state.db.lock().await.list_agents(&id) {
        Ok(agents) => Json(serde_json::json!({"ok": true, "agents": agents})),
        Err(e) => internal_error("admin", e),
    }
}

#[derive(serde::Deserialize)]
struct UpsertAgentReq {
    name: String,
    role: Option<String>,
    description: Option<String>,
    provider: Option<String>,
    model: Option<String>,
    system_prompt: Option<String>,
}

/// Create or update an agent for a tenant.
async fn upsert_tenant_agent(
    State(state): State<Arc<AdminState>>,
    Extension(claims): Extension<crate::auth::Claims>,
    Path(id): Path<String>,
    Json(req): Json<UpsertAgentReq>,
) -> Json<serde_json::Value> {
    if !can_write_tenant(&claims, &id, &*state.db.lock().await) {
        return Json(
            serde_json::json!({"ok": false, "error": "Không có quyền cấu hình tenant này."}),
        );
    }
    let db = state.db.lock().await;

    // Get tenant defaults for fallback values
    let tenant = db.get_tenant(&id).ok();
    let default_provider = tenant
        .as_ref()
        .map(|t| t.provider.as_str())
        .unwrap_or("openai");
    let default_model = tenant
        .as_ref()
        .map(|t| t.model.as_str())
        .unwrap_or("gpt-4o-mini");

    match db.upsert_agent(
        &id,
        &req.name,
        req.role.as_deref().unwrap_or("assistant"),
        req.description.as_deref().unwrap_or(""),
        req.provider.as_deref().unwrap_or(default_provider),
        req.model.as_deref().unwrap_or(default_model),
        req.system_prompt.as_deref().unwrap_or(""),
    ) {
        Ok(agent) => {
            drop(db);
            state
                .db
                .lock()
                .await
                .log_event(
                    "agent_upserted",
                    "admin",
                    &id,
                    Some(&format!("name={}", req.name)),
                )
                .ok();
            Json(serde_json::json!({"ok": true, "agent": agent}))
        }
        Err(e) => internal_error("admin", e),
    }
}

/// Delete an agent by tenant_id + name.
async fn delete_tenant_agent(
    State(state): State<Arc<AdminState>>,
    Extension(claims): Extension<crate::auth::Claims>,
    Path((id, name)): Path<(String, String)>,
) -> Json<serde_json::Value> {
    if !can_write_tenant(&claims, &id, &*state.db.lock().await) {
        return Json(serde_json::json!({"ok": false, "error": "Không có quyền xóa agent."}));
    }
    // IMPORTANT: separate lock scopes to avoid Mutex deadlock
    let del_result = state.db.lock().await.delete_agent_by_name(&id, &name);
    match del_result {
        Ok(()) => {
            state
                .db
                .lock()
                .await
                .log_event(
                    "agent_deleted",
                    "admin",
                    &id,
                    Some(&format!("name={}", name)),
                )
                .ok();
            Json(serde_json::json!({"ok": true}))
        }
        Err(e) => internal_error("admin", e),
    }
}

// ═════════════════════════════════════════════════════════════
// USER MANAGEMENT HANDLERS
// ═════════════════════════════════════════════════════════════

#[derive(serde::Deserialize)]
struct CreateUserReq {
    email: String,
    password: String,
    role: Option<String>,
    tenant_id: Option<String>,
}

async fn create_user_handler(
    State(state): State<Arc<AdminState>>,
    Extension(claims): Extension<crate::auth::Claims>,
    Json(req): Json<CreateUserReq>,
) -> Json<serde_json::Value> {
    if !is_super_admin(&claims) {
        return Json(
            serde_json::json!({"ok": false, "error": "Chỉ Super Admin mới có quyền tạo user."}),
        );
    }
    tracing::debug!("create_user: {}", req.email);
    if req.email.is_empty() || req.password.is_empty() {
        return Json(serde_json::json!({"ok": false, "error": "Email and password are required"}));
    }

    // Hash password in blocking thread
    let password = req.password.clone();
    tracing::debug!("create_user: hashing password");
    let hash = match tokio::task::spawn_blocking(move || crate::auth::hash_password(&password))
        .await
    {
        Ok(Ok(h)) => h,
        Ok(Err(e)) => {
            tracing::error!("create_user: Hash error: {e}");
            return Json(serde_json::json!({"ok": false, "error": "Failed to process password"}));
        }
        Err(e) => {
            tracing::error!("create_user: Spawn error: {e}");
            return Json(serde_json::json!({"ok": false, "error": "Internal error"}));
        }
    };

    let role = req.role.as_deref().unwrap_or("admin");
    // Extracted lock to avoid deadlock with subsequent log_event lock
    let db_res = state.db.lock().await.create_user(
        &req.email,
        &hash,
        role,
        req.tenant_id.as_deref().filter(|s| !s.is_empty()),
    );

    match db_res {
        Ok(id) => {
            state
                .db
                .lock()
                .await
                .log_event(
                    "user_created",
                    "admin",
                    &id,
                    Some(&format!("email={}", req.email)),
                )
                .ok();
            Json(serde_json::json!({"ok": true, "id": id}))
        }
        Err(e) => {
            tracing::error!("create_user_handler: Error creating user: {e}");
            // Check for duplicate email
            let msg = if e.to_string().contains("UNIQUE") {
                "Email already exists"
            } else {
                "Failed to create user"
            };
            Json(serde_json::json!({"ok": false, "error": msg}))
        }
    }
}

async fn delete_user_handler(
    State(state): State<Arc<AdminState>>,
    Extension(claims): Extension<crate::auth::Claims>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    // Only super-admin can delete users
    if !is_super_admin(&claims) {
        return Json(
            serde_json::json!({"ok": false, "error": "Only super admin can delete users"}),
        );
    }

    tracing::info!(
        "delete_user_handler: Cascade deleting user {} and their tenants",
        id
    );

    // Stop all tenant processes first
    let tenant_ids = state
        .db
        .lock()
        .await
        .list_tenants_by_owner(&id)
        .unwrap_or_default()
        .into_iter()
        .map(|t| t.id)
        .collect::<Vec<_>>();
    for tid in &tenant_ids {
        state.manager.lock().await.stop_tenant(tid).ok();
    }

    // Cascade delete user + tenants
    let db_res = state.db.lock().await.delete_user_cascade(&id);

    match db_res {
        Ok(deleted_tenants) => {
            tracing::info!(
                "delete_user_handler: Deleted user {} and {} tenants",
                id,
                deleted_tenants.len()
            );
            state
                .db
                .lock()
                .await
                .log_event(
                    "user_deleted_cascade",
                    "admin",
                    &id,
                    Some(&format!("tenants_deleted={}", deleted_tenants.len())),
                )
                .ok();
            if !deleted_tenants.is_empty() {
                sync_nginx_routing(&state).await;
            }
            Json(serde_json::json!({"ok": true, "tenants_deleted": deleted_tenants.len()}))
        }
        Err(e) => internal_error("delete_user", e),
    }
}

#[derive(serde::Deserialize)]
struct AssignTenantReq {
    tenant_id: Option<String>,
}

/// Assign or remove tenant for a user
async fn assign_tenant_handler(
    State(state): State<Arc<AdminState>>,
    Extension(claims): Extension<crate::auth::Claims>,
    Path(id): Path<String>,
    Json(req): Json<AssignTenantReq>,
) -> Json<serde_json::Value> {
    if !is_super_admin(&claims) {
        return Json(
            serde_json::json!({"ok": false, "error": "Chỉ Super Admin mới có quyền gán tenant."}),
        );
    }
    let tenant_id_str = req.tenant_id.as_deref().filter(|s| !s.is_empty());
    let db_res = state.db.lock().await.update_user_tenant(&id, tenant_id_str);

    match db_res {
        Ok(()) => {
            let details = format!("tenant_id={}", tenant_id_str.unwrap_or("none"));
            state
                .db
                .lock()
                .await
                .log_event("user_assigned", "admin", &id, Some(&details))
                .ok();
            Json(serde_json::json!({"ok": true}))
        }
        Err(e) => internal_error("admin", e),
    }
}

// ═════════════════════════════════════════════════════════════
// ADMIN RESET USER PASSWORD (Super Admin only)
// ═════════════════════════════════════════════════════════════

#[derive(serde::Deserialize)]
struct AdminResetPasswordReq {
    new_password: String,
}

/// Super Admin force-resets a user's password (no old password required).
async fn admin_reset_user_password(
    State(state): State<Arc<AdminState>>,
    Extension(claims): Extension<crate::auth::Claims>,
    Path(id): Path<String>,
    Json(req): Json<AdminResetPasswordReq>,
) -> Json<serde_json::Value> {
    if !is_super_admin(&claims) {
        return Json(
            serde_json::json!({"ok": false, "error": "Chỉ Super Admin mới có quyền reset mật khẩu."}),
        );
    }
    if req.new_password.len() < 8 {
        return Json(
            serde_json::json!({"ok": false, "error": "Password must be at least 8 characters"}),
        );
    }

    let new_pwd = req.new_password.clone();
    let hash = match tokio::task::spawn_blocking(move || crate::auth::hash_password(&new_pwd)).await
    {
        Ok(Ok(h)) => h,
        Ok(Err(e)) => return internal_error("hash_password", e),
        Err(e) => return internal_error("hash_task", e),
    };

    let db_res = state.db.lock().await.update_user_password(&id, &hash);
    match db_res {
        Ok(()) => {
            state
                .db
                .lock()
                .await
                .log_event(
                    "admin_password_reset",
                    "admin",
                    &id,
                    Some("password force-reset by admin"),
                )
                .ok();
            Json(serde_json::json!({"ok": true}))
        }
        Err(e) => internal_error("admin", e),
    }
}

// ═════════════════════════════════════════════════════════════
// USER STATUS MANAGEMENT (Super Admin only)
// ═════════════════════════════════════════════════════════════

#[derive(serde::Deserialize)]
struct UpdateUserStatusReq {
    status: String, // pending, active, suspended
}

/// Super Admin approves/suspends a user account.
async fn update_user_status_handler(
    State(state): State<Arc<AdminState>>,
    Extension(claims): Extension<crate::auth::Claims>,
    Path(id): Path<String>,
    Json(req): Json<UpdateUserStatusReq>,
) -> Json<serde_json::Value> {
    if !is_super_admin(&claims) {
        return Json(
            serde_json::json!({"ok": false, "error": "Only super admin can change user status"}),
        );
    }

    let valid_statuses = ["pending", "active", "suspended"];
    if !valid_statuses.contains(&req.status.as_str()) {
        return Json(
            serde_json::json!({"ok": false, "error": "Invalid status. Must be: pending, active, suspended"}),
        );
    }

    let db_res = state.db.lock().await.update_user_status(&id, &req.status);
    match db_res {
        Ok(()) => {
            state
                .db
                .lock()
                .await
                .log_event(
                    "user_status_changed",
                    "admin",
                    &id,
                    Some(&format!("status={}", req.status)),
                )
                .ok();

            // If activating a user, try to start their stopped tenant(s)
            if req.status == "active" {
                let tenants = state
                    .db
                    .lock()
                    .await
                    .list_tenants_by_owner(&id)
                    .unwrap_or_default();
                let stopped: Vec<_> = tenants
                    .iter()
                    .filter(|t| t.status == "stopped")
                    .cloned()
                    .collect();
                for tenant in &stopped {
                    // Acquire locks one at a time — never hold db + manager simultaneously
                    let start_result = {
                        let mut mgr = state.manager.lock().await;
                        let db_ref = state.db.lock().await;
                        mgr.start_tenant(tenant, &state.bizclaw_bin, &db_ref)
                    }; // Both locks dropped here
                    match start_result {
                        Ok(pid) => {
                            state
                                .db
                                .lock()
                                .await
                                .update_tenant_status(&tenant.id, "running", Some(pid))
                                .ok();
                            tracing::info!(
                                "user-activate: started tenant '{}' (pid={})",
                                tenant.slug,
                                pid
                            );
                        }
                        Err(e) => {
                            state
                                .db
                                .lock()
                                .await
                                .update_tenant_status(&tenant.id, "error", None)
                                .ok();
                            tracing::warn!(
                                "user-activate: failed to start tenant '{}': {e}",
                                tenant.slug
                            );
                        }
                    }
                }
                if !tenants.is_empty() {
                    sync_nginx_routing(&state).await;
                }
            }

            Json(serde_json::json!({"ok": true}))
        }
        Err(e) => internal_error("admin", e),
    }
}

// ═════════════════════════════════════════════════════════════
// USER ROLE MANAGEMENT (Super Admin only)
// ═════════════════════════════════════════════════════════════

#[derive(serde::Deserialize)]
struct UpdateUserRoleReq {
    role: String, // superadmin, admin, viewer
}

/// Super Admin changes a user's role.
async fn update_user_role_handler(
    State(state): State<Arc<AdminState>>,
    Extension(claims): Extension<crate::auth::Claims>,
    Path(id): Path<String>,
    Json(req): Json<UpdateUserRoleReq>,
) -> Json<serde_json::Value> {
    if !is_super_admin(&claims) {
        return Json(
            serde_json::json!({"ok": false, "error": "Chỉ Super Admin mới có quyền đổi role."}),
        );
    }

    let valid_roles = ["superadmin", "admin", "viewer"];
    if !valid_roles.contains(&req.role.as_str()) {
        return Json(
            serde_json::json!({"ok": false, "error": "Role không hợp lệ. Phải là: superadmin, admin, viewer"}),
        );
    }

    // Protect the platform owner account
    {
        let db = state.db.lock().await;
        let users = db.list_users().unwrap_or_default();
        if let Some(target) = users.iter().find(|u| u.id == id)
            && is_super_admin_user(&target.role)
        {
            return Json(
                serde_json::json!({"ok": false, "error": "Không thể thay đổi role của Super Admin gốc."}),
            );
        }
    }

    let db_res = state.db.lock().await.update_user_role(&id, &req.role);
    match db_res {
        Ok(()) => {
            state
                .db
                .lock()
                .await
                .log_event(
                    "user_role_changed",
                    "admin",
                    &id,
                    Some(&format!("role={}", req.role)),
                )
                .ok();
            Json(serde_json::json!({"ok": true}))
        }
        Err(e) => internal_error("admin", e),
    }
}

// ════════════════════════════════════════════════════════════════════
// ENTERPRISE HANDLERS: Multi-user RBAC
// ════════════════════════════════════════════════════════════════════

async fn list_members(
    State(state): State<Arc<AdminState>>,
    Extension(claims): Extension<crate::auth::Claims>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    if !can_access_tenant(&claims, &id, &*state.db.lock().await) {
        return Json(serde_json::json!({"ok": false, "error": "Không có quyền truy cập."}));
    }
    match &state.pg_db {
        Some(pg) => match pg.list_tenant_members(&id).await {
            Ok(members) => Json(serde_json::json!({"ok": true, "members": members})),
            Err(e) => internal_error("list_members", e),
        },
        None => Json(
            serde_json::json!({"ok": false, "error": "PostgreSQL required for enterprise features"}),
        ),
    }
}

#[derive(serde::Deserialize)]
struct InviteMemberReq {
    email: String,
    role: String,
}

async fn invite_member(
    State(state): State<Arc<AdminState>>,
    Extension(claims): Extension<crate::auth::Claims>,
    Path(id): Path<String>,
    Json(req): Json<InviteMemberReq>,
) -> Json<serde_json::Value> {
    if !can_write_tenant(&claims, &id, &*state.db.lock().await) {
        return Json(serde_json::json!({"ok": false, "error": "Không có quyền mời thành viên."}));
    }
    let valid_roles = ["owner", "admin", "operator", "viewer"];
    if !valid_roles.contains(&req.role.as_str()) {
        return Json(
            serde_json::json!({"ok": false, "error": "Role không hợp lệ. Phải là: owner, admin, operator, viewer"}),
        );
    }
    match &state.pg_db {
        Some(pg) => match pg
            .create_invitation(&id, &req.email, &req.role, &claims.sub, 72)
            .await
        {
            Ok(inv) => {
                let base_url = std::env::var("BIZCLAW_BASE_URL")
                    .unwrap_or_else(|_| "https://app.bizclaw.vn".into());
                let invite_url = format!("{base_url}/api/admin/invitations/{}/accept", inv.token);
                Json(serde_json::json!({
                    "ok": true,
                    "invitation_token": inv.token,
                    "invitation_url": invite_url,
                    "expires_in_hours": 72,
                }))
            }
            Err(e) => internal_error("invite_member", e),
        },
        None => Json(serde_json::json!({"ok": false, "error": "PostgreSQL required"})),
    }
}

#[derive(serde::Deserialize)]
struct UpdateMemberRoleReq {
    role: String,
}

async fn update_member_role(
    State(state): State<Arc<AdminState>>,
    Extension(claims): Extension<crate::auth::Claims>,
    Path((id, uid)): Path<(String, String)>,
    Json(req): Json<UpdateMemberRoleReq>,
) -> Json<serde_json::Value> {
    if !can_write_tenant(&claims, &id, &*state.db.lock().await) {
        return Json(serde_json::json!({"ok": false, "error": "Không có quyền."}));
    }
    match &state.pg_db {
        Some(pg) => match pg.add_tenant_member(&id, &uid, &req.role, None).await {
            Ok(()) => Json(serde_json::json!({"ok": true})),
            Err(e) => internal_error("update_member_role", e),
        },
        None => Json(serde_json::json!({"ok": false, "error": "PostgreSQL required"})),
    }
}

async fn remove_member(
    State(state): State<Arc<AdminState>>,
    Extension(claims): Extension<crate::auth::Claims>,
    Path((id, uid)): Path<(String, String)>,
) -> Json<serde_json::Value> {
    if !can_write_tenant(&claims, &id, &*state.db.lock().await) {
        return Json(serde_json::json!({"ok": false, "error": "Không có quyền."}));
    }
    match &state.pg_db {
        Some(pg) => match pg.remove_tenant_member(&id, &uid).await {
            Ok(()) => Json(serde_json::json!({"ok": true})),
            Err(e) => internal_error("remove_member", e),
        },
        None => Json(serde_json::json!({"ok": false, "error": "PostgreSQL required"})),
    }
}

#[derive(serde::Deserialize)]
struct AcceptInvitationReq {
    user_id: String,
}

async fn accept_invitation(
    State(state): State<Arc<AdminState>>,
    Path(token): Path<String>,
    Json(req): Json<AcceptInvitationReq>,
) -> Json<serde_json::Value> {
    match &state.pg_db {
        Some(pg) => match pg.accept_invitation(&token, &req.user_id).await {
            Ok((tenant_id, role)) => Json(serde_json::json!({
                "ok": true, "tenant_id": tenant_id, "role": role,
            })),
            Err(e) => internal_error("accept_invitation", e),
        },
        None => Json(serde_json::json!({"ok": false, "error": "PostgreSQL required"})),
    }
}

// ════════════════════════════════════════════════════════════════════
// ENTERPRISE HANDLERS: Human Handoff
// ════════════════════════════════════════════════════════════════════

async fn list_handoffs(
    State(state): State<Arc<AdminState>>,
    Extension(claims): Extension<crate::auth::Claims>,
    Path(id): Path<String>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<serde_json::Value> {
    if !can_access_tenant(&claims, &id, &*state.db.lock().await) {
        return Json(serde_json::json!({"ok": false, "error": "Không có quyền."}));
    }
    let status = params.get("status").map(|s| s.as_str());
    match &state.pg_db {
        Some(pg) => match pg.list_handoffs(&id, status).await {
            Ok(handoffs) => Json(serde_json::json!({"ok": true, "handoffs": handoffs})),
            Err(e) => internal_error("list_handoffs", e),
        },
        None => Json(serde_json::json!({"ok": false, "error": "PostgreSQL required"})),
    }
}

async fn claim_handoff(
    State(state): State<Arc<AdminState>>,
    Extension(claims): Extension<crate::auth::Claims>,
    Path((_id, hid)): Path<(String, String)>,
) -> Json<serde_json::Value> {
    match &state.pg_db {
        Some(pg) => match pg.claim_handoff(&hid, &claims.sub).await {
            Ok(()) => Json(serde_json::json!({"ok": true, "message": "Bạn đã nhận case này"})),
            Err(e) => internal_error("claim_handoff", e),
        },
        None => Json(serde_json::json!({"ok": false, "error": "PostgreSQL required"})),
    }
}

#[derive(serde::Deserialize)]
struct HandoffReplyReq {
    content: String,
}

async fn reply_handoff(
    State(state): State<Arc<AdminState>>,
    Extension(claims): Extension<crate::auth::Claims>,
    Path((_id, hid)): Path<(String, String)>,
    Json(req): Json<HandoffReplyReq>,
) -> Json<serde_json::Value> {
    match &state.pg_db {
        Some(pg) => match pg
            .add_handoff_message(&hid, "operator", Some(&claims.sub), &req.content)
            .await
        {
            Ok(msg) => Json(serde_json::json!({"ok": true, "message": msg})),
            Err(e) => internal_error("reply_handoff", e),
        },
        None => Json(serde_json::json!({"ok": false, "error": "PostgreSQL required"})),
    }
}

async fn resolve_handoff(
    State(state): State<Arc<AdminState>>,
    Extension(claims): Extension<crate::auth::Claims>,
    Path((_id, hid)): Path<(String, String)>,
) -> Json<serde_json::Value> {
    match &state.pg_db {
        Some(pg) => match pg.resolve_handoff(&hid, &claims.sub).await {
            Ok(()) => Json(serde_json::json!({"ok": true, "message": "Case đã được đóng"})),
            Err(e) => internal_error("resolve_handoff", e),
        },
        None => Json(serde_json::json!({"ok": false, "error": "PostgreSQL required"})),
    }
}

async fn list_handoff_messages(
    State(state): State<Arc<AdminState>>,
    Extension(claims): Extension<crate::auth::Claims>,
    Path((id, hid)): Path<(String, String)>,
) -> Json<serde_json::Value> {
    if !can_access_tenant(&claims, &id, &*state.db.lock().await) {
        return Json(serde_json::json!({"ok": false, "error": "Không có quyền."}));
    }
    match &state.pg_db {
        Some(pg) => match pg.list_handoff_messages(&hid).await {
            Ok(msgs) => Json(serde_json::json!({"ok": true, "messages": msgs})),
            Err(e) => internal_error("list_handoff_messages", e),
        },
        None => Json(serde_json::json!({"ok": false, "error": "PostgreSQL required"})),
    }
}

// ════════════════════════════════════════════════════════════════════
// ENTERPRISE HANDLERS: BI Analytics
// ════════════════════════════════════════════════════════════════════

async fn analytics_summary(
    State(state): State<Arc<AdminState>>,
    Extension(claims): Extension<crate::auth::Claims>,
    Path(id): Path<String>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<serde_json::Value> {
    if !can_access_tenant(&claims, &id, &*state.db.lock().await) {
        return Json(serde_json::json!({"ok": false, "error": "Không có quyền."}));
    }
    let days: i32 = params
        .get("days")
        .and_then(|d| d.parse().ok())
        .unwrap_or(30);
    match &state.pg_db {
        Some(pg) => match pg.get_analytics_summary(&id, days).await {
            Ok(summary) => {
                Json(serde_json::json!({"ok": true, "period_days": days, "summary": summary}))
            }
            Err(e) => internal_error("analytics_summary", e),
        },
        None => Json(serde_json::json!({"ok": false, "error": "PostgreSQL required"})),
    }
}

async fn analytics_tokens(
    State(state): State<Arc<AdminState>>,
    Extension(claims): Extension<crate::auth::Claims>,
    Path(id): Path<String>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<serde_json::Value> {
    if !can_access_tenant(&claims, &id, &*state.db.lock().await) {
        return Json(serde_json::json!({"ok": false, "error": "Không có quyền."}));
    }
    let days: i32 = params
        .get("days")
        .and_then(|d| d.parse().ok())
        .unwrap_or(30);
    match &state.pg_db {
        Some(pg) => match pg.get_token_usage_by_day(&id, days).await {
            Ok(usage) => Json(serde_json::json!({"ok": true, "period_days": days, "usage": usage})),
            Err(e) => internal_error("analytics_tokens", e),
        },
        None => Json(serde_json::json!({"ok": false, "error": "PostgreSQL required"})),
    }
}

// ════════════════════════════════════════════════════════════════════
// ENTERPRISE HANDLERS: Budget Quota
// ════════════════════════════════════════════════════════════════════

async fn list_quotas(
    State(state): State<Arc<AdminState>>,
    Extension(claims): Extension<crate::auth::Claims>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    if !can_access_tenant(&claims, &id, &*state.db.lock().await) {
        return Json(serde_json::json!({"ok": false, "error": "Không có quyền."}));
    }
    match &state.pg_db {
        Some(pg) => match pg.get_quota_status(&id).await {
            Ok(quotas) => Json(serde_json::json!({"ok": true, "quotas": quotas})),
            Err(e) => internal_error("list_quotas", e),
        },
        None => Json(serde_json::json!({"ok": false, "error": "PostgreSQL required"})),
    }
}

#[derive(serde::Deserialize)]
struct SetQuotaReq {
    limit_value: i64,
}

async fn set_quota(
    State(state): State<Arc<AdminState>>,
    Extension(claims): Extension<crate::auth::Claims>,
    Path((id, resource)): Path<(String, String)>,
    Json(req): Json<SetQuotaReq>,
) -> Json<serde_json::Value> {
    if !is_super_admin(&claims) && !can_write_tenant(&claims, &id, &*state.db.lock().await) {
        return Json(
            serde_json::json!({"ok": false, "error": "Chỉ Admin mới có thể thay đổi quota."}),
        );
    }
    let valid = ["tokens_per_month", "messages_per_day", "handoffs_per_month"];
    if !valid.contains(&resource.as_str()) {
        return Json(serde_json::json!({"ok": false, "error": "Resource không hợp lệ."}));
    }
    match &state.pg_db {
        Some(pg) => match pg.set_quota(&id, &resource, req.limit_value).await {
            Ok(()) => Json(
                serde_json::json!({"ok": true, "resource": resource, "limit": req.limit_value}),
            ),
            Err(e) => internal_error("set_quota", e),
        },
        None => Json(serde_json::json!({"ok": false, "error": "PostgreSQL required"})),
    }
}

// ════════════════════════════════════════════════════════════════════
// MISSION CONTROL HANDLERS: Kanban Task Board
// ════════════════════════════════════════════════════════════════════

async fn mc_list_tasks(
    State(state): State<Arc<AdminState>>,
    Extension(_claims): Extension<crate::auth::Claims>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<serde_json::Value> {
    match &state.pg_db {
        Some(pg) => {
            let tenant_id = params.get("tenant_id").map(|s| s.as_str());
            let status = params.get("status").map(|s| s.as_str());
            let priority = params.get("priority").map(|s| s.as_str());
            let assigned = params.get("assigned_to").map(|s| s.as_str());
            let limit: i64 = params
                .get("limit")
                .and_then(|v| v.parse().ok())
                .unwrap_or(100);
            match pg
                .list_tasks(tenant_id, status, priority, assigned, limit)
                .await
            {
                Ok(tasks) => Json(serde_json::json!({ "ok": true, "tasks": tasks })),
                Err(e) => internal_error("mc_list_tasks", e),
            }
        }
        None => Json(serde_json::json!({ "ok": false, "error": "PostgreSQL required" })),
    }
}

#[derive(serde::Deserialize)]
struct McCreateTaskReq {
    title: String,
    description: Option<String>,
    status: Option<String>,
    priority: Option<String>,
    assigned_to: Option<String>,
    assigned_agent: Option<String>,
    tags: Option<String>,
    due_at: Option<String>,
    quality_gate: Option<bool>,
    tenant_id: Option<String>,
}

async fn mc_create_task(
    State(state): State<Arc<AdminState>>,
    Extension(claims): Extension<crate::auth::Claims>,
    Json(req): Json<McCreateTaskReq>,
) -> Json<serde_json::Value> {
    match &state.pg_db {
        Some(pg) => {
            let create_req = crate::mission_control::CreateTaskReq {
                title: req.title,
                description: req.description,
                status: req.status,
                priority: req.priority,
                assigned_to: req.assigned_to,
                assigned_agent: req.assigned_agent,
                tags: req.tags,
                due_at: req.due_at,
                quality_gate: req.quality_gate,
                tenant_id: req.tenant_id,
            };
            match pg.create_task(&create_req, &claims.sub).await {
                Ok(task) => Json(serde_json::json!({ "ok": true, "task": task })),
                Err(e) => internal_error("mc_create_task", e),
            }
        }
        None => Json(serde_json::json!({ "ok": false, "error": "PostgreSQL required" })),
    }
}

async fn mc_kanban_board(
    State(state): State<Arc<AdminState>>,
    Extension(_claims): Extension<crate::auth::Claims>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<serde_json::Value> {
    match &state.pg_db {
        Some(pg) => {
            let tenant_id = params.get("tenant_id").map(|s| s.as_str());
            match pg.get_kanban_board(tenant_id).await {
                Ok(board) => Json(serde_json::json!({ "ok": true, "board": board })),
                Err(e) => internal_error("mc_kanban_board", e),
            }
        }
        None => Json(serde_json::json!({ "ok": false, "error": "PostgreSQL required" })),
    }
}

async fn mc_pending_reviews(
    State(state): State<Arc<AdminState>>,
    Extension(_claims): Extension<crate::auth::Claims>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<serde_json::Value> {
    match &state.pg_db {
        Some(pg) => {
            let tenant_id = params.get("tenant_id").map(|s| s.as_str());
            match pg.get_pending_reviews(tenant_id).await {
                Ok(tasks) => {
                    Json(serde_json::json!({ "ok": true, "tasks": tasks, "count": tasks.len() }))
                }
                Err(e) => internal_error("mc_pending_reviews", e),
            }
        }
        None => Json(serde_json::json!({ "ok": false, "error": "PostgreSQL required" })),
    }
}

async fn mc_get_task(
    State(state): State<Arc<AdminState>>,
    Path(tid): Path<String>,
) -> Json<serde_json::Value> {
    match &state.pg_db {
        Some(pg) => match pg.get_task(&tid).await {
            Ok(task) => Json(serde_json::json!({ "ok": true, "task": task })),
            Err(e) => internal_error("mc_get_task", e),
        },
        None => Json(serde_json::json!({ "ok": false, "error": "PostgreSQL required" })),
    }
}

#[derive(serde::Deserialize)]
struct McUpdateTaskReq {
    title: Option<String>,
    description: Option<String>,
    status: Option<String>,
    priority: Option<String>,
    assigned_to: Option<String>,
    assigned_agent: Option<String>,
    position: Option<i32>,
}

async fn mc_update_task(
    State(state): State<Arc<AdminState>>,
    Path(tid): Path<String>,
    Json(req): Json<McUpdateTaskReq>,
) -> Json<serde_json::Value> {
    match &state.pg_db {
        Some(pg) => match pg
            .update_task(
                &tid,
                req.title.as_deref(),
                req.description.as_deref(),
                req.status.as_deref(),
                req.priority.as_deref(),
                req.assigned_to.as_deref(),
                req.assigned_agent.as_deref(),
                req.position,
            )
            .await
        {
            Ok(task) => Json(serde_json::json!({ "ok": true, "task": task })),
            Err(e) => Json(serde_json::json!({ "ok": false, "error": e.to_string() })),
        },
        None => Json(serde_json::json!({ "ok": false, "error": "PostgreSQL required" })),
    }
}

async fn mc_delete_task(
    State(state): State<Arc<AdminState>>,
    Path(tid): Path<String>,
) -> Json<serde_json::Value> {
    match &state.pg_db {
        Some(pg) => match pg.delete_task(&tid).await {
            Ok(()) => Json(serde_json::json!({ "ok": true })),
            Err(e) => internal_error("mc_delete_task", e),
        },
        None => Json(serde_json::json!({ "ok": false, "error": "PostgreSQL required" })),
    }
}

async fn mc_list_comments(
    State(state): State<Arc<AdminState>>,
    Path(tid): Path<String>,
) -> Json<serde_json::Value> {
    match &state.pg_db {
        Some(pg) => match pg.list_task_comments(&tid).await {
            Ok(comments) => Json(serde_json::json!({ "ok": true, "comments": comments })),
            Err(e) => internal_error("mc_list_comments", e),
        },
        None => Json(serde_json::json!({ "ok": false, "error": "PostgreSQL required" })),
    }
}

#[derive(serde::Deserialize)]
struct McCommentReq {
    content: String,
}

async fn mc_add_comment(
    State(state): State<Arc<AdminState>>,
    Extension(claims): Extension<crate::auth::Claims>,
    Path(tid): Path<String>,
    Json(req): Json<McCommentReq>,
) -> Json<serde_json::Value> {
    match &state.pg_db {
        Some(pg) => match pg
            .add_task_comment(&tid, Some(&claims.sub), &claims.email, &req.content)
            .await
        {
            Ok(c) => Json(serde_json::json!({ "ok": true, "comment": c })),
            Err(e) => internal_error("mc_add_comment", e),
        },
        None => Json(serde_json::json!({ "ok": false, "error": "PostgreSQL required" })),
    }
}

#[derive(serde::Deserialize)]
struct McReviewReq {
    status: String, // approved | rejected
    notes: Option<String>,
}

async fn mc_submit_review(
    State(state): State<Arc<AdminState>>,
    Extension(claims): Extension<crate::auth::Claims>,
    Path(tid): Path<String>,
    Json(req): Json<McReviewReq>,
) -> Json<serde_json::Value> {
    let valid = ["approved", "rejected"];
    if !valid.contains(&req.status.as_str()) {
        return Json(
            serde_json::json!({ "ok": false, "error": "status phải là: approved | rejected" }),
        );
    }
    match &state.pg_db {
        Some(pg) => match pg
            .submit_quality_review(
                &tid,
                Some(&claims.sub),
                &claims.email,
                &req.status,
                req.notes.as_deref(),
            )
            .await
        {
            Ok(review) => Json(serde_json::json!({ "ok": true, "review": review })),
            Err(e) => internal_error("mc_submit_review", e),
        },
        None => Json(serde_json::json!({ "ok": false, "error": "PostgreSQL required" })),
    }
}

// ════════════════════════════════════════════════════════════════════
// MISSION CONTROL HANDLERS: Agent Session Monitor
// ════════════════════════════════════════════════════════════════════

async fn mc_list_sessions(
    State(state): State<Arc<AdminState>>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<serde_json::Value> {
    match &state.pg_db {
        Some(pg) => {
            let tenant_id = params.get("tenant_id").map(|s| s.as_str());
            match pg.list_agent_sessions(tenant_id).await {
                Ok(sessions) => Json(serde_json::json!({ "ok": true, "sessions": sessions })),
                Err(e) => internal_error("mc_list_sessions", e),
            }
        }
        None => Json(serde_json::json!({ "ok": false, "error": "PostgreSQL required" })),
    }
}

#[derive(serde::Deserialize)]
struct McHeartbeatReq {
    agent_name: String,
    session_key: String,
    tenant_id: Option<String>,
    prompt_tokens: Option<i64>,
    completion_tokens: Option<i64>,
    cost_usd: Option<f32>,
    model: Option<String>,
}

async fn mc_heartbeat(
    State(state): State<Arc<AdminState>>,
    Json(req): Json<McHeartbeatReq>,
) -> Json<serde_json::Value> {
    match &state.pg_db {
        Some(pg) => match pg
            .upsert_agent_session(
                req.tenant_id.as_deref(),
                &req.agent_name,
                &req.session_key,
                req.prompt_tokens.unwrap_or(0),
                req.completion_tokens.unwrap_or(0),
                req.cost_usd.unwrap_or(0.0),
                req.model.as_deref(),
            )
            .await
        {
            Ok(session) => Json(serde_json::json!({ "ok": true, "session": session })),
            Err(e) => internal_error("mc_heartbeat", e),
        },
        None => Json(serde_json::json!({ "ok": false, "error": "PostgreSQL required" })),
    }
}

async fn mc_terminate_session(
    State(state): State<Arc<AdminState>>,
    Path(key): Path<String>,
) -> Json<serde_json::Value> {
    match &state.pg_db {
        Some(pg) => match pg.terminate_session(&key).await {
            Ok(()) => Json(serde_json::json!({ "ok": true })),
            Err(e) => internal_error("mc_terminate_session", e),
        },
        None => Json(serde_json::json!({ "ok": false, "error": "PostgreSQL required" })),
    }
}

// ════════════════════════════════════════════════════════════════════
// MISSION CONTROL HANDLERS: GitHub Sync
// ════════════════════════════════════════════════════════════════════

async fn mc_list_github_syncs(
    State(state): State<Arc<AdminState>>,
    Extension(claims): Extension<crate::auth::Claims>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    if !can_access_tenant(&claims, &id, &*state.db.lock().await) {
        return Json(serde_json::json!({ "ok": false, "error": "Không có quyền." }));
    }
    match &state.pg_db {
        Some(pg) => match pg.list_github_syncs(&id).await {
            Ok(syncs) => Json(serde_json::json!({ "ok": true, "syncs": syncs })),
            Err(e) => internal_error("mc_list_github_syncs", e),
        },
        None => Json(serde_json::json!({ "ok": false, "error": "PostgreSQL required" })),
    }
}

#[derive(serde::Deserialize)]
struct McGithubSyncReq {
    repo: String,
    access_token: Option<String>,
    label_filter: Option<String>,
    auto_assign: Option<String>,
}

async fn mc_upsert_github_sync(
    State(state): State<Arc<AdminState>>,
    Extension(claims): Extension<crate::auth::Claims>,
    Path(id): Path<String>,
    Json(req): Json<McGithubSyncReq>,
) -> Json<serde_json::Value> {
    if !can_write_tenant(&claims, &id, &*state.db.lock().await) {
        return Json(serde_json::json!({ "ok": false, "error": "Không có quyền." }));
    }
    match &state.pg_db {
        Some(pg) => match pg
            .upsert_github_sync(
                &id,
                &req.repo,
                req.access_token.as_deref(),
                req.label_filter.as_deref().unwrap_or(""),
                req.auto_assign.as_deref().unwrap_or(""),
            )
            .await
        {
            Ok(sync) => Json(serde_json::json!({ "ok": true, "sync": sync })),
            Err(e) => internal_error("mc_upsert_github_sync", e),
        },
        None => Json(serde_json::json!({ "ok": false, "error": "PostgreSQL required" })),
    }
}

async fn mc_trigger_github_sync(
    State(state): State<Arc<AdminState>>,
    Extension(claims): Extension<crate::auth::Claims>,
    Path((id, repo)): Path<(String, String)>,
) -> Json<serde_json::Value> {
    if !can_access_tenant(&claims, &id, &*state.db.lock().await) {
        return Json(serde_json::json!({ "ok": false, "error": "Không có quyền." }));
    }
    // repo path param may be URL-encoded "owner/repo" → decode
    let repo_decoded = repo.replace("%2F", "/");
    match &state.pg_db {
        Some(pg) => match pg.sync_github_issues(&id, &repo_decoded).await {
            Ok(count) => Json(serde_json::json!({
                "ok": true,
                "synced": count,
                "message": format!("Đã sync {count} GitHub issues vào Task Board")
            })),
            Err(e) => internal_error("mc_trigger_github_sync", e),
        },
        None => Json(serde_json::json!({ "ok": false, "error": "PostgreSQL required" })),
    }
}

// ════════════════════════════════════════════════════════════════════
// MISSION CONTROL HANDLERS: Webhooks
// ════════════════════════════════════════════════════════════════════

async fn mc_list_webhooks(
    State(state): State<Arc<AdminState>>,
    Extension(claims): Extension<crate::auth::Claims>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    if !can_access_tenant(&claims, &id, &*state.db.lock().await) {
        return Json(serde_json::json!({ "ok": false, "error": "Không có quyền." }));
    }
    match &state.pg_db {
        Some(pg) => match pg.list_webhooks(&id).await {
            Ok(webhooks) => Json(serde_json::json!({ "ok": true, "webhooks": webhooks })),
            Err(e) => internal_error("mc_list_webhooks", e),
        },
        None => Json(serde_json::json!({ "ok": false, "error": "PostgreSQL required" })),
    }
}

// ════════════════════════════════════════════════════════════════
// CLOUD: Remote Server Provisioner Handlers
// ════════════════════════════════════════════════════════════════

async fn srv_list_servers(
    State(state): State<Arc<AdminState>>,
    Extension(claims): Extension<crate::auth::Claims>,
) -> Json<serde_json::Value> {
    if !is_super_admin(&claims) {
        return Json(serde_json::json!({ "ok": false, "error": "Superadmin required" }));
    }
    match &state.pg_db {
        Some(pg) => match pg.list_servers().await {
            Ok(servers) => Json(serde_json::json!({ "ok": true, "servers": servers })),
            Err(e) => internal_error("srv_list_servers", e),
        },
        None => Json(serde_json::json!({ "ok": false, "error": "PostgreSQL required" })),
    }
}

async fn srv_provision(
    State(state): State<Arc<AdminState>>,
    Extension(claims): Extension<crate::auth::Claims>,
    Json(req): Json<crate::server_provisioner::ProvisionRequest>,
) -> Json<serde_json::Value> {
    if !is_super_admin(&claims) {
        return Json(serde_json::json!({ "ok": false, "error": "Superadmin required" }));
    }
    let pg = match &state.pg_db {
        Some(pg) => pg,
        None => return Json(serde_json::json!({ "ok": false, "error": "PostgreSQL required" })),
    };

    // Register server in DB
    let port = req.port.unwrap_or(3001);
    match pg
        .register_server(&req.name, &req.ip, req.domain.as_deref(), port)
        .await
    {
        Ok(server) => {
            let server_id = server.id.clone();
            let pg_clone = pg.clone();

            // Provision asynchronously
            tokio::spawn(async move {
                match crate::server_provisioner::provision_server(&req).await {
                    Ok(result) => {
                        let status = if result.success { "online" } else { "error" };
                        let _ = pg_clone
                            .update_server_health(&server_id, status, None, 0, None, None, None)
                            .await;
                        tracing::info!("Provision completed for {}: {}", server_id, result.message);
                    }
                    Err(e) => {
                        let _ = pg_clone
                            .update_server_health(&server_id, "error", None, 0, None, None, None)
                            .await;
                        tracing::error!("Provision failed for {}: {}", server_id, e);
                    }
                }
            });

            Json(serde_json::json!({
                "ok": true,
                "server": server,
                "message": "Provisioning started — check status in a few minutes",
            }))
        }
        Err(e) => internal_error("srv_provision", e),
    }
}

async fn srv_get_server(
    State(state): State<Arc<AdminState>>,
    Extension(claims): Extension<crate::auth::Claims>,
    Path(sid): Path<String>,
) -> Json<serde_json::Value> {
    if !is_super_admin(&claims) {
        return Json(serde_json::json!({ "ok": false, "error": "Superadmin required" }));
    }
    match &state.pg_db {
        Some(pg) => match pg.list_servers().await {
            Ok(servers) => match servers.into_iter().find(|s| s.id == sid) {
                Some(server) => Json(serde_json::json!({ "ok": true, "server": server })),
                None => Json(serde_json::json!({ "ok": false, "error": "Server not found" })),
            },
            Err(e) => internal_error("srv_get_server", e),
        },
        None => Json(serde_json::json!({ "ok": false, "error": "PostgreSQL required" })),
    }
}

async fn srv_delete_server(
    State(state): State<Arc<AdminState>>,
    Extension(claims): Extension<crate::auth::Claims>,
    Path(sid): Path<String>,
) -> Json<serde_json::Value> {
    if !is_super_admin(&claims) {
        return Json(serde_json::json!({ "ok": false, "error": "Superadmin required" }));
    }
    match &state.pg_db {
        Some(pg) => match pg.delete_server(&sid).await {
            Ok(_) => Json(serde_json::json!({ "ok": true, "deleted": true })),
            Err(e) => internal_error("srv_delete_server", e),
        },
        None => Json(serde_json::json!({ "ok": false, "error": "PostgreSQL required" })),
    }
}

async fn srv_health_check(
    State(state): State<Arc<AdminState>>,
    Extension(claims): Extension<crate::auth::Claims>,
    Path(sid): Path<String>,
) -> Json<serde_json::Value> {
    if !is_super_admin(&claims) {
        return Json(serde_json::json!({ "ok": false, "error": "Superadmin required" }));
    }
    let pg = match &state.pg_db {
        Some(pg) => pg,
        None => return Json(serde_json::json!({ "ok": false, "error": "PostgreSQL required" })),
    };

    let servers = pg.list_servers().await.unwrap_or_default();
    match servers.into_iter().find(|s| s.id == sid) {
        Some(server) => {
            let health = crate::server_provisioner::health_check(&server.ip, server.port).await;
            let status = if health.online { "online" } else { "offline" };
            let _ = pg
                .update_server_health(
                    &sid,
                    status,
                    health.version.as_deref(),
                    health.tenants,
                    health.cpu,
                    health.ram,
                    health.disk,
                )
                .await;
            Json(serde_json::json!({ "ok": true, "health": health }))
        }
        None => Json(serde_json::json!({ "ok": false, "error": "Server not found" })),
    }
}

#[derive(serde::Deserialize)]
struct SrvCommandReq {
    action: String,
}

async fn srv_execute_command(
    State(state): State<Arc<AdminState>>,
    Extension(claims): Extension<crate::auth::Claims>,
    Path(sid): Path<String>,
    Json(cmd): Json<SrvCommandReq>,
) -> Json<serde_json::Value> {
    if !is_super_admin(&claims) {
        return Json(serde_json::json!({ "ok": false, "error": "Superadmin required" }));
    }
    let pg = match &state.pg_db {
        Some(pg) => pg,
        None => return Json(serde_json::json!({ "ok": false, "error": "PostgreSQL required" })),
    };

    let servers = pg.list_servers().await.unwrap_or_default();
    match servers.into_iter().find(|s| s.id == sid) {
        Some(server) => {
            let remote_cmd = match cmd.action.as_str() {
                "restart" => "systemctl restart bizclaw",
                "stop" => "systemctl stop bizclaw",
                "start" => "systemctl start bizclaw",
                "status" => "systemctl status bizclaw --no-pager",
                "update" => {
                    "cd /opt/bizclaw && git pull origin master && cargo build --release && systemctl restart bizclaw"
                }
                "logs" => "journalctl -u bizclaw -n 100 --no-pager",
                "version" => {
                    "/opt/bizclaw/bizclaw-platform --version 2>/dev/null || echo 'unknown'"
                }
                _ => {
                    return Json(serde_json::json!({
                        "ok": false,
                        "error": format!("Unknown action: {}. Use: restart, stop, start, status, update, logs, version", cmd.action),
                    }));
                }
            };

            match crate::server_provisioner::remote_exec(&server.ip, remote_cmd).await {
                Ok(output) => Json(serde_json::json!({ "ok": true, "output": output })),
                Err(e) => Json(serde_json::json!({ "ok": false, "error": e.to_string() })),
            }
        }
        None => Json(serde_json::json!({ "ok": false, "error": "Server not found" })),
    }
}
