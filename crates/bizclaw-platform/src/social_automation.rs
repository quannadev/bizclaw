//! Social Automation — Agentic AI pipeline for automatic social media management.
//!
//! 3 core capabilities:
//! 1. **Content Pipeline**: Crawl URLs → AI tổng hợp → Auto-post to FB/IG/Zalo
//! 2. **Comment Auto-Reply**: Webhook nhận comment → AI trả lời dựa trên RAG
//! 3. **Engagement Monitor**: Track interactions, sentiment, respond proactively
//!
//! Designed for SME Đà Lạt verticals: Tourism, F&B, Specialty Products.

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::admin::AdminState;

// ══════════════════════════════════════════════════════════
// 1. FACEBOOK COMMENT WEBHOOK
// ══════════════════════════════════════════════════════════

/// Facebook Webhook verification (GET) — required during setup.
/// Facebook sends: hub.mode=subscribe&hub.verify_token=TOKEN&hub.challenge=CHALLENGE
#[derive(Debug, Deserialize)]
pub struct FbVerifyParams {
    #[serde(rename = "hub.mode")]
    pub mode: Option<String>,
    #[serde(rename = "hub.verify_token")]
    pub verify_token: Option<String>,
    #[serde(rename = "hub.challenge")]
    pub challenge: Option<String>,
}

pub async fn fb_webhook_verify(Query(params): Query<FbVerifyParams>) -> impl IntoResponse {
    let expected_token =
        std::env::var("BIZCLAW_FB_VERIFY_TOKEN").unwrap_or_else(|_| "bizclaw_verify".into());

    if params.mode.as_deref() == Some("subscribe")
        && params.verify_token.as_deref() == Some(&expected_token)
    {
        tracing::info!("✅ Facebook webhook verified");
        (StatusCode::OK, params.challenge.unwrap_or_default())
    } else {
        tracing::warn!("❌ Facebook webhook verification failed");
        (StatusCode::FORBIDDEN, "Verification failed".into())
    }
}

/// Facebook Webhook payload (POST) — receives page events.
#[derive(Debug, Deserialize)]
pub struct FbWebhookPayload {
    pub object: Option<String>,
    pub entry: Option<Vec<FbEntry>>,
}

#[derive(Debug, Deserialize)]
pub struct FbEntry {
    pub id: Option<String>,
    pub time: Option<u64>,
    pub changes: Option<Vec<FbChange>>,
    /// Messaging events (Messenger)
    pub messaging: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Deserialize)]
pub struct FbChange {
    pub field: Option<String>,
    pub value: Option<serde_json::Value>,
}

/// Comment extracted from webhook payload.
#[derive(Debug, Clone, Serialize)]
pub struct IncomingComment {
    pub comment_id: String,
    pub post_id: String,
    pub page_id: String,
    pub sender_name: String,
    pub sender_id: String,
    pub message: String,
    pub created_time: String,
    pub platform: String, // "facebook" or "instagram"
}

/// POST /api/social/fb-webhook — receive Facebook page events.
pub async fn fb_webhook_handler(
    State(state): State<Arc<AdminState>>,
    Json(payload): Json<FbWebhookPayload>,
) -> StatusCode {
    // Facebook requires 200 response within 20 seconds
    let object = payload.object.as_deref().unwrap_or("");

    if object != "page" && object != "instagram" {
        tracing::debug!("FB webhook: ignoring object type '{}'", object);
        return StatusCode::OK;
    }

    let platform = if object == "instagram" {
        "instagram"
    } else {
        "facebook"
    };

    // Process entries
    if let Some(entries) = payload.entry {
        for entry in entries {
            let page_id = entry.id.unwrap_or_default();

            // Process feed changes (comments, posts)
            if let Some(changes) = entry.changes {
                for change in changes {
                    let field = change.field.as_deref().unwrap_or("");

                    match field {
                        "feed" | "comments" => {
                            if let Some(value) = change.value {
                                process_comment_event(
                                    &state,
                                    &page_id,
                                    &value,
                                    platform,
                                )
                                .await;
                            }
                        }
                        "messages" | "messaging" => {
                            if let Some(value) = change.value {
                                tracing::info!(
                                    "📨 {} message received on page {}: {}",
                                    platform,
                                    page_id,
                                    serde_json::to_string(&value)
                                        .unwrap_or_default()
                                        .chars()
                                        .take(200)
                                        .collect::<String>()
                                );
                            }
                        }
                        _ => {
                            tracing::debug!("FB webhook: ignoring field '{}'", field);
                        }
                    }
                }
            }

            // Process messenger events
            if let Some(messaging) = entry.messaging {
                for msg in messaging {
                    tracing::info!(
                        "📨 Messenger event on page {}: {}",
                        page_id,
                        serde_json::to_string(&msg)
                            .unwrap_or_default()
                            .chars()
                            .take(200)
                            .collect::<String>()
                    );
                }
            }
        }
    }

    StatusCode::OK
}

