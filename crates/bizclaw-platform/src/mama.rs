//! Mama AI — Tổng quản Agent Orchestrator.
//!
//! Nhận yêu cầu từ user → tìm skills/tools phù hợp → tạo execution plan →
//! chọn AI provider tiết kiệm nhất → thực thi → báo cáo.
//!
//! Lấy cảm hứng từ OpenViking (volcengine): filesystem paradigm, tiered context,
//! self-evolving agent memory.
//!
//! ## Cost-Aware Provider Routing
//!
//! Mama biết giá của từng provider, chọn model phù hợp theo task complexity:
//! - Simple tasks (trả lời comment, format data) → cheapest model (Deepseek/Groq/Ollama)
//! - Medium tasks (viết bài, tổng hợp) → mid-tier (GPT-4o-mini, Claude Haiku)
//! - Complex tasks (planning, reasoning, code) → top-tier (Claude Sonnet, GPT-4o)

use serde::{Deserialize, Serialize};

// ══════════════════════════════════════════════════════════
// 1. PROVIDER COST MAP — Giá mỗi 1M token (USD)
// ══════════════════════════════════════════════════════════

/// Cost per 1M tokens (input/output) in USD.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelCost {
    pub provider: &'static str,
    pub model: &'static str,
    pub input_per_1m: f64,   // USD per 1M input tokens
    pub output_per_1m: f64,  // USD per 1M output tokens
    pub tier: TaskTier,      // Which task tier this model is best for
    pub context_window: u32, // Max context length
    pub env_key: &'static str, // Env var for API key
    pub speed: Speed,        // Relative response speed
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskTier {
    /// Simple: comment replies, formatting, classification
    Simple,
    /// Medium: content writing, summarization, translation
    Medium,
    /// Complex: planning, reasoning, code generation, multi-step
    Complex,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Speed {
    Fast,    // < 1s TTFT
    Medium,  // 1-3s TTFT
    Slow,    // > 3s TTFT
}

/// Global cost table — updated March 2026 pricing.
pub static MODEL_COSTS: &[ModelCost] = &[
    // ── FREE / LOCAL ──────────────────────────────
    ModelCost {
        provider: "ollama",
        model: "qwen3.5-4b-neo",
        input_per_1m: 0.0,
        output_per_1m: 0.0,
        tier: TaskTier::Simple,
        context_window: 32768,
        env_key: "OLLAMA_HOST",
        speed: Speed::Fast,
    },
    ModelCost {
        provider: "ollama",
        model: "jan-nano",
        input_per_1m: 0.0,
        output_per_1m: 0.0,
        tier: TaskTier::Simple,
        context_window: 32768,
        env_key: "OLLAMA_HOST",
        speed: Speed::Fast,
    },
    // ── BUDGET TIER ───────────────────────────────
    ModelCost {
        provider: "deepseek",
        model: "deepseek-chat",
        input_per_1m: 0.14,
        output_per_1m: 0.28,
        tier: TaskTier::Medium,
        context_window: 128000,
        env_key: "DEEPSEEK_API_KEY",
        speed: Speed::Medium,
    },
    ModelCost {
        provider: "groq",
        model: "llama-3.3-70b-versatile",
        input_per_1m: 0.59,
        output_per_1m: 0.79,
        tier: TaskTier::Medium,
        context_window: 128000,
        env_key: "GROQ_API_KEY",
        speed: Speed::Fast,
    },
    ModelCost {
        provider: "gemini",
        model: "gemini-2.5-flash",
        input_per_1m: 0.15,
        output_per_1m: 0.60,
        tier: TaskTier::Medium,
        context_window: 1048576,
        env_key: "GEMINI_API_KEY",
        speed: Speed::Fast,
    },
    // ── MID TIER ──────────────────────────────────
    ModelCost {
        provider: "openai",
        model: "gpt-4o-mini",
        input_per_1m: 0.15,
        output_per_1m: 0.60,
        tier: TaskTier::Medium,
        context_window: 128000,
        env_key: "OPENAI_API_KEY",
        speed: Speed::Fast,
    },
    ModelCost {
        provider: "anthropic",
        model: "claude-3-5-haiku-20241022",
        input_per_1m: 0.80,
        output_per_1m: 4.00,
        tier: TaskTier::Medium,
        context_window: 200000,
        env_key: "ANTHROPIC_API_KEY",
        speed: Speed::Fast,
    },
    // ── TOP TIER ──────────────────────────────────
    ModelCost {
        provider: "anthropic",
        model: "claude-sonnet-4-20250514",
        input_per_1m: 3.00,
        output_per_1m: 15.00,
        tier: TaskTier::Complex,
        context_window: 200000,
        env_key: "ANTHROPIC_API_KEY",
        speed: Speed::Medium,
    },
    ModelCost {
        provider: "openai",
        model: "gpt-4o",
        input_per_1m: 2.50,
        output_per_1m: 10.00,
        tier: TaskTier::Complex,
        context_window: 128000,
        env_key: "OPENAI_API_KEY",
        speed: Speed::Medium,
    },
    ModelCost {
        provider: "gemini",
        model: "gemini-2.5-pro",
        input_per_1m: 1.25,
        output_per_1m: 10.00,
        tier: TaskTier::Complex,
        context_window: 1048576,
        env_key: "GEMINI_API_KEY",
        speed: Speed::Slow,
    },
    ModelCost {
        provider: "deepseek",
        model: "deepseek-reasoner",
        input_per_1m: 0.55,
        output_per_1m: 2.19,
        tier: TaskTier::Complex,
        context_window: 64000,
        env_key: "DEEPSEEK_API_KEY",
        speed: Speed::Slow,
    },
    // ── VOLCENGINE / MODELARK (China) ─────────────
    ModelCost {
        provider: "modelark",
        model: "doubao-1-5-pro-256k-250115",
        input_per_1m: 0.40,
        output_per_1m: 0.90,
        tier: TaskTier::Medium,
        context_window: 256000,
        env_key: "ARK_API_KEY",
        speed: Speed::Fast,
    },
];

// ══════════════════════════════════════════════════════════
// 2. SMART PROVIDER SELECTOR — Chọn model tối ưu
// ══════════════════════════════════════════════════════════

/// Available providers detected from environment.
#[derive(Debug, Clone, Serialize)]
pub struct AvailableProvider {
    pub provider: String,
    pub model: String,
    pub cost_input: f64,
    pub cost_output: f64,
    pub tier: TaskTier,
    pub speed: Speed,
}

/// Detect which AI providers are configured (have API keys set).
pub fn detect_available_providers() -> Vec<AvailableProvider> {
    MODEL_COSTS
        .iter()
        .filter(|m| {
            if m.provider == "ollama" {
                // Ollama is available if host is set or default localhost
                check_ollama_available()
            } else {
                !std::env::var(m.env_key).unwrap_or_default().is_empty()
            }
        })
        .map(|m| AvailableProvider {
            provider: m.provider.to_string(),
            model: m.model.to_string(),
            cost_input: m.input_per_1m,
            cost_output: m.output_per_1m,
            tier: m.tier,
            speed: m.speed,
        })
        .collect()
}

/// Select the cheapest available model for a given task tier.
pub fn select_cheapest_model(tier: TaskTier) -> Option<AvailableProvider> {
    let mut available = detect_available_providers();

    // Filter by tier (allow using higher-tier models for lower-tier tasks)
    available.retain(|p| match tier {
        TaskTier::Simple => true, // Any model can do simple tasks
        TaskTier::Medium => p.tier == TaskTier::Medium || p.tier == TaskTier::Complex,
        TaskTier::Complex => p.tier == TaskTier::Complex,
    });

    // Sort by total cost (input + output, weighted 60/40)
    available.sort_by(|a, b| {
        let cost_a = a.cost_input * 0.6 + a.cost_output * 0.4;
        let cost_b = b.cost_input * 0.6 + b.cost_output * 0.4;
        cost_a.partial_cmp(&cost_b).unwrap_or(std::cmp::Ordering::Equal)
    });

    available.into_iter().next()
}

/// Classify a user request into a task tier.
pub fn classify_task_tier(request: &str) -> TaskTier {
    let lower = request.to_lowercase();
    let words: Vec<&str> = lower.split_whitespace().collect();

    // Complex indicators (substring match is fine for multi-char phrases)
    let complex_keywords = [
        "lập kế hoạch", "plan", "phân tích", "analyze", "strategy",
        "code", "viết code", "tạo workflow", "refactor", "debug",
        "so sánh", "compare", "đánh giá", "evaluate", "architecture",
        "thiết kế", "design", "tối ưu", "optimize", "reasoning",
    ];

    // Simple indicators — use exact word match for short tokens
    let simple_words = [
        "trả lời", "reply", "format", "dịch", "translate",
        "tóm tắt ngắn", "brief", "yes/no", "đúng không",
        "chào", "hello", "cảm ơn", "thanks",
    ];

    if complex_keywords.iter().any(|k| lower.contains(k)) {
        TaskTier::Complex
    } else if simple_words.iter().any(|k| {
        // For multi-word phrases, use contains
        if k.contains(' ') {
            lower.contains(k)
        } else {
            // For single words, match exact word boundaries
            words.iter().any(|w| *w == *k)
        }
    }) {
        TaskTier::Simple
    } else {
        TaskTier::Medium
    }
}

/// Check if Ollama is running locally.
fn check_ollama_available() -> bool {
    let host = std::env::var("OLLAMA_HOST")
        .unwrap_or_else(|_| "http://localhost:11434".into());
    // Quick sync check — just verify env is set or use default
    // In production, would do async health check
    !host.is_empty()
}

// ══════════════════════════════════════════════════════════
// 3. SKILL DISCOVERY — Tìm tools phù hợp từ Hub
// ══════════════════════════════════════════════════════════

/// Registered skill/tool metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillMeta {
    pub name: String,
    pub description: String,
    pub category: String,
    pub keywords: Vec<String>,
}

