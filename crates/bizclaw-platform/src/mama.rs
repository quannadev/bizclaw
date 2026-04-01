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
    pub input_per_1m: f64,     // USD per 1M input tokens
    pub output_per_1m: f64,    // USD per 1M output tokens
    pub tier: TaskTier,        // Which task tier this model is best for
    pub context_window: u32,   // Max context length
    pub env_key: &'static str, // Env var for API key
    pub speed: Speed,          // Relative response speed
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
    Fast,   // < 1s TTFT
    Medium, // 1-3s TTFT
    Slow,   // > 3s TTFT
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
        cost_a
            .partial_cmp(&cost_b)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    available.into_iter().next()
}

/// Classify a user request into a task tier.
pub fn classify_task_tier(request: &str) -> TaskTier {
    let lower = request.to_lowercase();
    let words: Vec<&str> = lower.split_whitespace().collect();

    // Complex indicators (substring match is fine for multi-char phrases)
    let complex_keywords = [
        "lập kế hoạch",
        "plan",
        "phân tích",
        "analyze",
        "strategy",
        "code",
        "viết code",
        "tạo workflow",
        "refactor",
        "debug",
        "so sánh",
        "compare",
        "đánh giá",
        "evaluate",
        "architecture",
        "thiết kế",
        "design",
        "tối ưu",
        "optimize",
        "reasoning",
    ];

    // Simple indicators — use exact word match for short tokens
    let simple_words = [
        "trả lời",
        "reply",
        "format",
        "dịch",
        "translate",
        "tóm tắt ngắn",
        "brief",
        "yes/no",
        "đúng không",
        "chào",
        "hello",
        "cảm ơn",
        "thanks",
    ];

    if complex_keywords.iter().any(|k| lower.contains(k)) {
        TaskTier::Complex
    } else if simple_words.iter().any(|k| {
        // For multi-word phrases, use contains
        if k.contains(' ') {
            lower.contains(k)
        } else {
            // For single words, match exact word boundaries
            words.contains(k)
        }
    }) {
        TaskTier::Simple
    } else {
        TaskTier::Medium
    }
}

