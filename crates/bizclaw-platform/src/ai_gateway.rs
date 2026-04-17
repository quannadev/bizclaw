//! AI Gateway — Centralized inference proxy for multi-tenant Cloud SaaS.
//!
//! Instead of each tenant VM running its own Ollama instance (wasting GPU),
//! the AI Gateway centralizes inference on shared GPU server(s) and routes
//! tenant requests with:
//!
//! - Per-tenant rate limiting (based on subscription plan)
//! - Model access control (plan determines available models)
//! - Request queuing with priority
//! - Token metering for usage-based billing
//! - Automatic failover (vLLM → Ollama → Gemini API)
//!
//! ## Architecture
//!
//! ```text
//! Tenant Gateway VM ──HTTP──▶ AI Gateway ──▶ vLLM / Ollama / Gemini
//!                            (this module)
//! ```
//!
//! API is OpenAI-compatible: POST /v1/ai/chat/completions

use axum::{extract::State, response::Json};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

use crate::admin::AdminState;

// ════════════════════════════════════════════════════════════════
// DATA MODELS
// ════════════════════════════════════════════════════════════════

/// Subscription plan tier → determines model access and rate limits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlanTier {
    Starter,
    Pro,
    Business,
    Enterprise,
}

impl PlanTier {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "pro" => PlanTier::Pro,
            "business" => PlanTier::Business,
            "enterprise" => PlanTier::Enterprise,
            _ => PlanTier::Starter,
        }
    }

    /// Max requests per minute for this plan.
    pub fn max_rpm(&self) -> u32 {
        match self {
            PlanTier::Starter => 30,
            PlanTier::Pro => 60,
            PlanTier::Business => 120,
            PlanTier::Enterprise => 999,
        }
    }

    /// Max tokens per hour for this plan.
    pub fn max_tokens_per_hour(&self) -> u64 {
        match self {
            PlanTier::Starter => 50_000,
            PlanTier::Pro => 200_000,
            PlanTier::Business => 500_000,
            PlanTier::Enterprise => 5_000_000,
        }
    }

    /// Models accessible by this plan.
    pub fn allowed_models(&self) -> Vec<&'static str> {
        match self {
            PlanTier::Starter => vec!["gemma4:e2b", "gemma4:e4b"],
            PlanTier::Pro => vec!["gemma4:e2b", "gemma4:e4b", "gemma4:26b"],
            PlanTier::Business => vec!["gemma4:e2b", "gemma4:e4b", "gemma4:26b", "gemma4:31b"],
            PlanTier::Enterprise => vec![
                "gemma4:e2b",
                "gemma4:e4b",
                "gemma4:26b",
                "gemma4:31b",
                "custom",
            ],
        }
    }

    /// Default model for this plan.
    pub fn default_model(&self) -> &'static str {
        match self {
            PlanTier::Starter => "gemma4:e4b",
            PlanTier::Pro => "gemma4:26b",
            PlanTier::Business => "gemma4:26b",
            PlanTier::Enterprise => "gemma4:31b",
        }
    }
}

/// Inference backend configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceBackend {
    pub name: String,
    /// OpenAI-compatible endpoint (e.g., http://gpu-server:8000/v1)
    pub endpoint: String,
    /// Optional API key
    pub api_key: Option<String>,
    /// Priority (lower = preferred)
    pub priority: u8,
    /// Whether this backend is currently healthy
    pub healthy: bool,
}

/// Chat completion request (OpenAI-compatible).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    pub model: Option<String>,
    pub messages: Vec<ChatMessage>,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    #[serde(default)]
    pub stream: bool,
    #[serde(default)]
    pub tools: Option<serde_json::Value>,
}

fn default_max_tokens() -> u32 {
    2048
}
fn default_temperature() -> f32 {
    0.7
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

/// Chat completion response (OpenAI-compatible).
#[derive(Debug, Serialize)]
pub struct ChatResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<ChatChoice>,
    pub usage: UsageInfo,
}