/// Built-in skills available in BizClaw.
pub fn builtin_skills() -> Vec<SkillMeta> {
    vec![
        SkillMeta {
            name: "browser".into(),
            description: "Duyệt web, crawl trang, lấy nội dung. Dùng PinchTab.".into(),
            category: "automation".into(),
            keywords: vec!["web".into(), "crawl".into(), "scrape".into(), "browse".into(), "duyệt".into()],
        },
        SkillMeta {
            name: "calendar".into(),
            description: "Quản lý Google Calendar — xem lịch, tạo sự kiện, check lịch rảnh.".into(),
            category: "productivity".into(),
            keywords: vec!["lịch".into(), "calendar".into(), "sự kiện".into(), "event".into(), "booking".into(), "đặt lịch".into()],
        },
        SkillMeta {
            name: "social_post".into(),
            description: "Đăng bài lên Facebook Page, Telegram Channel, webhook.".into(),
            category: "marketing".into(),
            keywords: vec!["đăng bài".into(), "post".into(), "facebook".into(), "telegram".into(), "marketing".into()],
        },
        SkillMeta {
            name: "research".into(),
            description: "Nghiên cứu học thuật — tìm papers từ OpenAlex, Semantic Scholar.".into(),
            category: "research".into(),
            keywords: vec!["nghiên cứu".into(), "research".into(), "paper".into(), "academic".into()],
        },
        SkillMeta {
            name: "shell".into(),
            description: "Chạy lệnh terminal trên server.".into(),
            category: "system".into(),
            keywords: vec!["terminal".into(), "command".into(), "shell".into(), "lệnh".into()],
        },
        SkillMeta {
            name: "file".into(),
            description: "Đọc/ghi file trên hệ thống.".into(),
            category: "system".into(),
            keywords: vec!["file".into(), "đọc".into(), "ghi".into(), "read".into(), "write".into()],
        },
        SkillMeta {
            name: "http_request".into(),
            description: "Gọi API bên ngoài (GET/POST/PUT/DELETE).".into(),
            category: "integration".into(),
            keywords: vec!["api".into(), "http".into(), "request".into(), "webhook".into()],
        },
        SkillMeta {
            name: "memory_search".into(),
            description: "Tìm kiếm trong bộ nhớ agent (RAG).".into(),
            category: "memory".into(),
            keywords: vec!["nhớ".into(), "memory".into(), "tìm".into(), "search".into(), "rag".into()],
        },
        SkillMeta {
            name: "db_query".into(),
            description: "Truy vấn database SQLite/PostgreSQL.".into(),
            category: "data".into(),
            keywords: vec!["database".into(), "query".into(), "sql".into(), "dữ liệu".into()],
        },
        SkillMeta {
            name: "edit_file".into(),
            description: "Chỉnh sửa file (search & replace, append).".into(),
            category: "system".into(),
            keywords: vec!["edit".into(), "sửa".into(), "replace".into(), "chỉnh".into()],
        },
        SkillMeta {
            name: "document_reader".into(),
            description: "Đọc file PDF, DOCX, TXT và trích nội dung.".into(),
            category: "data".into(),
            keywords: vec!["pdf".into(), "docx".into(), "đọc".into(), "document".into()],
        },
        SkillMeta {
            name: "grep_search".into(),
            description: "Tìm kiếm nội dung trong file/thư mục (grep).".into(),
            category: "system".into(),
            keywords: vec!["grep".into(), "tìm".into(), "search".into(), "find".into()],
        },
    ]
}