/// Process a comment event from Facebook/Instagram.
async fn process_comment_event(
    state: &AdminState,
    page_id: &str,
    value: &serde_json::Value,
    platform: &str,
) {
    let item = value["item"].as_str().unwrap_or("");
    let verb = value["verb"].as_str().unwrap_or("");

    // Only process new comments (not edits/deletes)
    if item != "comment" || verb != "add" {
        return;
    }

    let comment = IncomingComment {
        comment_id: value["comment_id"]
            .as_str()
            .unwrap_or_default()
            .to_string(),
        post_id: value["post_id"].as_str().unwrap_or_default().to_string(),
        page_id: page_id.to_string(),
        sender_name: value["from"]["name"].as_str().unwrap_or("").to_string(),
        sender_id: value["from"]["id"].as_str().unwrap_or("").to_string(),
        message: value["message"].as_str().unwrap_or("").to_string(),
        created_time: value["created_time"]
            .as_str()
            .unwrap_or("")
            .to_string(),
        platform: platform.to_string(),
    };

    // Skip if comment is from the page itself (avoid self-reply loops)
    if comment.sender_id == comment.page_id {
        tracing::debug!("Skipping self-comment on {} post {}", platform, comment.post_id);
        return;
    }

    tracing::info!(
        "💬 New {} comment from {} on post {}: \"{}\"",
        platform,
        comment.sender_name,
        comment.post_id,
        comment.message.chars().take(100).collect::<String>()
    );

    // Find tenant by page_id
    let tenant_id = match find_tenant_by_page_id(state, page_id).await {
        Some(id) => id,
        None => {
            tracing::warn!("No tenant found for {} page_id: {}", platform, page_id);
            return;
        }
    };

    // Generate AI reply
    let reply = generate_comment_reply(state, &tenant_id, &comment).await;

    if let Some(reply_text) = reply {
        // Post reply via Graph API
        post_comment_reply(state, &tenant_id, &comment, &reply_text).await;
    }
}

/// Find which tenant owns a Facebook/Instagram page.
async fn find_tenant_by_page_id(state: &AdminState, page_id: &str) -> Option<String> {
    let db = state.db.lock().await;
    // Search tenant_configs for oauth_facebook_tokens or oauth_instagram_tokens
    // that contain this page_id
    // For now, search by page_id stored in config
    let config_key = format!("social_page_{}", page_id);
    if let Ok(Some(tenant_id)) = db.get_config("__social_pages__", &config_key) {
        return Some(tenant_id);
    }

    // Fallback: check all tenants (expensive, should be cached)
    if let Ok(tenants) = db.list_tenants() {
        for tenant in tenants {
            let fb_page_key = "social_fb_page_id";
            if let Ok(Some(stored_page_id)) = db.get_config(&tenant.id, fb_page_key)
                && stored_page_id == page_id {
                    // Cache for future lookups
                    let _ = db.set_config("__social_pages__", &config_key, &tenant.id);
                    return Some(tenant.id.clone());
                }
        }
    }

    None
}