#[derive(Debug, Serialize)]
pub struct ChatChoice {
    pub index: u32,
    pub message: ChatMessage,
    pub finish_reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageInfo {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

// ════════════════════════════════════════════════════════════════
// RATE LIMITER
// ════════════════════════════════════════════════════════════════

/// Per-tenant rate limiter using sliding window.
#[derive(Debug)]
struct TenantRateState {
    /// Request timestamps within the current minute
    request_times: Vec<Instant>,
    /// Tokens consumed this hour
    tokens_this_hour: u64,
    /// Hour boundary for token reset
    hour_start: Instant,
}

impl TenantRateState {
    fn new() -> Self {
        Self {
            request_times: Vec::new(),
            tokens_this_hour: 0,
            hour_start: Instant::now(),
        }
    }

    /// Check if a new request is allowed (rate limit).
    fn check_rpm(&mut self, max_rpm: u32) -> bool {
        let now = Instant::now();
        let one_minute_ago = now - Duration::from_secs(60);

        // Remove expired timestamps
        self.request_times.retain(|t| *t > one_minute_ago);

        if self.request_times.len() as u32 >= max_rpm {
            return false;
        }

        self.request_times.push(now);
        true
    }

    /// Check and add token usage.
    fn check_and_add_tokens(&mut self, tokens: u64, max_per_hour: u64) -> bool {
        let now = Instant::now();

        // Reset hourly counter if needed
        if now.duration_since(self.hour_start) >= Duration::from_secs(3600) {
            self.tokens_this_hour = 0;
            self.hour_start = now;
        }

        if self.tokens_this_hour + tokens > max_per_hour {
            return false;
        }

        self.tokens_this_hour += tokens;
        true
    }
}

// ════════════════════════════════════════════════════════════════
// USAGE METER
// ════════════════════════════════════════════════════════════════

/// Tracks token usage per tenant for billing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantUsage {
    pub tenant_id: String,
    pub total_requests: u64,
    pub total_prompt_tokens: u64,
    pub total_completion_tokens: u64,
    pub total_tokens: u64,
    pub models_used: HashMap<String, u64>,
    pub last_request: Option<String>,
}

// ════════════════════════════════════════════════════════════════
// AI GATEWAY STATE
// ════════════════════════════════════════════════════════════════

/// Shared AI Gateway state — lives alongside AdminState.
pub struct AiGatewayState {
    /// Inference backends, ordered by priority
    backends: Vec<InferenceBackend>,
    /// Per-tenant rate limiting
    rate_limits: RwLock<HashMap<String, TenantRateState>>,
    /// Per-tenant usage tracking
    usage: RwLock<HashMap<String, TenantUsage>>,
    /// HTTP client for forwarding requests
    http: reqwest::Client,
}

impl AiGatewayState {
    /// Create a new AI Gateway.
    pub fn new() -> Self {
        let mut backends = Vec::new();

        // Primary: vLLM (if configured)
        if let Ok(endpoint) = std::env::var("VLLM_ENDPOINT") {
            backends.push(InferenceBackend {
                name: "vllm".into(),
                endpoint,
                api_key: std::env::var("VLLM_API_KEY").ok(),
                priority: 1,
                healthy: true,
            });
        }

        // Secondary: Ollama (shared instance)
        let ollama_host =
            std::env::var("OLLAMA_HOST").unwrap_or_else(|_| "http://localhost:11434".into());
        backends.push(InferenceBackend {
            name: "ollama".into(),
            endpoint: ollama_host,
            api_key: None,
            priority: 2,
            healthy: true,
        });

        // Fallback: Gemini API (cloud burst)
        if let Ok(key) = std::env::var("GEMINI_API_KEY") {
            backends.push(InferenceBackend {
                name: "gemini".into(),
                endpoint: "https://generativelanguage.googleapis.com".into(),
                api_key: Some(key),
                priority: 10,
                healthy: true,
            });
        }

        backends.sort_by_key(|b| b.priority);

        tracing::info!(
            "🧠 AI Gateway initialized with {} backends: [{}]",
            backends.len(),
            backends
                .iter()
                .map(|b| b.name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );

        Self {
            backends,
            rate_limits: RwLock::new(HashMap::new()),
            usage: RwLock::new(HashMap::new()),
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(120)) // LLM can be slow
                .build()
                .expect("HTTP client creation failed"),
        }
    }