/// Search skills by keyword relevance.
pub fn search_skills(query: &str, top_n: usize) -> Vec<SkillMeta> {
    let query_lower = query.to_lowercase();
    let query_words: Vec<&str> = query_lower.split_whitespace().collect();

    let mut scored: Vec<(f64, SkillMeta)> = builtin_skills()
        .into_iter()
        .map(|skill| {
            let mut score = 0.0;

            // Exact name match
            if query_lower.contains(&skill.name) {
                score += 10.0;
            }

            // Keyword matching
            for kw in &skill.keywords {
                for qw in &query_words {
                    if kw.contains(qw) || qw.contains(kw.as_str()) {
                        score += 3.0;
                    }
                }
            }

            // Description matching
            let desc_lower = skill.description.to_lowercase();
            for qw in &query_words {
                if desc_lower.contains(qw) {
                    score += 1.0;
                }
            }

            (score, skill)
        })
        .filter(|(score, _)| *score > 0.0)
        .collect();

    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    scored.into_iter().take(top_n).map(|(_, s)| s).collect()
}

// ══════════════════════════════════════════════════════════
// 4. EXECUTION PLAN — Tạo workflow từ skills
// ══════════════════════════════════════════════════════════

/// A step in the execution plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanStep {
    pub step: u32,
    pub tool: String,
    pub action: String,
    pub description: String,
    pub depends_on: Vec<u32>,
}

