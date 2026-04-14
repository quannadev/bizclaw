//! # BizClaw Providers
//!
//! LLM provider implementations for BizClaw.
//!
//! All OpenAI-compatible providers (OpenAI, Anthropic, DeepSeek, Gemini, Groq,
//! Ollama, LlamaCpp, OpenRouter) are handled by a single `OpenAiCompatibleProvider`.
//! The `BrainProvider` handles local GGUF models separately.

pub mod brain;
pub mod failover;
pub mod llm_tracing;
pub mod openai_compatible;
pub mod provider_registry;
pub mod text_tool_calls;

use bizclaw_core::config::BizClawConfig;
use bizclaw_core::error::{BizClawError, Result};
use bizclaw_core::traits::Provider;

/// Create a provider from configuration.
///
/// Resolution order for provider name:
/// 1. `config.llm.provider` (from `[LLM]` section)
/// 2. `config.default_provider` (legacy top-level field)
///
/// If `config.llm.fallback_provider` is set, wraps in FailoverProvider
/// for automatic failover (CAS state machine: Healthy → Degraded → Unhealthy).
pub fn create_provider(config: &BizClawConfig) -> Result<Box<dyn Provider>> {
    // Prefer [LLM] section, fallback to legacy top-level field
    let provider_name = if !config.llm.provider.is_empty() {
        config.llm.provider.as_str()
    } else {
        config.default_provider.as_str()
    };

    let primary = create_single_provider(provider_name, config)?;

    // If fallback_provider is configured, wrap in FailoverProvider
    let fallback_name = &config.llm.fallback_provider;
    if !fallback_name.is_empty() && fallback_name != provider_name {
        match create_single_provider(fallback_name, config) {
            Ok(fallback) => {
                tracing::info!(
                    "🔄 Failover chain: {} → {} (CAS state machine active)",
                    provider_name,
                    fallback_name
                );
                return Ok(Box::new(
                    failover::FailoverProvider::with_fallback(primary, fallback),
                ));
            }
            Err(e) => {
                tracing::warn!(
                    "⚠️ Fallback provider '{}' failed to init: {} — running without failover",
                    fallback_name,
                    e
                );
            }
        }
    }

    Ok(primary)
}

/// Create a single provider instance by name (internal helper).
fn create_single_provider(name: &str, config: &BizClawConfig) -> Result<Box<dyn Provider>> {
    match name {
        // Local GGUF engine — not OpenAI-compatible
        "brain" => Ok(Box::new(brain::BrainProvider::new(config)?)),

        // Custom endpoint: "custom:https://my-server.com/v1"
        other if other.starts_with("custom:") => Ok(Box::new(
            openai_compatible::OpenAiCompatibleProvider::custom(other, config)?,
        )),

        // All known OpenAI-compatible providers
        _ => {
            let registry = provider_registry::get_provider_config(name)
                .ok_or_else(|| BizClawError::ProviderNotFound(name.into()))?;
            Ok(Box::new(
                openai_compatible::OpenAiCompatibleProvider::from_registry(registry, config)?,
            ))
        }
    }
}

/// List all available provider names.
pub fn available_providers() -> Vec<&'static str> {
    let mut names = provider_registry::all_provider_names();
    names.push("brain");
    names.push("custom");
    names
}