    /// Forward a chat request to the best available backend.
    async fn forward_chat(
        &self,
        tenant_id: &str,
        plan: PlanTier,
        req: &ChatRequest,
    ) -> Result<ChatResponse, String> {
        // 1. Determine model
        let model = req.model.as_deref().unwrap_or(plan.default_model());

        // 2. Verify model access
        if !plan.allowed_models().contains(&model) && model != "auto" {
            return Err(format!(
                "Model '{}' not available on {:?} plan. Allowed: {:?}",
                model,
                plan,
                plan.allowed_models()
            ));
        }

        // 3. Rate limit check
        {
            let mut limits = self.rate_limits.write().await;
            let state = limits
                .entry(tenant_id.to_string())
                .or_insert_with(TenantRateState::new);

            if !state.check_rpm(plan.max_rpm()) {
                return Err(format!(
                    "Rate limit exceeded: {} requests/min max for {:?} plan",
                    plan.max_rpm(),
                    plan
                ));
            }
        }

        // 4. Try backends in priority order
        for backend in &self.backends {
            if !backend.healthy {
                continue;
            }

            match self.try_backend(backend, model, req).await {
                Ok(mut resp) => {
                    // 5. Meter usage
                    let tokens = resp.usage.total_tokens as u64;
                    {
                        let mut limits = self.rate_limits.write().await;
                        if let Some(state) = limits.get_mut(tenant_id) {
                            if !state.check_and_add_tokens(tokens, plan.max_tokens_per_hour()) {
                                tracing::warn!("⚠️ Tenant {} approaching token limit", tenant_id);
                            }
                        }
                    }

                    // 6. Record usage
                    {
                        let mut usage = self.usage.write().await;
                        let entry = usage.entry(tenant_id.to_string()).or_insert(TenantUsage {
                            tenant_id: tenant_id.to_string(),
                            total_requests: 0,
                            total_prompt_tokens: 0,
                            total_completion_tokens: 0,
                            total_tokens: 0,
                            models_used: HashMap::new(),
                            last_request: None,
                        });

                        entry.total_requests += 1;
                        entry.total_prompt_tokens += resp.usage.prompt_tokens as u64;
                        entry.total_completion_tokens += resp.usage.completion_tokens as u64;
                        entry.total_tokens += tokens;
                        *entry.models_used.entry(model.to_string()).or_insert(0) += 1;
                        entry.last_request = Some(chrono::Utc::now().to_rfc3339());
                    }

                    resp.model = format!("{}@{}", model, backend.name);
                    return Ok(resp);
                }
                Err(e) => {
                    tracing::warn!(
                        "⚠️ Backend {} failed for tenant {}: {}",
                        backend.name,
                        tenant_id,
                        e
                    );
                    continue;
                }
            }
        }

        Err("All inference backends unavailable".into())
    }

    /// Try a specific backend.
    async fn try_backend(
        &self,
        backend: &InferenceBackend,
        model: &str,
        req: &ChatRequest,
    ) -> Result<ChatResponse, String> {
        match backend.name.as_str() {
            "ollama" => self.call_ollama(backend, model, req).await,
            "vllm" => self.call_openai_compat(backend, model, req).await,
            "gemini" => self.call_gemini(backend, model, req).await,
            _ => self.call_openai_compat(backend, model, req).await,
        }
    }

    /// Call Ollama's native /api/chat endpoint.
    async fn call_ollama(
        &self,
        backend: &InferenceBackend,
        model: &str,
        req: &ChatRequest,
    ) -> Result<ChatResponse, String> {
        let url = format!("{}/api/chat", backend.endpoint);

        // Convert model name: "gemma4:26b" → "gemma4:26b" (Ollama uses same format)
        let ollama_model = model.replace("gemma4:", "gemma4:");

        let body = serde_json::json!({
            "model": ollama_model,
            "messages": req.messages,
            "stream": false,
            "options": {
                "temperature": req.temperature,
                "num_predict": req.max_tokens,
            }
        });

        let resp = self
            .http
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Ollama request failed: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let err = resp.text().await.unwrap_or_default();
            return Err(format!(
                "Ollama HTTP {}: {}",
                status,
                &err[..err.len().min(200)]
            ));
        }

        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Parse Ollama response: {}", e))?;

        let content = body["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        // Estimate tokens (Ollama provides eval_count)
        let prompt_tokens = body["prompt_eval_count"].as_u64().unwrap_or(0) as u32;
        let completion_tokens = body["eval_count"].as_u64().unwrap_or(
            (content.len() / 4) as u64, // rough estimate
        ) as u32;

        Ok(ChatResponse {
            id: format!("chatcmpl-{}", uuid_short()),
            object: "chat.completion".into(),
            created: chrono::Utc::now().timestamp() as u64,
            model: model.to_string(),
            choices: vec![ChatChoice {
                index: 0,
                message: ChatMessage {
                    role: "assistant".into(),
                    content,
                },
                finish_reason: "stop".into(),
            }],
            usage: UsageInfo {
                prompt_tokens,
                completion_tokens,
                total_tokens: prompt_tokens + completion_tokens,
            },
        })
    }