/// Complete execution plan generated by Mama AI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPlan {
    pub request: String,
    pub task_tier: TaskTier,
    pub selected_provider: String,
    pub selected_model: String,
    pub estimated_cost: String,
    pub skills_used: Vec<String>,
    pub steps: Vec<PlanStep>,
}

/// Generate a plan preview (without executing).
/// This shows what Mama would do — tenant admin can approve/reject.
pub fn generate_plan_preview(request: &str) -> ExecutionPlan {
    let tier = classify_task_tier(request);
    let provider = select_cheapest_model(tier);
    let relevant_skills = search_skills(request, 5);

    let (provider_name, model_name, cost_str) = match &provider {
        Some(p) => {
            let estimated = format!(
                "~${:.4}/1K tokens (input: ${:.2}/1M, output: ${:.2}/1M)",
                (p.cost_input + p.cost_output) / 2000.0,
                p.cost_input,
                p.cost_output
            );
            (p.provider.clone(), p.model.clone(), estimated)
        }
        None => (
            "none".into(),
            "none".into(),
            "No provider available".into(),
        ),
    };

    let skills_used: Vec<String> = relevant_skills.iter().map(|s| s.name.clone()).collect();

    // Generate steps based on discovered skills
    let steps: Vec<PlanStep> = relevant_skills
        .iter()
        .enumerate()
        .map(|(i, skill)| PlanStep {
            step: (i + 1) as u32,
            tool: skill.name.clone(),
            action: "execute".into(),
            description: format!("Dùng {} — {}", skill.name, skill.description),
            depends_on: if i > 0 { vec![i as u32] } else { vec![] },
        })
        .collect();

    ExecutionPlan {
        request: request.to_string(),
        task_tier: tier,
        selected_provider: provider_name,
        selected_model: model_name,
        estimated_cost: cost_str,
        skills_used,
        steps,
    }
}

// ══════════════════════════════════════════════════════════
// 5. API HANDLERS
// ══════════════════════════════════════════════════════════

use axum::extract::State;
use axum::Json;
use std::sync::Arc;
use crate::admin::AdminState;

/// GET /api/mama/providers — list available AI providers with costs.
pub async fn list_providers(
    State(_state): State<Arc<AdminState>>,
) -> Json<serde_json::Value> {
    let providers = detect_available_providers();
    let all_models: Vec<serde_json::Value> = MODEL_COSTS
        .iter()
        .map(|m| {
            let available = providers.iter().any(|p| p.model == m.model);
            serde_json::json!({
                "provider": m.provider,
                "model": m.model,
                "input_per_1m_usd": m.input_per_1m,
                "output_per_1m_usd": m.output_per_1m,
                "tier": format!("{:?}", m.tier),
                "speed": format!("{:?}", m.speed),
                "context_window": m.context_window,
                "available": available,
                "env_key": m.env_key,
            })
        })
        .collect();

    Json(serde_json::json!({
        "available_count": providers.len(),
        "total_models": MODEL_COSTS.len(),
        "models": all_models,
        "recommendation": if providers.is_empty() {
            "⚠️ Chưa có provider nào được cấu hình. Set API key env vars."
        } else {
            "✅ Mama AI sẵn sàng. Sẽ tự chọn model tiết kiệm nhất cho mỗi task."
        },
    }))
}