/// Generate an AI reply to a comment using tenant's context/RAG.
async fn generate_comment_reply(
    state: &AdminState,
    tenant_id: &str,
    comment: &IncomingComment,
) -> Option<String> {
    // Get tenant's LLM config
    let db = state.db.lock().await;

    // Get business context from tenant config
    let business_context = db
        .get_config(tenant_id, "business_context")
        .ok()
        .flatten()
        .unwrap_or_else(|| "Doanh nghiệp SME Việt Nam".into());

    let reply_style = db
        .get_config(tenant_id, "reply_style")
        .ok()
        .flatten()
        .unwrap_or_else(|| "Thân thiện, chuyên nghiệp, ngắn gọn. Luôn dùng kính ngữ (dạ, ạ). Kết thúc bằng CTA (gọi điện, inbox).".into());

    let auto_reply_enabled = db
        .get_config(tenant_id, "auto_reply_enabled")
        .ok()
        .flatten()
        .map(|v| v == "true")
        .unwrap_or(false);

    if !auto_reply_enabled {
        tracing::debug!(
            "Auto-reply disabled for tenant {}, skipping comment {}",
            tenant_id,
            comment.comment_id
        );
        return None;
    }

    // Build prompt for LLM
    let system_prompt = format!(
        "Bạn là trợ lý AI trả lời comment trên {} cho doanh nghiệp.\n\
         Thông tin doanh nghiệp: {}\n\
         Phong cách trả lời: {}\n\
         \n\
         QUY TẮC:\n\
         1. Trả lời ngắn gọn (1-3 câu)\n\
         2. Luôn thân thiện, dùng kính ngữ\n\
         3. Nếu khách hỏi giá/chi tiết → mời inbox hoặc gọi hotline\n\
         4. Nếu khách khen → cảm ơn + mời quay lại\n\
         5. Nếu khách phàn nàn → xin lỗi + hứa cải thiện + mời liên hệ trực tiếp\n\
         6. KHÔNG bao giờ bịa thông tin giá cả cụ thể\n\
         7. Trả lời bằng tiếng Việt",
        comment.platform, business_context, reply_style
    );

    let user_prompt = format!(
        "Người comment: {}\nNội dung comment: \"{}\"\n\nTrả lời comment này:",
        comment.sender_name, comment.message
    );

    // Get LLM API key (tenant BYO or global)
    let api_key = db
        .get_config(tenant_id, "llm_api_key")
        .ok()
        .flatten()
        .or_else(|| std::env::var("BIZCLAW_LLM_API_KEY").ok());

    let api_key = match api_key {
        Some(k) if !k.is_empty() => k,
        _ => {
            tracing::warn!("No LLM API key for tenant {} — cannot auto-reply", tenant_id);
            return None;
        }
    };

    drop(db); // Release lock before HTTP call

    // Call LLM (OpenAI-compatible)
    let http = reqwest::Client::new();

    let provider = std::env::var("BIZCLAW_LLM_PROVIDER").unwrap_or_else(|_| "anthropic".into());

    let reply = match provider.as_str() {
        "anthropic" => {
            call_anthropic(&http, &api_key, &system_prompt, &user_prompt).await
        }
        _ => {
            call_openai(&http, &api_key, &system_prompt, &user_prompt).await
        }
    };

    match reply {
        Ok(text) => {
            tracing::info!(
                "🤖 AI reply generated for comment on {}: \"{}\"",
                comment.platform,
                text.chars().take(100).collect::<String>()
            );
            Some(text)
        }
        Err(e) => {
            tracing::error!("LLM call failed for auto-reply: {e}");
            None
        }
    }
}