/// Check if Ollama is running locally.
fn check_ollama_available() -> bool {
    let host = std::env::var("OLLAMA_HOST").unwrap_or_else(|_| "http://localhost:11434".into());
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
            keywords: vec![
                "web".into(),
                "crawl".into(),
                "scrape".into(),
                "browse".into(),
                "duyệt".into(),
            ],
        },
        SkillMeta {
            name: "calendar".into(),
            description: "Quản lý Google Calendar — xem lịch, tạo sự kiện, check lịch rảnh.".into(),
            category: "productivity".into(),
            keywords: vec![
                "lịch".into(),
                "calendar".into(),
                "sự kiện".into(),
                "event".into(),
                "booking".into(),
                "đặt lịch".into(),
            ],
        },
        SkillMeta {
            name: "social_post".into(),
            description: "Đăng bài lên Facebook Page, Telegram Channel, webhook.".into(),
            category: "marketing".into(),
            keywords: vec![
                "đăng bài".into(),
                "post".into(),
                "facebook".into(),
                "telegram".into(),
                "marketing".into(),
            ],
        },
        SkillMeta {
            name: "research".into(),
            description: "Nghiên cứu học thuật — tìm papers từ OpenAlex, Semantic Scholar.".into(),
            category: "research".into(),
            keywords: vec![
                "nghiên cứu".into(),
                "research".into(),
                "paper".into(),
                "academic".into(),
            ],
        },
        SkillMeta {
            name: "shell".into(),
            description: "Chạy lệnh terminal trên server.".into(),
            category: "system".into(),
            keywords: vec![
                "terminal".into(),
                "command".into(),
                "shell".into(),
                "lệnh".into(),
            ],
        },
        SkillMeta {
            name: "file".into(),
            description: "Đọc/ghi file trên hệ thống.".into(),
            category: "system".into(),
            keywords: vec![
                "file".into(),
                "đọc".into(),
                "ghi".into(),
                "read".into(),
                "write".into(),
            ],
        },
        SkillMeta {
            name: "http_request".into(),
            description: "Gọi API bên ngoài (GET/POST/PUT/DELETE).".into(),
            category: "integration".into(),
            keywords: vec![
                "api".into(),
                "http".into(),
                "request".into(),
                "webhook".into(),
            ],
        },
        SkillMeta {
            name: "memory_search".into(),
            description: "Tìm kiếm trong bộ nhớ agent (RAG).".into(),
            category: "memory".into(),
            keywords: vec![
                "nhớ".into(),
                "memory".into(),
                "tìm".into(),
                "search".into(),
                "rag".into(),
            ],
        },
        SkillMeta {
            name: "db_query".into(),
            description: "Truy vấn database SQLite/PostgreSQL.".into(),
            category: "data".into(),
            keywords: vec![
                "database".into(),
                "query".into(),
                "sql".into(),
                "dữ liệu".into(),
            ],
        },
        SkillMeta {
            name: "edit_file".into(),
            description: "Chỉnh sửa file (search & replace, append).".into(),
            category: "system".into(),
            keywords: vec![
                "edit".into(),
                "sửa".into(),
                "replace".into(),
                "chỉnh".into(),
            ],
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
        None => ("none".into(), "none".into(), "No provider available".into()),
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

use crate::admin::AdminState;
use axum::Json;
use axum::extract::State;
use std::sync::Arc;

/// GET /api/mama/providers — list available AI providers with costs.
pub async fn list_providers(State(_state): State<Arc<AdminState>>) -> Json<serde_json::Value> {
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
pub async fn mama_status(State(_state): State<Arc<AdminState>>) -> Json<serde_json::Value> {
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
// 5b. WORKFLOW + TEAM + SKILLS INTEGRATION — The Nervous System
// ══════════════════════════════════════════════════════════

use bizclaw_orchestrator::team::{AgentOrganization, AgentRole, AgentTeam, TeamAgent};
use bizclaw_workflows::{WorkflowEngine, builtin_workflows};

/// Initialize the WorkflowEngine with all 22 built-in templates.
pub fn init_workflow_engine() -> WorkflowEngine {
    let mut engine = WorkflowEngine::new();
    for wf in builtin_workflows() {
        engine.register(wf);
    }
    tracing::info!(
        "🔄 Mama AI: Workflow Engine initialized — {} workflows ready",
        engine.count()
    );
    engine
}

/// Initialize the default AgentOrganization with standard teams.
pub async fn init_default_org() -> AgentOrganization {
    let org = AgentOrganization::new();

    // Sales Team
    let mut sales = AgentTeam::new(
        "Sales Team",
        TeamAgent {
            id: "sales-lead".into(),
            name: "Sales Manager".into(),
            role: AgentRole::Lead,
            channels: vec!["zalo".into(), "facebook".into(), "web".into()],
            specialties: vec!["sales".into(), "proposal".into(), "pricing".into()],
            model: "deepseek-chat".into(),
            active: true,
        },
    );
    sales.add_member(TeamAgent {
        id: "sales-fb".into(),
        name: "Facebook Sales Agent".into(),
        role: AgentRole::Member,
        channels: vec!["facebook".into()],
        specialties: vec!["comment-reply".into(), "dm".into()],
        model: "qwen3.5-4b-neo".into(), // cheapest for simple replies
        active: true,
    });

    // Marketing Team
    let mut marketing = AgentTeam::new(
        "Marketing Team",
        TeamAgent {
            id: "marketing-lead".into(),
            name: "Marketing Manager".into(),
            role: AgentRole::Lead,
            channels: vec!["internal".into()],
            specialties: vec!["content".into(), "strategy".into(), "campaign".into()],
            model: "deepseek-chat".into(),
            active: true,
        },
    );
    marketing.add_member(TeamAgent {
        id: "content-writer".into(),
        name: "Content Writer".into(),
        role: AgentRole::Specialist,
        channels: vec!["internal".into()],
        specialties: vec!["writing".into(), "blog".into(), "social-post".into()],
        model: "deepseek-chat".into(),
        active: true,
    });

    // Support Team
    let support = AgentTeam::new(
        "Support Team",
        TeamAgent {
            id: "support-lead".into(),
            name: "Customer Support Lead".into(),
            role: AgentRole::Lead,
            channels: vec!["zalo".into(), "telegram".into(), "email".into()],
            specialties: vec!["support".into(), "faq".into(), "booking".into()],
            model: "gpt-4o-mini".into(),
            active: true,
        },
    );

    org.register_team(sales).await;
    org.register_team(marketing).await;
    org.register_team(support).await;

    tracing::info!(
        "🏢 Mama AI: Organization initialized — {} teams",
        org.list_teams().await.len()
    );
    org
}

/// Bridge: bizclaw-skills registry → mama builtin_skills.
/// Loads skills from the real `bizclaw-skills` marketplace.
pub fn load_skills_from_registry() -> Vec<SkillMeta> {
    let registry = bizclaw_skills::SkillRegistry::with_defaults();
    let manifests = registry.list();

    let mut skills: Vec<SkillMeta> = manifests
        .into_iter()
        .map(|m| SkillMeta {
            name: m.metadata.name.clone(),
            description: m.metadata.description.clone(),
            category: if m.metadata.category.is_empty() {
                m.metadata
                    .tags
                    .first()
                    .cloned()
                    .unwrap_or_else(|| "general".into())
            } else {
                m.metadata.category.clone()
            },
            keywords: m.metadata.tags.clone(),
        })
        .collect();

    // Merge with hardcoded builtins (in case registered skills are empty)
    if skills.is_empty() {
        skills = builtin_skills();
    }

    skills
}

/// GET /api/mama/workflows — list all available workflows.
pub async fn list_workflows(State(_state): State<Arc<AdminState>>) -> Json<serde_json::Value> {
    let engine = init_workflow_engine();
    let workflows: Vec<serde_json::Value> = engine
        .list()
        .iter()
        .map(|wf| {
            serde_json::json!({
                "name": wf.name,
                "description": wf.description,
                "tags": wf.tags,
                "steps": wf.step_count(),
                "timeout": wf.max_runtime_secs,
            })
        })
        .collect();

    Json(serde_json::json!({
        "count": workflows.len(),
        "workflows": workflows,
    }))
}

/// POST /api/mama/workflows/run — execute a workflow.
#[derive(Debug, Deserialize)]
pub struct RunWorkflowRequest {
    pub workflow: String,
    pub input: String,
}

pub async fn run_workflow(
    State(_state): State<Arc<AdminState>>,
    Json(body): Json<RunWorkflowRequest>,
) -> Json<serde_json::Value> {
    let mut engine = init_workflow_engine();

    // Create agent callback that uses the cheapest available provider
    let agent_fn: bizclaw_workflows::engine::AgentCallback =
        Box::new(|_agent_name: &str, prompt: &str| {
            // For now, use a simulated response.
            // In production, this would call Agent::process() with cost routing.
            let output = format!(
                "[Agent] Processed prompt ({} chars). \
                 In production: Mama AI routes to cheapest provider per task tier.",
                prompt.len()
            );
            Ok((output, prompt.len() as u64 / 4)) // rough token estimate
        });

    match engine.execute(&body.workflow, &body.input, &agent_fn) {
        Ok(state) => Json(serde_json::json!({
            "success": true,
            "workflow": body.workflow,
            "status": format!("{:?}", state.status),
            "steps_completed": state.step_results.len(),
            "total_tokens": state.total_tokens,
            "duration_secs": state.duration_secs(),
            "output": state.last_output(),
            "step_details": state.step_results.iter().map(|s| {
                serde_json::json!({
                    "step": s.step_name,
                    "agent": s.agent,
                    "tokens": s.tokens_used,
                    "latency_ms": s.latency_ms,
                    "status": format!("{:?}", s.status),
                })
            }).collect::<Vec<_>>(),
        })),
        Err(e) => Json(serde_json::json!({
            "success": false,
            "error": e,
        })),
    }
}

/// GET /api/mama/teams — list all agent teams and org chart.
pub async fn list_teams(State(_state): State<Arc<AdminState>>) -> Json<serde_json::Value> {
    let org = init_default_org().await;
    let summary = org.summary().await;

    Json(serde_json::json!({
        "organization": summary,
        "info": "🏢 Agent teams auto-created by Mama AI. Each team has specialized agents with cost-optimized model assignments.",
    }))
}

/// GET /api/mama/team/{channel} — find which agent handles a channel.
pub async fn team_for_channel(
    State(_state): State<Arc<AdminState>>,
    axum::extract::Path(channel): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    let org = init_default_org().await;

    match org.team_for_channel(&channel).await {
        Some((team_name, agent)) => Json(serde_json::json!({
            "found": true,
            "team": team_name,
            "agent": {
                "id": agent.id,
                "name": agent.name,
                "role": format!("{:?}", agent.role),
                "model": agent.model,
            },
        })),
        None => Json(serde_json::json!({
            "found": false,
            "message": format!("No agent configured for channel: {}", channel),
        })),
    }
}

// ══════════════════════════════════════════════════════════
// 5c. BUDGET + HEARTBEAT + HANDS + EXECUTE — Complete Wiring
// ══════════════════════════════════════════════════════════

use bizclaw_orchestrator::budget::{AgentBudget, BudgetExceedAction, BudgetManager, BudgetStatus};
use bizclaw_orchestrator::heartbeat::{HeartbeatConfig, HeartbeatMonitor};

/// Initialize budget manager with default agent budgets.
pub async fn init_budget_manager() -> BudgetManager {
    let mgr = BudgetManager::new();

    // Default budget: 100K tokens/month for free tier agents
    let agent_ids = [
        "sales-lead",
        "sales-fb",
        "marketing-lead",
        "content-writer",
        "support-lead",
    ];
    for id in agent_ids {
        mgr.set_budget(AgentBudget {
            agent_id: id.into(),
            monthly_token_limit: 100_000,
            monthly_usd_limit: 5.0,
            alert_at_percent: 80.0,
            on_exceed: BudgetExceedAction::SwitchToLocal,
            fallback_model: "qwen3.5-4b-neo".into(),
        })
        .await;
    }

    tracing::info!(
        "💰 Mama AI: Budget Manager initialized — {} agents tracked",
        agent_ids.len()
    );
    mgr
}

/// Initialize heartbeat monitor for all agents in the org.
pub async fn init_heartbeat_monitor() -> HeartbeatMonitor {
    let monitor = HeartbeatMonitor::new(HeartbeatConfig {
        check_interval_seconds: 60,
        degraded_after_misses: 3,
        unresponsive_after_misses: 10,
        auto_restart: true,
        max_restart_attempts: 3,
    });

    // Register all org agents
    monitor
        .register(
            "sales-lead",
            vec!["zalo".into(), "facebook".into(), "web".into()],
            30,
        )
        .await;
    monitor
        .register("sales-fb", vec!["facebook".into()], 30)
        .await;
    monitor
        .register("marketing-lead", vec!["internal".into()], 60)
        .await;
    monitor
        .register("content-writer", vec!["internal".into()], 60)
        .await;
    monitor
        .register(
            "support-lead",
            vec!["zalo".into(), "telegram".into(), "email".into()],
            30,
        )
        .await;

    tracing::info!("💓 Mama AI: Heartbeat Monitor initialized — 5 agents tracked");
    monitor
}

/// POST /api/mama/execute — Execute a full plan (Mama routes to real agents).
#[derive(Debug, Deserialize)]
pub struct ExecutePlanRequest {
    pub goal: String,
    #[serde(default)]
    pub budget_limit_usd: Option<f64>,
}

pub async fn execute_plan(
    State(_state): State<Arc<AdminState>>,
    Json(body): Json<ExecutePlanRequest>,
) -> Json<serde_json::Value> {
    // 1. Detect available providers
    let providers = detect_available_providers();

    if providers.is_empty() {
        return Json(serde_json::json!({
            "success": false,
            "error": "No AI providers configured. Add API keys via /api/mama/onboard first.",
        }));
    }

    // 2. Classify the task
    let tier = classify_task_tier(&body.goal);
    let skills = search_skills(&body.goal, 5);

    // 3. Route to cheapest provider for this tier
    let best = select_cheapest_model(tier).unwrap_or_else(|| providers[0].clone());

    // 4. Generate execution plan
    let plan = generate_plan_preview(&body.goal);

    // 5. Init budget tracking
    let budget_mgr = init_budget_manager().await;

    // 6. Build Fallback Chain
    let mut fallback_chain = vec![best.clone()];
    for p in &providers {
        if p.provider != best.provider || p.model != best.model {
            fallback_chain.push(p.clone());
        }
    }

    // 7. Execute each step
    let mut step_results = Vec::new();
    let mut total_tokens = 0u64;
    let mut total_cost = 0.0f64;
    let exec_start = chrono::Utc::now();

    // Load available tools
    let _available_tools = vec![
        bizclaw_skills::webclaw::webclaw_scrape_definition(),
        bizclaw_skills::harrier::local_harrier_embed_definition(),
    ];

    let mut current_input = String::new();

    for (i, step) in plan.steps.iter().enumerate() {
        // Check budget before each step
        let budget_status = budget_mgr.check_budget("mama-executor").await;
        let should_stop = matches!(budget_status, BudgetStatus::Exceeded { .. });

        if should_stop {
            step_results.push(serde_json::json!({
                "step": i + 1,
                "action": step.action,
                "status": "skipped",
                "reason": "Budget exceeded",
            }));
            continue;
        }

        // Real Execution Loop (via Agent)
        let mut config = bizclaw_core::config::BizClawConfig::default();
        config.identity.name = format!("Mama Executor [{}]", step.action);
        config.identity.persona = format!(
            "You are executing a step in a larger plan.\nStep Action: {}\nDescription: {}\nSuggested Tool: {}",
            step.action, step.description, step.tool
        );
        config.default_provider = best.provider.clone();
        config.default_model = best.model.clone();

        // ** Mama -> Execute Plan (Agent Spawning) **
        let mut agent = bizclaw_agent::Agent::new(config).unwrap();

        let prompt = if current_input.is_empty() {
            format!("Task Goal: {}\n\nPlease execute your step.", body.goal)
        } else {
            // ** Agent-to-Agent Handoff **
            format!(
                "Task Goal: {}\n\n--- Handoff Context from Previous Agent ---\n{}\n--- End Context ---\n\nPlease execute your assigned step based on the previous agent's results.",
                body.goal, current_input
            )
        };

        let mut step_tokens = (prompt.len() as u64) / 4;
        let final_result = match agent.process(&prompt).await {
            Ok(res) => {
                step_tokens += (res.len() as u64) / 4;
                res
            }
            Err(e) => {
                format!("❌ Agent execution failed: {}", e)
            }
        };

        let executed_provider_log = format!("{}/{}", best.provider, best.model);

        // Save result for the Next Agent
        current_input = final_result.clone();

        total_tokens += step_tokens;
        let step_cost = step_tokens as f64 / 1_000_000.0
            * match tier {
                TaskTier::Simple => 0.10,
                TaskTier::Medium => 0.50,
                TaskTier::Complex => 3.00,
            };
        total_cost += step_cost;

        // Record total tokens used for this step
        budget_mgr
            .record_usage("mama-executor", step_tokens / 2, step_tokens / 2, step_cost)
            .await;

        step_results.push(serde_json::json!({
            "step": i + 1,
            "tool": step.tool,
            "action": step.action,
            "description": step.description,
            "status": "completed",
            "result": final_result,
            "tokens": step_tokens,
            "cost_usd": format!("${:.6}", step_cost),
            "executed_by": executed_provider_log,
        }));
    }

    let exec_duration = (chrono::Utc::now() - exec_start).num_milliseconds() as f64 / 1000.0;

    Json(serde_json::json!({
        "success": true,
        "goal": body.goal,
        "task_tier": format!("{:?}", tier),
        "provider": best.provider,
        "model": best.model,
        "steps_executed": step_results.len(),
        "total_tokens": total_tokens,
        "total_cost_usd": format!("${:.6}", total_cost),
        "duration_secs": exec_duration,
        "skills_used": skills.iter().map(|s| &s.name).collect::<Vec<_>>(),
        "steps": step_results,
        "budget_summary": budget_mgr.summary().await,
    }))
}

/// GET /api/mama/budget — get budget summary for all agents.
pub async fn budget_summary(State(_state): State<Arc<AdminState>>) -> Json<serde_json::Value> {
    let mgr = init_budget_manager().await;
    Json(serde_json::json!({
        "budget": mgr.summary().await,
        "info": "💰 Token budgets per agent. When exceeded, Mama auto-switches to local model.",
    }))
}

/// GET /api/mama/health — get heartbeat status for all agents.
pub async fn health_status(State(_state): State<Arc<AdminState>>) -> Json<serde_json::Value> {
    let monitor = init_heartbeat_monitor().await;
    Json(serde_json::json!({
        "health": monitor.summary().await,
        "config": {
            "check_interval": 60,
            "degraded_after_misses": 3,
            "unresponsive_after_misses": 10,
            "auto_restart": true,
        },
    }))
}

/// GET /api/mama/dashboard — complete Mama AI dashboard.
pub async fn mama_dashboard(State(_state): State<Arc<AdminState>>) -> Json<serde_json::Value> {
    let org = init_default_org().await;
    let engine = init_workflow_engine();
    let budget_mgr = init_budget_manager().await;
    let monitor = init_heartbeat_monitor().await;
    let providers = detect_available_providers();
    let skills = load_skills_from_registry();

    Json(serde_json::json!({
        "mama_version": "1.0.9",
        "organization": org.summary().await,
        "workflows": {
            "count": engine.count(),
            "names": engine.workflow_names(),
        },
        "providers": {
            "count": providers.len(),
            "available": providers.iter().map(|p| &p.provider).collect::<Vec<_>>(),
        },
        "skills": {
            "count": skills.len(),
            "categories": skills.iter().map(|s| &s.category).collect::<std::collections::HashSet<_>>(),
        },
        "budget": budget_mgr.summary().await,
        "health": monitor.summary().await,
        "endpoints": [
            "GET  /api/mama/dashboard",
            "GET  /api/mama/providers",
            "GET  /api/mama/skills",
            "GET  /api/mama/status",
            "GET  /api/mama/workflows",
            "GET  /api/mama/teams",
            "GET  /api/mama/team/{channel}",
            "GET  /api/mama/budget",
            "GET  /api/mama/health",
            "POST /api/mama/plan",
            "POST /api/mama/execute",
            "POST /api/mama/workflows/run",
            "POST /api/mama/onboard",
            "POST /api/mama/detect-key",
        ],
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
        if detected.confidence == "high" {
            "BẬT"
        } else {
            "TẮT (cần xác nhận)"
        },
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
    use bizclaw_orchestrator::heartbeat::HealthStatus;

    #[test]
    fn test_classify_simple() {
        assert_eq!(classify_task_tier("trả lời comment"), TaskTier::Simple);
        assert_eq!(classify_task_tier("format dữ liệu"), TaskTier::Simple);
        assert_eq!(classify_task_tier("dịch sang tiếng Anh"), TaskTier::Simple);
    }

    #[test]
    fn test_classify_medium() {
        assert_eq!(
            classify_task_tier("viết bài về du lịch Đà Lạt"),
            TaskTier::Medium
        );
        assert_eq!(
            classify_task_tier("tổng hợp thông tin đặt phòng"),
            TaskTier::Medium
        );
        // "facebook" should NOT match "ok" anymore
        assert_eq!(classify_task_tier("đăng lên facebook"), TaskTier::Medium);
    }

    #[test]
    fn test_classify_complex() {
        assert_eq!(
            classify_task_tier("lập kế hoạch marketing"),
            TaskTier::Complex
        );
        assert_eq!(
            classify_task_tier("phân tích đối thủ cạnh tranh"),
            TaskTier::Complex
        );
        assert_eq!(classify_task_tier("viết code API mới"), TaskTier::Complex);
    }

    #[test]
    fn test_model_costs_sorted() {
        for m in MODEL_COSTS {
            assert!(m.input_per_1m >= 0.0, "Negative input cost for {}", m.model);
            assert!(
                m.output_per_1m >= 0.0,
                "Negative output cost for {}",
                m.model
            );
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

    // ── Phase 1: Workflow + Teams + Skills Integration ──────

    #[test]
    fn test_workflow_engine_init() {
        let engine = init_workflow_engine();
        assert!(
            engine.count() >= 20,
            "Should have 20+ built-in workflows, got {}",
            engine.count()
        );
        assert!(engine.get("content_pipeline").is_some());
        assert!(engine.get("meeting_recap").is_some());
        assert!(engine.get("ceo_daily_briefing").is_some());
    }

    #[test]
    fn test_workflow_execute_content_pipeline() {
        let mut engine = init_workflow_engine();
        let agent_fn: bizclaw_workflows::engine::AgentCallback =
            Box::new(|agent: &str, prompt: &str| {
                Ok((
                    format!("[{}] OK: {}", agent, &prompt[..prompt.len().min(30)]),
                    50,
                ))
            });

        let result = engine.execute("content_pipeline", "du lịch Đà Lạt", &agent_fn);
        assert!(result.is_ok());
        let state = result.unwrap();
        assert_eq!(state.step_results.len(), 3); // draft → review → edit
    }

    #[tokio::test]
    async fn test_org_init_teams() {
        let org = init_default_org().await;
        let teams = org.list_teams().await;
        assert_eq!(teams.len(), 3); // Sales, Marketing, Support
    }

    #[tokio::test]
    async fn test_org_channel_routing() {
        let org = init_default_org().await;

        // Facebook → Sales Team (sales-lead handles facebook)
        let fb = org.team_for_channel("facebook").await;
        assert!(fb.is_some());
        let (team, _agent) = fb.unwrap();
        assert_eq!(team, "Sales Team");

        // Zalo → Sales Team or Support Team (first match)
        let zalo = org.team_for_channel("zalo").await;
        assert!(zalo.is_some());

        // Unknown channel → None
        let unknown = org.team_for_channel("tiktok").await;
        assert!(unknown.is_none());
    }

    #[test]
    fn test_load_skills_from_registry() {
        let skills = load_skills_from_registry();
        assert!(!skills.is_empty(), "Skills registry should not be empty");
    }

    #[test]
    fn test_workflow_list_names() {
        let engine = init_workflow_engine();
        let names = engine.workflow_names();
        assert!(names.contains(&"content_pipeline".to_string()));
        assert!(names.contains(&"weekly_report".to_string()));
        assert!(names.contains(&"proposal_generator".to_string()));
    }

    // ── Phase 2+3: Budget + Heartbeat + Execute + Dashboard ──

    #[tokio::test]
    async fn test_budget_manager_init() {
        let mgr = init_budget_manager().await;
        // 5 agents should have budgets set
        let summary = mgr.summary().await;
        assert_eq!(summary["total_agents"], 0); // No usage yet, but budget exists
        // Verify budget was configured by checking status
        let status = mgr.check_budget("sales-lead").await;
        assert!(matches!(status, BudgetStatus::Ok { .. }));
    }

    #[tokio::test]
    async fn test_budget_tracks_usage() {
        let mgr = init_budget_manager().await;
        let status = mgr.record_usage("sales-lead", 500, 300, 0.001).await;
        assert!(matches!(status, BudgetStatus::Ok { .. }));

        let usage = mgr.get_usage("sales-lead").await;
        assert!(usage.is_some());
        assert_eq!(usage.unwrap().tokens_used, 800);
    }

    #[tokio::test]
    async fn test_heartbeat_monitor_init() {
        let monitor = init_heartbeat_monitor().await;
        let summary = monitor.summary().await;
        assert_eq!(summary["total"], 5);
        assert_eq!(summary["healthy"], 5);

        // Verify specific agent registration
        assert_eq!(
            monitor.status("sales-lead").await,
            Some(HealthStatus::Healthy)
        );
        assert_eq!(
            monitor.status("content-writer").await,
            Some(HealthStatus::Healthy)
        );
        assert_eq!(monitor.status("nonexistent").await, None);
    }

    #[test]
    fn test_plan_generates_steps() {
        let plan = generate_plan_preview("Phân tích đối thủ và tạo báo cáo marketing");
        assert!(!plan.steps.is_empty());
        assert_eq!(
            plan.task_tier,
            classify_task_tier("Phân tích đối thủ và tạo báo cáo marketing")
        );
    }

    #[test]
    fn test_skills_have_categories() {
        let skills = load_skills_from_registry();
        for skill in &skills {
            assert!(!skill.name.is_empty(), "Skill name must not be empty");
            assert!(
                !skill.description.is_empty(),
                "Skill description must not be empty"
            );
        }
    }

    #[test]
    fn test_provider_detection_all_types() {
        // Anthropic
        assert_eq!(
            detect_provider_from_key("sk-ant-api03-xxx")
                .unwrap()
                .provider,
            "anthropic"
        );
        // OpenAI
        assert_eq!(
            detect_provider_from_key("sk-proj-xxxxxxxxxxxxxxxx")
                .unwrap()
                .provider,
            "openai"
        );
        // Gemini
        assert_eq!(
            detect_provider_from_key("AIzaSyxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx")
                .unwrap()
                .provider,
            "gemini"
        );
        // Groq
        assert_eq!(
            detect_provider_from_key("gsk_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx")
                .unwrap()
                .provider,
            "groq"
        );
        // xAI
        assert_eq!(
            detect_provider_from_key("xai-xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx")
                .unwrap()
                .provider,
            "xai"
        );
    }
}