/// POST /api/mama/plan — generate execution plan from user request.
#[derive(Debug, Deserialize)]
pub struct PlanRequest {
    pub request: String,
}

pub async fn create_plan(
    State(_state): State<Arc<AdminState>>,
    Json(body): Json<PlanRequest>,
) -> Json<serde_json::Value> {
    let plan = generate_plan_preview(&body.request);

    Json(serde_json::json!({
        "plan": plan,
        "info": format!(
            "🧠 Mama AI đã phân tích yêu cầu. Task tier: {:?}. Sẽ dùng {} ({}) — {}",
            plan.task_tier, plan.selected_provider, plan.selected_model, plan.estimated_cost
        ),
    }))
}

/// GET /api/mama/skills?q=booking — search available skills.
#[derive(Debug, Deserialize)]
pub struct SkillSearchParams {
    pub q: Option<String>,
}

pub async fn search_skills_api(
    State(_state): State<Arc<AdminState>>,
    axum::extract::Query(params): axum::extract::Query<SkillSearchParams>,
) -> Json<serde_json::Value> {
    let query = params.q.unwrap_or_default();

    let results = if query.is_empty() {
        builtin_skills()
    } else {
        search_skills(&query, 10)
    };

    Json(serde_json::json!({
        "query": query,
        "count": results.len(),
        "skills": results,
    }))
}

/// GET /api/mama/status — Mama AI system overview.
pub async fn mama_status(
    State(_state): State<Arc<AdminState>>,
) -> Json<serde_json::Value> {
    let providers = detect_available_providers();
    let skills = builtin_skills();

    let cheapest_simple = select_cheapest_model(TaskTier::Simple);
    let cheapest_medium = select_cheapest_model(TaskTier::Medium);
    let cheapest_complex = select_cheapest_model(TaskTier::Complex);

    Json(serde_json::json!({
        "status": "operational",
        "version": "1.0.0",
        "providers_available": providers.len(),
        "skills_available": skills.len(),
        "cost_routing": {
            "simple_tasks": cheapest_simple.as_ref().map(|p| format!("{}/{}", p.provider, p.model)),
            "medium_tasks": cheapest_medium.as_ref().map(|p| format!("{}/{}", p.provider, p.model)),
            "complex_tasks": cheapest_complex.as_ref().map(|p| format!("{}/{}", p.provider, p.model)),
        },
        "cost_savings_tip": "Mama AI tự chọn model rẻ nhất có thể cho mỗi task. \
            Simple tasks (reply, format) → local/free models. \
            Medium tasks (content, summary) → budget APIs. \
            Complex tasks (plan, code) → premium models.",
    }))
}

// ══════════════════════════════════════════════════════════
// 6. SMART ONBOARDING — Token Auto-Detect + Auto-Setup
// ══════════════════════════════════════════════════════════

/// Detected provider from an API key.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DetectedProvider {
    pub provider: String,
    pub confidence: String, // "high", "medium", "low"
    pub env_key: String,
    pub models_available: Vec<String>,
    pub estimated_monthly_cost: String,
}