/// Post a reply to a Facebook/Instagram comment via Graph API.
async fn post_comment_reply(
    state: &AdminState,
    tenant_id: &str,
    comment: &IncomingComment,
    reply_text: &str,
) {
    let db = state.db.lock().await;

    // Get OAuth token for the platform
    let config_key = format!("oauth_{}_tokens", comment.platform);
    let tokens_json = match db.get_config(tenant_id, &config_key).ok().flatten() {
        Some(j) => j,
        None => {
            tracing::warn!(
                "No {} OAuth token for tenant {} — cannot reply",
                comment.platform,
                tenant_id
            );
            return;
        }
    };

    let tokens: crate::oauth::OAuthTokens = match serde_json::from_str(&tokens_json) {
        Ok(t) => t,
        Err(e) => {
            tracing::error!("Failed to parse OAuth tokens: {e}");
            return;
        }
    };

    drop(db);

    // Post reply via Graph API
    let http = reqwest::Client::new();
    let url = format!(
        "https://graph.facebook.com/v21.0/{}/comments",
        comment.comment_id
    );

    match http
        .post(&url)
        .form(&[
            ("message", reply_text),
            ("access_token", &tokens.access_token),
        ])
        .send()
        .await
    {
        Ok(resp) => {
            let status = resp.status();
            if status.is_success() {
                tracing::info!(
                    "✅ Auto-replied to {} comment {} on post {}",
                    comment.platform,
                    comment.comment_id,
                    comment.post_id
                );
            } else {
                let body = resp.text().await.unwrap_or_default();
                tracing::error!(
                    "❌ Failed to reply to comment: HTTP {} — {}",
                    status,
                    body.chars().take(200).collect::<String>()
                );
            }
        }
        Err(e) => {
            tracing::error!("❌ Comment reply request failed: {e}");
        }
    }
}

// ══════════════════════════════════════════════════════════
// 2. CONTENT PIPELINE — Crawl → AI → Auto-Post
// ══════════════════════════════════════════════════════════

/// Content Pipeline configuration per tenant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentPipelineConfig {
    /// URLs to crawl for content inspiration
    pub source_urls: Vec<String>,
    /// Business info for content generation
    pub business_context: String,
    /// Target platforms
    pub platforms: Vec<String>,
    /// Posting schedule (cron expressions or simple slots)
    pub schedule: Vec<String>,
    /// Content themes/topics
    pub topics: Vec<String>,
    /// Whether pipeline is active
    pub active: bool,
}

impl Default for ContentPipelineConfig {
    fn default() -> Self {
        Self {
            source_urls: vec![],
            business_context: String::new(),
            platforms: vec!["facebook".into()],
            schedule: vec!["08:00".into(), "12:00".into(), "18:00".into()],
            topics: vec![],
            active: false,
        }
    }
}

/// GET /api/social/pipeline/{tenant_id} — get pipeline config.
pub async fn get_pipeline_config(
    State(state): State<Arc<AdminState>>,
    axum::extract::Path(tenant_id): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    let db = state.db.lock().await;
    let config = db
        .get_config(&tenant_id, "content_pipeline")
        .ok()
        .flatten()
        .and_then(|json| serde_json::from_str::<ContentPipelineConfig>(&json).ok())
        .unwrap_or_default();

    Json(serde_json::json!({
        "tenant_id": tenant_id,
        "config": config,
    }))
}

/// POST /api/social/pipeline/{tenant_id} — update pipeline config.
pub async fn update_pipeline_config(
    State(state): State<Arc<AdminState>>,
    axum::extract::Path(tenant_id): axum::extract::Path<String>,
    Json(config): Json<ContentPipelineConfig>,
) -> Json<serde_json::Value> {
    let db = state.db.lock().await;
    let json = serde_json::to_string(&config).unwrap_or_default();

    match db.set_config(&tenant_id, "content_pipeline", &json) {
        Ok(_) => {
            tracing::info!("📋 Content pipeline updated for tenant {}", tenant_id);
            Json(serde_json::json!({
                "success": true,
                "message": "Pipeline config saved",
            }))
        }
        Err(e) => Json(serde_json::json!({
            "success": false,
            "error": format!("Failed to save: {e}"),
        })),
    }
}