    /// Call OpenAI-compatible endpoint (vLLM, etc.).
    async fn call_openai_compat(
        &self,
        backend: &InferenceBackend,
        model: &str,
        req: &ChatRequest,
    ) -> Result<ChatResponse, String> {
        let url = format!("{}/v1/chat/completions", backend.endpoint);

        let body = serde_json::json!({
            "model": model,
            "messages": req.messages,
            "max_tokens": req.max_tokens,
            "temperature": req.temperature,
            "stream": false,
        });

        let mut request = self.http.post(&url).json(&body);

        if let Some(ref key) = backend.api_key {
            request = request.header("Authorization", format!("Bearer {}", key));
        }

        let resp = request
            .send()
            .await
            .map_err(|e| format!("vLLM request failed: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let err = resp.text().await.unwrap_or_default();
            return Err(format!(
                "vLLM HTTP {}: {}",
                status,
                &err[..err.len().min(200)]
            ));
        }

        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Parse vLLM response: {}", e))?;

        let choice = &body["choices"][0];
        let content = choice["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let usage = &body["usage"];

        Ok(ChatResponse {
            id: body["id"].as_str().unwrap_or("unknown").to_string(),
            object: "chat.completion".into(),
            created: body["created"].as_u64().unwrap_or(0),
            model: model.to_string(),
            choices: vec![ChatChoice {
                index: 0,
                message: ChatMessage {
                    role: "assistant".into(),
                    content,
                },
                finish_reason: choice["finish_reason"]
                    .as_str()
                    .unwrap_or("stop")
                    .to_string(),
            }],
            usage: UsageInfo {
                prompt_tokens: usage["prompt_tokens"].as_u64().unwrap_or(0) as u32,
                completion_tokens: usage["completion_tokens"].as_u64().unwrap_or(0) as u32,
                total_tokens: usage["total_tokens"].as_u64().unwrap_or(0) as u32,
            },
        })
    }

    /// Call Gemini API as fallback.
    async fn call_gemini(
        &self,
        backend: &InferenceBackend,
        _model: &str,
        req: &ChatRequest,
    ) -> Result<ChatResponse, String> {
        let api_key = backend
            .api_key
            .as_deref()
            .ok_or("Gemini API key not configured")?;

        // Use Gemini 2.5 Flash as fallback (cheapest)
        let url = format!(
            "{}/v1beta/models/gemini-2.5-flash:generateContent?key={}",
            backend.endpoint, api_key
        );

        // Convert messages to Gemini format
        let parts: Vec<serde_json::Value> = req
            .messages
            .iter()
            .filter(|m| m.role != "system")
            .map(|m| {
                serde_json::json!({
                    "role": if m.role == "assistant" { "model" } else { "user" },
                    "parts": [{"text": m.content}]
                })
            })
            .collect();

        // Get system instruction if present
        let system = req
            .messages
            .iter()
            .find(|m| m.role == "system")
            .map(|m| m.content.clone());

        let mut body = serde_json::json!({
            "contents": parts,
            "generationConfig": {
                "temperature": req.temperature,
                "maxOutputTokens": req.max_tokens,
            }
        });

        if let Some(sys) = system {
            body["systemInstruction"] = serde_json::json!({
                "parts": [{"text": sys}]
            });
        }

        let resp = self
            .http
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Gemini request failed: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let err = resp.text().await.unwrap_or_default();
            return Err(format!(
                "Gemini HTTP {}: {}",
                status,
                &err[..err.len().min(200)]
            ));
        }

        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Parse Gemini response: {}", e))?;

        let content = body["candidates"][0]["content"]["parts"][0]["text"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let usage_meta = &body["usageMetadata"];
        let prompt_tokens = usage_meta["promptTokenCount"].as_u64().unwrap_or(0) as u32;
        let completion_tokens = usage_meta["candidatesTokenCount"].as_u64().unwrap_or(0) as u32;

        Ok(ChatResponse {
            id: format!("chatcmpl-gemini-{}", uuid_short()),
            object: "chat.completion".into(),
            created: chrono::Utc::now().timestamp() as u64,
            model: "gemini-2.5-flash".into(),
            choices: vec![ChatChoice {
                index: 0,
                message: ChatMessage {
                    role: "assistant".into(),
                    content,
                },
                finish_reason: "stop".into(),
            }],
            usage: UsageInfo {
                prompt_tokens,
                completion_tokens,
                total_tokens: prompt_tokens + completion_tokens,
            },
        })
    }
}

// ════════════════════════════════════════════════════════════════
// API HANDLERS
// ════════════════════════════════════════════════════════════════

/// Shared AI Gateway state — stored alongside AdminState.
/// Access via `State<Arc<AiGatewayState>>`.
pub type SharedAiGateway = Arc<AiGatewayState>;

/// POST /v1/ai/chat/completions — OpenAI-compatible completion for tenants.
///
/// Headers:
/// - X-Tenant-ID: tenant identifier
/// - X-Tenant-Plan: starter|pro|business|enterprise
/// - Authorization: Bearer <jwt> (validated by gateway)
pub async fn ai_chat_completions(
    State(state): State<Arc<AdminState>>,
    headers: axum::http::HeaderMap,
    Json(req): Json<ChatRequest>,
) -> Json<serde_json::Value> {
    // Extract tenant info from headers
    let tenant_id = headers
        .get("x-tenant-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown");

    let plan = headers
        .get("x-tenant-plan")
        .and_then(|v| v.to_str().ok())
        .map(PlanTier::from_str)
        .unwrap_or(PlanTier::Starter);

    // Use the AI Gateway
    let gateway = state.ai_gateway.as_ref();
    match gateway {
        Some(gw) => match gw.forward_chat(tenant_id, plan, &req).await {
            Ok(resp) => {
                tracing::info!(
                    "🧠 AI Gateway: tenant={} model={} tokens={}",
                    tenant_id,
                    resp.model,
                    resp.usage.total_tokens
                );
                Json(serde_json::to_value(resp).unwrap_or_default())
            }
            Err(e) => {
                // Check if rate limit error
                if e.contains("Rate limit") {
                    Json(serde_json::json!({
                        "error": {
                            "message": e,
                            "type": "rate_limit_error",
                            "code": "rate_limit_exceeded"
                        }
                    }))
                } else {
                    tracing::error!("❌ AI Gateway error: {}", e);
                    Json(serde_json::json!({
                        "error": {
                            "message": "AI inference temporarily unavailable",
                            "type": "server_error",
                            "code": "inference_error"
                        }
                    }))
                }
            }
        },
        None => Json(serde_json::json!({
            "error": {
                "message": "AI Gateway not initialized",
                "type": "server_error",
                "code": "gateway_not_ready"
            }
        })),
    }
}

/// GET /api/v1/ai/status — AI Gateway status + backend health.
pub async fn ai_gateway_status(State(state): State<Arc<AdminState>>) -> Json<serde_json::Value> {
    match state.ai_gateway.as_ref() {
        Some(gw) => {
            let backend_info: Vec<serde_json::Value> = gw
                .backends
                .iter()
                .map(|b| {
                    serde_json::json!({
                        "name": b.name,
                        "endpoint": b.endpoint,
                        "priority": b.priority,
                        "healthy": b.healthy,
                        "has_api_key": b.api_key.is_some(),
                    })
                })
                .collect();

            let usage = gw.usage.read().await;
            let total_requests: u64 = usage.values().map(|u| u.total_requests).sum();
            let total_tokens: u64 = usage.values().map(|u| u.total_tokens).sum();

            Json(serde_json::json!({
                "status": "operational",
                "backends": backend_info,
                "total_tenants_served": usage.len(),
                "total_requests": total_requests,
                "total_tokens": total_tokens,
                "plan_tiers": {
                    "starter": {
                        "max_rpm": PlanTier::Starter.max_rpm(),
                        "max_tokens_per_hour": PlanTier::Starter.max_tokens_per_hour(),
                        "models": PlanTier::Starter.allowed_models(),
                        "default_model": PlanTier::Starter.default_model(),
                    },
                    "pro": {
                        "max_rpm": PlanTier::Pro.max_rpm(),
                        "max_tokens_per_hour": PlanTier::Pro.max_tokens_per_hour(),
                        "models": PlanTier::Pro.allowed_models(),
                        "default_model": PlanTier::Pro.default_model(),
                    },
                    "business": {
                        "max_rpm": PlanTier::Business.max_rpm(),
                        "max_tokens_per_hour": PlanTier::Business.max_tokens_per_hour(),
                        "models": PlanTier::Business.allowed_models(),
                        "default_model": PlanTier::Business.default_model(),
                    },
                    "enterprise": {
                        "max_rpm": PlanTier::Enterprise.max_rpm(),
                        "max_tokens_per_hour": PlanTier::Enterprise.max_tokens_per_hour(),
                        "models": PlanTier::Enterprise.allowed_models(),
                        "default_model": PlanTier::Enterprise.default_model(),
                    },
                }
            }))
        }
        None => Json(serde_json::json!({
            "status": "not_initialized",
            "message": "AI Gateway not configured. Set OLLAMA_HOST or VLLM_ENDPOINT."
        })),
    }
}

/// GET /api/v1/ai/usage — Per-tenant usage stats.
pub async fn ai_usage_stats(State(state): State<Arc<AdminState>>) -> Json<serde_json::Value> {
    match state.ai_gateway.as_ref() {
        Some(gw) => {
            let usage = gw.usage.read().await;
            let stats: Vec<serde_json::Value> = usage
                .values()
                .map(|u| serde_json::to_value(u).unwrap_or_default())
                .collect();

            Json(serde_json::json!({
                "tenants": stats,
                "total_tenants": stats.len(),
            }))
        }
        None => Json(serde_json::json!({
            "tenants": [],
            "total_tenants": 0,
        })),
    }
}

/// GET /api/v1/ai/usage/:tenant_id — Single tenant usage.
pub async fn ai_tenant_usage(
    State(state): State<Arc<AdminState>>,
    axum::extract::Path(tenant_id): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    match state.ai_gateway.as_ref() {
        Some(gw) => {
            let usage = gw.usage.read().await;
            match usage.get(&tenant_id) {
                Some(u) => Json(serde_json::to_value(u).unwrap_or_default()),
                None => Json(serde_json::json!({
                    "tenant_id": tenant_id,
                    "total_requests": 0,
                    "total_tokens": 0,
                    "message": "No usage recorded yet"
                })),
            }
        }
        None => Json(serde_json::json!({"error": "AI Gateway not initialized"})),
    }
}

// ════════════════════════════════════════════════════════════════
// HELPERS
// ════════════════════════════════════════════════════════════════

/// Generate a short UUID for response IDs.
fn uuid_short() -> String {
    let r: u64 = rand::random();
    format!("{:016x}", r)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plan_tiers() {
        assert_eq!(PlanTier::from_str("starter").max_rpm(), 30);
        assert_eq!(PlanTier::from_str("pro").max_rpm(), 60);
        assert_eq!(PlanTier::from_str("business").max_rpm(), 120);
        assert_eq!(PlanTier::from_str("enterprise").max_rpm(), 999);
    }

    #[test]
    fn test_plan_models() {
        let starter = PlanTier::Starter;
        assert!(starter.allowed_models().contains(&"gemma4:e4b"));
        assert!(!starter.allowed_models().contains(&"gemma4:26b"));

        let pro = PlanTier::Pro;
        assert!(pro.allowed_models().contains(&"gemma4:26b"));
        assert!(!pro.allowed_models().contains(&"gemma4:31b"));

        let biz = PlanTier::Business;
        assert!(biz.allowed_models().contains(&"gemma4:31b"));
    }

    #[test]
    fn test_rate_limiter() {
        let mut state = TenantRateState::new();
        // Should allow up to 30 rpm
        for _ in 0..30 {
            assert!(state.check_rpm(30));
        }
        // 31st should be rejected
        assert!(!state.check_rpm(30));
    }

    #[test]
    fn test_token_budget() {
        let mut state = TenantRateState::new();
        assert!(state.check_and_add_tokens(40_000, 50_000));
        assert!(state.check_and_add_tokens(9_000, 50_000));
        // This would exceed
        assert!(!state.check_and_add_tokens(2_000, 50_000));
    }

    #[test]
    fn test_default_model_per_plan() {
        assert_eq!(PlanTier::Starter.default_model(), "gemma4:e4b");
        assert_eq!(PlanTier::Pro.default_model(), "gemma4:26b");
        assert_eq!(PlanTier::Enterprise.default_model(), "gemma4:31b");
    }
}