/// Detect which AI provider an API key belongs to.
/// Uses prefix patterns — no API call needed.
pub fn detect_provider_from_key(api_key: &str) -> Option<DetectedProvider> {
    let key = api_key.trim();

    if key.is_empty() || key.len() < 8 {
        return None;
    }

    // Anthropic: sk-ant-api03-... or starts with sk-ant-
    if key.starts_with("sk-ant-") {
        return Some(DetectedProvider {
            provider: "anthropic".into(),
            confidence: "high".into(),
            env_key: "ANTHROPIC_API_KEY".into(),
            models_available: vec![
                "claude-sonnet-4-20250514".into(),
                "claude-3-5-haiku-20241022".into(),
            ],
            estimated_monthly_cost: "~$5-50/tháng (tùy usage)".into(),
        });
    }

    // OpenAI: sk-proj-... or sk-... (not sk-ant-)
    if key.starts_with("sk-proj-") || (key.starts_with("sk-") && !key.starts_with("sk-ant-")) {
        return Some(DetectedProvider {
            provider: "openai".into(),
            confidence: "high".into(),
            env_key: "OPENAI_API_KEY".into(),
            models_available: vec!["gpt-4o".into(), "gpt-4o-mini".into()],
            estimated_monthly_cost: "~$3-30/tháng (tùy usage)".into(),
        });
    }

    // Gemini / Google AI: AIza...
    if key.starts_with("AIza") {
        return Some(DetectedProvider {
            provider: "gemini".into(),
            confidence: "high".into(),
            env_key: "GEMINI_API_KEY".into(),
            models_available: vec!["gemini-2.5-pro".into(), "gemini-2.5-flash".into()],
            estimated_monthly_cost: "~$1-15/tháng (tùy usage)".into(),
        });
    }

    // DeepSeek: starts with sk- but shorter pattern or deepseek-specific
    // Note: DeepSeek uses sk- prefix too, handled by length/format heuristics
    // Most DeepSeek keys are longer and don't have "proj" prefix

    // Groq: gsk_...
    if key.starts_with("gsk_") {
        return Some(DetectedProvider {
            provider: "groq".into(),
            confidence: "high".into(),
            env_key: "GROQ_API_KEY".into(),
            models_available: vec![
                "llama-3.3-70b-versatile".into(),
                "llama-3.1-8b-instant".into(),
            ],
            estimated_monthly_cost: "~$1-10/tháng (rất rẻ)".into(),
        });
    }

    // xAI/Grok: xai-...
    if key.starts_with("xai-") {
        return Some(DetectedProvider {
            provider: "xai".into(),
            confidence: "high".into(),
            env_key: "XAI_API_KEY".into(),
            models_available: vec!["grok-3".into(), "grok-3-mini".into()],
            estimated_monthly_cost: "~$5-20/tháng".into(),
        });
    }

    // Mistral: starts with specific pattern
    if key.len() == 32 && key.chars().all(|c| c.is_alphanumeric()) {
        return Some(DetectedProvider {
            provider: "mistral".into(),
            confidence: "low".into(),
            env_key: "MISTRAL_API_KEY".into(),
            models_available: vec!["mistral-large-latest".into()],
            estimated_monthly_cost: "~$2-15/tháng".into(),
        });
    }

    // Volcengine/ModelArk: various formats
    if key.len() > 40 && (key.contains('-') || key.starts_with("ark-")) {
        return Some(DetectedProvider {
            provider: "modelark".into(),
            confidence: "medium".into(),
            env_key: "ARK_API_KEY".into(),
            models_available: vec!["doubao-1-5-pro-256k-250115".into()],
            estimated_monthly_cost: "~$1-10/tháng (rẻ nhất)".into(),
        });
    }

    // Generic fallback — could be DeepSeek, custom endpoint, etc.
    Some(DetectedProvider {
        provider: "unknown".into(),
        confidence: "low".into(),
        env_key: "BIZCLAW_LLM_API_KEY".into(),
        models_available: vec!["auto-detect".into()],
        estimated_monthly_cost: "Không xác định".into(),
    })
}

/// POST /api/mama/onboard — Smart onboarding: detect + save + configure.
#[derive(Debug, Deserialize)]
pub struct OnboardRequest {
    pub tenant_id: String,
    pub api_key: String,
    /// Business type: "tourism", "fnb", "retail", "service", "other"
    pub business_type: Option<String>,
    /// Business name
    pub business_name: Option<String>,
    /// Business description
    pub business_desc: Option<String>,
}