/// POST /api/social/pipeline/{tenant_id}/trigger — manually trigger content generation.
pub async fn trigger_pipeline(
    State(state): State<Arc<AdminState>>,
    axum::extract::Path(tenant_id): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    let db = state.db.lock().await;

    let config = match db
        .get_config(&tenant_id, "content_pipeline")
        .ok()
        .flatten()
        .and_then(|json| serde_json::from_str::<ContentPipelineConfig>(&json).ok())
    {
        Some(c) => c,
        None => {
            return Json(serde_json::json!({
                "success": false,
                "error": "No pipeline config found. Configure first.",
            }));
        }
    };

    if config.source_urls.is_empty() {
        return Json(serde_json::json!({
            "success": false,
            "error": "No source URLs configured",
        }));
    }

    let api_key = db
        .get_config(&tenant_id, "llm_api_key")
        .ok()
        .flatten()
        .or_else(|| std::env::var("BIZCLAW_LLM_API_KEY").ok());

    drop(db);

    let api_key = match api_key {
        Some(k) if !k.is_empty() => k,
        _ => {
            return Json(serde_json::json!({
                "success": false,
                "error": "No LLM API key configured",
            }));
        }
    };

    // Fire-and-forget: run pipeline in background
    let state_clone = state.clone();
    let tenant_clone = tenant_id.clone();
    let url_count = config.source_urls.len();
    tokio::spawn(async move {
        run_content_pipeline(&state_clone, &tenant_clone, &config, &api_key).await;
    });

    Json(serde_json::json!({
        "success": true,
        "message": format!("Pipeline triggered for tenant {}. Crawling {} URLs...", tenant_id, url_count),
    }))
}

/// Run the full content pipeline: Crawl → AI → Post.
async fn run_content_pipeline(
    state: &AdminState,
    tenant_id: &str,
    config: &ContentPipelineConfig,
    api_key: &str,
) {
    tracing::info!(
        "🚀 Content pipeline started for tenant {} — {} URLs",
        tenant_id,
        config.source_urls.len()
    );

    // Step 1: Crawl source URLs
    let http = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .user_agent("BizClaw/1.0")
        .build()
        .unwrap_or_default();

    let mut crawled_content = Vec::new();

    for url in &config.source_urls {
        match http.get(url).send().await {
            Ok(resp) if resp.status().is_success() => {
                let text = resp.text().await.unwrap_or_default();
                // Extract meaningful text (strip HTML simply)
                let clean = strip_html(&text);
                let truncated: String = clean.chars().take(2000).collect();
                crawled_content.push(format!("📄 {url}:\n{truncated}"));
                tracing::info!("✅ Crawled {} ({} chars)", url, truncated.len());
            }
            Ok(resp) => {
                tracing::warn!("⚠️ Crawl {} failed: HTTP {}", url, resp.status());
            }
            Err(e) => {
                tracing::warn!("❌ Crawl {} error: {}", url, e);
            }
        }
    }

    if crawled_content.is_empty() {
        tracing::warn!("No content crawled — aborting pipeline for {}", tenant_id);
        return;
    }

    // Step 2: Generate social post via AI
    let topics = if config.topics.is_empty() {
        "general business content".to_string()
    } else {
        config.topics.join(", ")
    };

    let system_prompt = format!(
        "Bạn là chuyên gia content marketing cho doanh nghiệp SME Việt Nam.\n\
         Doanh nghiệp: {}\n\
         Chủ đề ưu tiên: {}\n\
         \n\
         NHIỆM VỤ: Viết 1 bài đăng Facebook/Instagram dựa trên nội dung crawl được.\n\
         \n\
         YÊU CẦU:\n\
         1. Bài ngắn gọn (100-200 từ)\n\
         2. Có emoji phù hợp\n\
         3. Có hashtag Việt Nam (5-8 tags)\n\
         4. CTA rõ ràng (inbox, gọi, đặt hàng)\n\
         5. Giọng văn thân thiện, gần gũi\n\
         6. KHÔNG bịa thông tin\n\
         7. Tạo cảm giác FOMO nhẹ nhàng",
        config.business_context, topics
    );

    let user_prompt = format!(
        "Dưới đây là nội dung crawl được từ các nguồn:\n\n{}\n\n\
         Hãy viết 1 bài đăng hấp dẫn cho Facebook/Instagram.",
        crawled_content.join("\n\n---\n\n")
    );

    let provider = std::env::var("BIZCLAW_LLM_PROVIDER").unwrap_or_else(|_| "anthropic".into());

    let generated = match provider.as_str() {
        "anthropic" => call_anthropic(&http, api_key, &system_prompt, &user_prompt).await,
        _ => call_openai(&http, api_key, &system_prompt, &user_prompt).await,
    };

    let post_content = match generated {
        Ok(text) => text,
        Err(e) => {
            tracing::error!("❌ AI content generation failed: {e}");
            return;
        }
    };

    tracing::info!(
        "✍️ Generated post for {}: \"{}...\"",
        tenant_id,
        post_content.chars().take(100).collect::<String>()
    );

    // Step 3: Auto-post to configured platforms
    let db = state.db.lock().await;

    for platform in &config.platforms {
        match platform.as_str() {
            "facebook" => {
                if let Ok(Some(tokens_json)) =
                    db.get_config(tenant_id, "oauth_facebook_tokens")
                    && let Ok(tokens) =
                        serde_json::from_str::<crate::oauth::OAuthTokens>(&tokens_json)
                        && let Ok(Some(page_id)) =
                            db.get_config(tenant_id, "social_fb_page_id")
                        {
                            drop(db);
                            auto_post_facebook(
                                &http,
                                &tokens.access_token,
                                &page_id,
                                &post_content,
                            )
                            .await;
                            return; // Simplified: post to first platform only for now
                        }
                tracing::warn!(
                    "Facebook not configured for tenant {} — skipping",
                    tenant_id
                );
            }
            "instagram" => {
                tracing::info!(
                    "Instagram auto-post for {} — requires Container API (TODO)",
                    tenant_id
                );
            }
            _ => {
                tracing::debug!("Unknown platform: {}", platform);
            }
        }
    }
}

