//! WebAuth provider trait and registry.
//!
//! Each web chat platform (Gemini, Claude, ChatGPT, etc.) implements
//! the `WebProvider` trait. Providers handle template capture, message
//! sending, and response parsing.

pub mod chatgpt;
pub mod claude;
pub mod deepseek;
pub mod gemini;
pub mod grok;

use async_trait::async_trait;

use crate::cdp::CdpClient;
use crate::types::{AuthCheckResult, WebAuthModel};

/// Trait that every WebAuth provider must implement.
#[async_trait]
pub trait WebProvider: Send + Sync {
    /// Provider identifier (e.g., "gemini", "claude", "chatgpt")
    fn id(&self) -> &str;

    /// Provider display name
    fn name(&self) -> &str;

    /// URL the user needs to visit to log in
    fn login_url(&self) -> &str;

    /// Models exposed by this provider
    fn models(&self) -> &[WebAuthModel];

    /// Check if the session is still valid
    async fn check_auth(&self, cdp: &CdpClient) -> AuthCheckResult;

    /// Execute a chat completion.
    ///
    /// Takes a single text prompt (already consolidated from messages)
    /// and returns the model's response text.
    async fn chat(&self, cdp: &CdpClient, prompt: &str) -> Result<String, String>;

    /// Initialize the provider (capture template, etc.)
    /// Called once after authentication is confirmed.
    async fn initialize(&self, cdp: &CdpClient) -> Result<(), String> {
        // Default: no-op
        let _ = cdp;
        Ok(())
    }
}

/// Create all available providers.
pub fn create_all_providers() -> Vec<Box<dyn WebProvider>> {
    vec![
        Box::new(gemini::GeminiWebProvider::new()),
        Box::new(claude::ClaudeWebProvider::new()),
        Box::new(chatgpt::ChatGPTWebProvider::new()),
        Box::new(deepseek::DeepSeekWebProvider::new()),
        Box::new(grok::GrokWebProvider::new()),
    ]
}

/// Find a provider by model ID.
pub fn find_provider_for_model<'a>(
    providers: &'a [Box<dyn WebProvider>],
    model_id: &str,
) -> Option<&'a dyn WebProvider> {
    for provider in providers {
        if provider.models().iter().any(|m| m.id == model_id) {
            return Some(provider.as_ref());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_all_providers() {
        let providers = create_all_providers();
        assert_eq!(providers.len(), 5);

        let ids: Vec<&str> = providers.iter().map(|p| p.id()).collect();
        assert!(ids.contains(&"gemini"));
        assert!(ids.contains(&"claude"));
        assert!(ids.contains(&"chatgpt"));
        assert!(ids.contains(&"deepseek"));
        assert!(ids.contains(&"grok"));
    }

    #[test]
    fn test_find_provider_for_model() {
        let providers = create_all_providers();
        let provider = find_provider_for_model(&providers, "webauth-gemini-pro");
        assert!(provider.is_some());
        assert_eq!(provider.unwrap().id(), "gemini");

        let provider = find_provider_for_model(&providers, "nonexistent-model");
        assert!(provider.is_none());
    }

    #[test]
    fn test_all_providers_have_models() {
        let providers = create_all_providers();
        for p in &providers {
            assert!(
                !p.models().is_empty(),
                "Provider '{}' has no models",
                p.id()
            );
        }
    }

    #[test]
    fn test_model_ids_start_with_webauth() {
        let providers = create_all_providers();
        for p in &providers {
            for m in p.models() {
                assert!(
                    m.id.starts_with("webauth-"),
                    "Model '{}' from provider '{}' doesn't start with 'webauth-'",
                    m.id,
                    p.id()
                );
            }
        }
    }
}