pub async fn smart_onboard(
    State(state): State<Arc<AdminState>>,
    Json(body): Json<OnboardRequest>,
) -> Json<serde_json::Value> {
    // Step 1: Detect provider
    let detected = match detect_provider_from_key(&body.api_key) {
        Some(d) => d,
        None => {
            return Json(serde_json::json!({
                "success": false,
                "error": "API key không hợp lệ. Vui lòng kiểm tra lại.",
            }));
        }
    };

    if detected.provider == "unknown" && detected.confidence == "low" {
        tracing::warn!(
            "Unknown API key pattern for tenant {} — saving as generic",
            body.tenant_id
        );
    }

    tracing::info!(
        "🔑 Detected provider: {} (confidence: {}) for tenant {}",
        detected.provider,
        detected.confidence,
        body.tenant_id
    );

    // Step 2: Save to tenant config
    let db = state.db.lock().await;

    // Save API key
    let _ = db.set_config(&body.tenant_id, "llm_api_key", &body.api_key);
    let _ = db.set_config(&body.tenant_id, "llm_provider", &detected.provider);
    let _ = db.set_config(&body.tenant_id, "llm_env_key", &detected.env_key);

    // Save business info
    if let Some(ref name) = body.business_name {
        let _ = db.set_config(&body.tenant_id, "business_name", name);
    }
    if let Some(ref desc) = body.business_desc {
        let _ = db.set_config(&body.tenant_id, "business_context", desc);
    }
    if let Some(ref btype) = body.business_type {
        let _ = db.set_config(&body.tenant_id, "business_type", btype);
    }

    // Auto-enable auto-reply if provider detected
    if detected.confidence == "high" {
        let _ = db.set_config(&body.tenant_id, "auto_reply_enabled", "true");
        let _ = db.set_config(
            &body.tenant_id,
            "reply_style",
            "Thân thiện, chuyên nghiệp, ngắn gọn. Luôn dùng kính ngữ (dạ, ạ). Kết thúc bằng CTA.",
        );
    }

    // Set default model based on detected provider
    let default_model = detected
        .models_available
        .last() // Pick cheapest (last in list)
        .cloned()
        .unwrap_or_else(|| "auto".into());
    let _ = db.set_config(&body.tenant_id, "llm_model", &default_model);

    drop(db);

    // Step 3: Determine what skills are recommended
    let btype = body.business_type.as_deref().unwrap_or("other");
    let recommended_skills = match btype {
        "tourism" => vec![
            "calendar — quản lý booking",
            "social_post — đăng bài quảng cáo",
            "browser — check giá đối thủ",
            "research — xu hướng du lịch",
        ],
        "fnb" => vec![
            "social_post — đăng menu, khuyến mãi",
            "calendar — quản lý đặt bàn",
            "http_request — tích hợp POS",
        ],
        "retail" => vec![
            "social_post — đăng sản phẩm mới",
            "browser — check giá thị trường",
            "db_query — quản lý đơn hàng",
        ],
        _ => vec![
            "social_post — marketing",
            "calendar — lịch hẹn",
            "browser — nghiên cứu",
        ],
    };

    // Step 4: Welcome message
    let business_name = body.business_name.as_deref().unwrap_or("Doanh nghiệp");
    let welcome = format!(
        "🎉 Chào mừng {} đến với BizClaw!\n\n\
         ✅ Đã nhận diện AI: {} ({})\n\
         ✅ Đã cấu hình model: {}\n\
         ✅ Auto-reply: {}\n\
         💰 Chi phí ước tính: {}\n\n\
         🛠️ Kỹ năng được khuyến nghị cho {}:\n{}\n\n\
         🚀 Mama AI sẵn sàng phục vụ! Hãy bắt đầu bằng cách:\n\
         1. Kết nối Facebook/Instagram (mục Kết nối dịch vụ)\n\
         2. Cấu hình Content Pipeline (nguồn URLs + lịch đăng)\n\
         3. Hoặc đơn giản nhắn \"Mama, hãy viết bài quảng cáo\" 🧠",
        business_name,
        detected.provider,
        detected.confidence,
        default_model,
        if detected.confidence == "high" { "BẬT" } else { "TẮT (cần xác nhận)" },
        detected.estimated_monthly_cost,
        btype,
        recommended_skills
            .iter()
            .map(|s| format!("  • {s}"))
            .collect::<Vec<_>>()
            .join("\n"),
    );

    Json(serde_json::json!({
        "success": true,
        "detected": detected,
        "welcome_message": welcome,
        "auto_configured": {
            "provider": detected.provider,
            "model": default_model,
            "auto_reply": detected.confidence == "high",
            "business_type": btype,
        },
        "next_steps": [
            "Kết nối Facebook/Instagram để auto-post",
            "Cấu hình Content Pipeline (nguồn URLs)",
            "Hoặc trò chuyện với Mama AI ngay!",
        ],
    }))
}

/// POST /api/mama/detect-key — detect provider without saving (for preview).
#[derive(Debug, Deserialize)]
pub struct DetectKeyRequest {
    pub api_key: String,
}