/// Post content to Facebook Page.
async fn auto_post_facebook(
    http: &reqwest::Client,
    access_token: &str,
    page_id: &str,
    content: &str,
) {
    let url = format!("https://graph.facebook.com/v21.0/{}/feed", page_id);

    match http
        .post(&url)
        .form(&[("message", content), ("access_token", access_token)])
        .send()
        .await
    {
        Ok(resp) => {
            let status = resp.status();
            let body: serde_json::Value = resp.json().await.unwrap_or_default();

            if status.is_success() {
                let post_id = body["id"].as_str().unwrap_or("unknown");
                tracing::info!("✅ Auto-posted to Facebook! Post ID: {}", post_id);
            } else {
                let err = body["error"]["message"].as_str().unwrap_or("unknown");
                tracing::error!("❌ Facebook post failed: {}", err);
            }
        }
        Err(e) => {
            tracing::error!("❌ Facebook post request failed: {}", e);
        }
    }
}

// ══════════════════════════════════════════════════════════
// 3. SOCIAL STATUS API
// ══════════════════════════════════════════════════════════

/// GET /api/social/status/{tenant_id} — overview of social automation.
pub async fn social_status(
    State(state): State<Arc<AdminState>>,
    axum::extract::Path(tenant_id): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    let db = state.db.lock().await;

    let auto_reply = db
        .get_config(&tenant_id, "auto_reply_enabled")
        .ok()
        .flatten()
        .map(|v| v == "true")
        .unwrap_or(false);

    let pipeline = db
        .get_config(&tenant_id, "content_pipeline")
        .ok()
        .flatten()
        .and_then(|json| serde_json::from_str::<ContentPipelineConfig>(&json).ok());

    let fb_connected = db
        .get_config(&tenant_id, "oauth_facebook_tokens")
        .ok()
        .flatten()
        .is_some();

    let ig_connected = db
        .get_config(&tenant_id, "oauth_instagram_tokens")
        .ok()
        .flatten()
        .is_some();

    let fb_page_id = db
        .get_config(&tenant_id, "social_fb_page_id")
        .ok()
        .flatten();

    Json(serde_json::json!({
        "tenant_id": tenant_id,
        "auto_reply": {
            "enabled": auto_reply,
            "platforms": {
                "facebook": fb_connected,
                "instagram": ig_connected,
            },
            "fb_page_id": fb_page_id,
        },
        "content_pipeline": {
            "configured": pipeline.is_some(),
            "active": pipeline.as_ref().map(|p| p.active).unwrap_or(false),
            "source_urls_count": pipeline.as_ref().map(|p| p.source_urls.len()).unwrap_or(0),
            "platforms": pipeline.as_ref().map(|p| p.platforms.clone()).unwrap_or_default(),
        },
    }))
}

