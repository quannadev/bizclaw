//! Smart routing — decide local vs cloud LLM based on task characteristics.
//!
//! For Vietnamese SME with BYO keys, this module helps minimize cloud API costs
//! by routing simple tasks to local Ollama and complex tasks to cloud providers.

/// Task complexity assessment result.
#[derive(Debug, Clone, PartialEq)]
pub enum RouteDecision {
    /// Use local model (Ollama, llama.cpp) — fast, zero cost
    Local,
    /// Use cloud provider (OpenAI, Anthropic, etc.) — higher quality, costs money
    Cloud,
}

/// Configuration for smart routing behavior.
#[derive(Debug, Clone)]
pub struct RoutingConfig {
    /// Maximum token count for local routing (default: 500)
    pub local_max_tokens: usize,
    /// Whether local model is available (Ollama running)
    pub local_available: bool,
    /// Keywords that force cloud routing (complex tasks)
    pub cloud_keywords: Vec<String>,
    /// Force all requests to cloud (disable smart routing)
    pub force_cloud: bool,
}

impl Default for RoutingConfig {
    fn default() -> Self {
        Self {
            local_max_tokens: 500,
            local_available: false,
            cloud_keywords: vec![
                // Vietnamese complex task keywords
                "phân tích".into(),
                "báo cáo".into(),
                "tổng hợp".into(),
                "so sánh".into(),
                "chiến lược".into(),
                "kế hoạch".into(),
                // English complex task keywords
                "analyze".into(),
                "summarize".into(),
                "compare".into(),
                "strategy".into(),
                "report".into(),
                "translate".into(),
            ],
            force_cloud: false,
        }
    }
}

/// Determine whether a message should be routed to local or cloud model.
///
/// # Logic
/// 1. If `force_cloud` → always Cloud
/// 2. If local not available → always Cloud
/// 3. If message contains complex task keywords → Cloud
/// 4. If message length > threshold → Cloud (complex context)
/// 5. Otherwise → Local (simple chit-chat, FAQ, greetings)
pub fn route_message(message: &str, config: &RoutingConfig) -> RouteDecision {
    // Force cloud if configured
    if config.force_cloud {
        return RouteDecision::Cloud;
    }

    // No local model available
    if !config.local_available {
        return RouteDecision::Cloud;
    }

    let msg_lower = message.to_lowercase();

    // Check for complex task keywords
    for keyword in &config.cloud_keywords {
        if msg_lower.contains(&keyword.to_lowercase()) {
            return RouteDecision::Cloud;
        }
    }

    // Long messages likely need more capable model
    // Vietnamese text: ~2 chars per token on average
    let estimated_tokens = message.len() / 2;
    if estimated_tokens > config.local_max_tokens {
        return RouteDecision::Cloud;
    }

    // Simple messages → route to local
    RouteDecision::Local
}

/// Check if Ollama is running locally by probing the default endpoint.
pub async fn check_ollama_available() -> bool {
    // Try to connect to Ollama default endpoint
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(2))
        .build();

    match client {
        Ok(c) => {
            let resp = c.get("http://127.0.0.1:11434/api/version").send().await;
            resp.is_ok()
        }
        Err(_) => false,
    }
}

/// Resolve the best provider/model for a given message.
///
/// Returns (provider, model) tuple.
pub fn resolve_provider(
    message: &str,
    default_provider: &str,
    default_model: &str,
    config: &RoutingConfig,
) -> (String, String) {
    match route_message(message, config) {
        RouteDecision::Local => ("ollama".into(), "qwen2.5:3b".into()),
        RouteDecision::Cloud => (default_provider.into(), default_model.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> RoutingConfig {
        RoutingConfig {
            local_available: true,
            ..Default::default()
        }
    }

    #[test]
    fn test_simple_greeting_routes_local() {
        let config = test_config();
        assert_eq!(route_message("Xin chào!", &config), RouteDecision::Local);
        assert_eq!(route_message("hello", &config), RouteDecision::Local);
        assert_eq!(
            route_message("giá phòng bao nhiêu?", &config),
            RouteDecision::Local
        );
    }

    #[test]
    fn test_complex_task_routes_cloud() {
        let config = test_config();
        assert_eq!(
            route_message(
                "Hãy phân tích doanh thu quý 4 và đề xuất chiến lược",
                &config
            ),
            RouteDecision::Cloud
        );
        assert_eq!(
            route_message("Tổng hợp báo cáo từ các chi nhánh", &config),
            RouteDecision::Cloud
        );
    }

    #[test]
    fn test_long_message_routes_cloud() {
        let config = test_config();
        let long_msg = "a".repeat(1200); // ~600 tokens
        assert_eq!(route_message(&long_msg, &config), RouteDecision::Cloud);
    }

    #[test]
    fn test_force_cloud() {
        let config = RoutingConfig {
            force_cloud: true,
            local_available: true,
            ..Default::default()
        };
        assert_eq!(route_message("hi", &config), RouteDecision::Cloud);
    }

    #[test]
    fn test_no_local_available() {
        let config = RoutingConfig {
            local_available: false,
            ..Default::default()
        };
        assert_eq!(route_message("hi", &config), RouteDecision::Cloud);
    }

    #[test]
    fn test_resolve_provider_local() {
        let config = test_config();
        let (p, m) = resolve_provider("chào bạn", "openai", "gpt-4o-mini", &config);
        assert_eq!(p, "ollama");
        assert_eq!(m, "qwen2.5:3b");
    }

    #[test]
    fn test_resolve_provider_cloud() {
        let config = test_config();
        let (p, m) = resolve_provider(
            "phân tích doanh thu chi tiết",
            "openai",
            "gpt-4o-mini",
            &config,
        );
        assert_eq!(p, "openai");
        assert_eq!(m, "gpt-4o-mini");
    }

    #[test]
    fn test_vietnamese_keywords() {
        let config = test_config();
        assert_eq!(
            route_message("so sánh giá đối thủ", &config),
            RouteDecision::Cloud
        );
        assert_eq!(
            route_message("lập kế hoạch marketing", &config),
            RouteDecision::Cloud
        );
        assert_eq!(route_message("menu có gì?", &config), RouteDecision::Local);
    }
}