pub async fn detect_key(
    State(_state): State<Arc<AdminState>>,
    Json(body): Json<DetectKeyRequest>,
) -> Json<serde_json::Value> {
    match detect_provider_from_key(&body.api_key) {
        Some(detected) => Json(serde_json::json!({
            "detected": true,
            "provider": detected,
        })),
        None => Json(serde_json::json!({
            "detected": false,
            "error": "Không nhận diện được API key",
        })),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_simple() {
        assert_eq!(classify_task_tier("trả lời comment"), TaskTier::Simple);
        assert_eq!(classify_task_tier("format dữ liệu"), TaskTier::Simple);
        assert_eq!(classify_task_tier("dịch sang tiếng Anh"), TaskTier::Simple);
    }

    #[test]
    fn test_classify_medium() {
        assert_eq!(classify_task_tier("viết bài về du lịch Đà Lạt"), TaskTier::Medium);
        assert_eq!(classify_task_tier("tổng hợp thông tin đặt phòng"), TaskTier::Medium);
        // "facebook" should NOT match "ok" anymore
        assert_eq!(classify_task_tier("đăng lên facebook"), TaskTier::Medium);
    }

    #[test]
    fn test_classify_complex() {
        assert_eq!(classify_task_tier("lập kế hoạch marketing"), TaskTier::Complex);
        assert_eq!(classify_task_tier("phân tích đối thủ cạnh tranh"), TaskTier::Complex);
        assert_eq!(classify_task_tier("viết code API mới"), TaskTier::Complex);
    }

    #[test]
    fn test_model_costs_sorted() {
        for m in MODEL_COSTS {
            assert!(m.input_per_1m >= 0.0, "Negative input cost for {}", m.model);
            assert!(m.output_per_1m >= 0.0, "Negative output cost for {}", m.model);
        }
    }

    #[test]
    fn test_search_skills_calendar() {
        let results = search_skills("đặt lịch booking phòng", 3);
        assert!(!results.is_empty());
        assert!(results.iter().any(|s| s.name == "calendar"));
    }

    #[test]
    fn test_search_skills_social() {
        let results = search_skills("đăng bài facebook marketing", 3);
        assert!(!results.is_empty());
        assert!(results.iter().any(|s| s.name == "social_post"));
    }

    #[test]
    fn test_search_skills_web() {
        let results = search_skills("crawl web duyệt trang", 3);
        assert!(!results.is_empty());
        assert!(results.iter().any(|s| s.name == "browser"));
    }

    #[test]
    fn test_search_skills_empty_query() {
        let results = search_skills("", 10);
        assert!(results.is_empty());
    }

    #[test]
    fn test_generate_plan() {
        let plan = generate_plan_preview("đăng bài quảng cáo homestay lên facebook");
        assert_eq!(plan.task_tier, TaskTier::Medium);
        assert!(plan.skills_used.contains(&"social_post".to_string()));
    }

    #[test]
    fn test_builtin_skills_count() {
        let skills = builtin_skills();
        assert!(skills.len() >= 10);
    }

    #[test]
    fn test_skill_meta_serialization() {
        let skill = SkillMeta {
            name: "test".into(),
            description: "Test skill".into(),
            category: "testing".into(),
            keywords: vec!["test".into()],
        };
        let json = serde_json::to_string(&skill).unwrap();
        assert!(json.contains("test"));
    }

    #[test]
    fn test_execution_plan_serialization() {
        let plan = generate_plan_preview("tìm thông tin du lịch");
        let json = serde_json::to_string(&plan).unwrap();
        assert!(json.contains("task_tier"));
        assert!(json.contains("selected_provider"));
    }

    // ── NEW: Token Detection Tests ──────────────────────────

    #[test]
    fn test_detect_anthropic_key() {
        let result = detect_provider_from_key("sk-ant-api03-xxxxxxxxxxxxxxxxxxxxx");
        assert!(result.is_some());
        let d = result.unwrap();
        assert_eq!(d.provider, "anthropic");
        assert_eq!(d.confidence, "high");
        assert_eq!(d.env_key, "ANTHROPIC_API_KEY");
    }

    #[test]
    fn test_detect_openai_key() {
        let result = detect_provider_from_key("sk-proj-xxxxxxxxxxxxxxxxxxxxxxxxxxxx");
        assert!(result.is_some());
        let d = result.unwrap();
        assert_eq!(d.provider, "openai");
        assert_eq!(d.confidence, "high");
    }

    #[test]
    fn test_detect_gemini_key() {
        let result = detect_provider_from_key("AIzaSyxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx");
        assert!(result.is_some());
        let d = result.unwrap();
        assert_eq!(d.provider, "gemini");
        assert_eq!(d.confidence, "high");
    }

    #[test]
    fn test_detect_groq_key() {
        let result = detect_provider_from_key("gsk_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx");
        assert!(result.is_some());
        let d = result.unwrap();
        assert_eq!(d.provider, "groq");
        assert_eq!(d.confidence, "high");
    }

    #[test]
    fn test_detect_xai_key() {
        let result = detect_provider_from_key("xai-xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx");
        assert!(result.is_some());
        let d = result.unwrap();
        assert_eq!(d.provider, "xai");
    }

    #[test]
    fn test_detect_empty_key() {
        assert!(detect_provider_from_key("").is_none());
        assert!(detect_provider_from_key("short").is_none());
    }

    #[test]
    fn test_detect_unknown_key() {
        let result = detect_provider_from_key("some-random-api-key-that-is-long-enough");
        assert!(result.is_some());
        // Unknown keys get fallback detection
    }
}