// ══════════════════════════════════════════════════════════
// HELPER: LLM CALLS
// ══════════════════════════════════════════════════════════

async fn call_anthropic(
    http: &reqwest::Client,
    api_key: &str,
    system: &str,
    user: &str,
) -> Result<String, String> {
    let body = serde_json::json!({
        "model": "claude-sonnet-4-20250514",
        "max_tokens": 500,
        "system": system,
        "messages": [{"role": "user", "content": user}]
    });

    let resp = http
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Anthropic request failed: {e}"))?;

    let result: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Anthropic parse failed: {e}"))?;

    result["content"][0]["text"]
        .as_str()
        .map(String::from)
        .ok_or_else(|| {
            format!(
                "No text in Anthropic response: {}",
                result.to_string().chars().take(200).collect::<String>()
            )
        })
}

async fn call_openai(
    http: &reqwest::Client,
    api_key: &str,
    system: &str,
    user: &str,
) -> Result<String, String> {
    let body = serde_json::json!({
        "model": "gpt-4o-mini",
        "max_tokens": 500,
        "messages": [
            {"role": "system", "content": system},
            {"role": "user", "content": user},
        ]
    });

    let resp = http
        .post("https://api.openai.com/v1/chat/completions")
        .bearer_auth(api_key)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("OpenAI request failed: {e}"))?;

    let result: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("OpenAI parse failed: {e}"))?;

    result["choices"][0]["message"]["content"]
        .as_str()
        .map(String::from)
        .ok_or_else(|| "No content in OpenAI response".into())
}

/// Simple HTML tag stripper.
fn strip_html(html: &str) -> String {
    let mut result = String::with_capacity(html.len());
    let mut in_tag = false;
    let in_script = false;

    for c in html.chars() {
        if c == '<' {
            in_tag = true;
            continue;
        }
        if c == '>' {
            in_tag = false;
            continue;
        }
        if in_tag {
            // Check for script/style tags
            continue;
        }
        if !in_script {
            if c == '\n' || c == '\r' {
                if !result.ends_with('\n') {
                    result.push('\n');
                }
            } else {
                result.push(c);
            }
        }
    }

    // Collapse whitespace
    let _ = in_script; // suppress warning
    result
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_html() {
        assert_eq!(strip_html("<p>Hello</p>"), "Hello");
        assert_eq!(strip_html("<div>A</div><div>B</div>"), "AB");
        assert_eq!(strip_html("No tags"), "No tags");
    }

    #[test]
    fn test_pipeline_config_default() {
        let config = ContentPipelineConfig::default();
        assert!(!config.active);
        assert!(config.source_urls.is_empty());
        assert_eq!(config.platforms, vec!["facebook".to_string()]);
        assert_eq!(config.schedule.len(), 3);
    }

    #[test]
    fn test_pipeline_config_serialization() {
        let config = ContentPipelineConfig {
            source_urls: vec!["https://example.com".into()],
            business_context: "Homestay Đà Lạt".into(),
            platforms: vec!["facebook".into(), "instagram".into()],
            schedule: vec!["08:00".into()],
            topics: vec!["du lịch".into(), "ẩm thực".into()],
            active: true,
        };
        let json = serde_json::to_string(&config).unwrap();
        let parsed: ContentPipelineConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.source_urls.len(), 1);
        assert!(parsed.active);
    }

    #[test]
    fn test_incoming_comment() {
        let comment = IncomingComment {
            comment_id: "123".into(),
            post_id: "456".into(),
            page_id: "789".into(),
            sender_name: "Nguyễn Văn A".into(),
            sender_id: "111".into(),
            message: "Phòng còn không ạ?".into(),
            created_time: "2026-03-30".into(),
            platform: "facebook".into(),
        };
        let json = serde_json::to_value(&comment).unwrap();
        assert_eq!(json["platform"], "facebook");
        assert_eq!(json["message"], "Phòng còn không ạ?");
    }
}
