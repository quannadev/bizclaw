//! Marketplace Client - Kết nối với OpenHub/BizClaw Marketplace
//! 
//! Client để search và download skills từ marketplace.

use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplaceSkill {
    pub id: String,
    pub name: String,
    pub description: String,
    pub author: String,
    pub category: String,
    pub tags: Vec<String>,
    pub rating: f32,
    pub downloads: u64,
    pub version: String,
}

#[derive(Debug, Clone)]
pub struct MarketplaceConfig {
    pub base_url: String,
    pub timeout_secs: u64,
    pub api_key: Option<String>,
}

impl Default for MarketplaceConfig {
    fn default() -> Self {
        Self {
            base_url: "https://api.bizclaw.io/marketplace".to_string(),
            timeout_secs: 30,
            api_key: None,
        }
    }
}

/// Marketplace Client
pub struct MarketplaceClient {
    config: MarketplaceConfig,
    client: reqwest::Client,
}

impl MarketplaceClient {
    pub fn new(config: MarketplaceConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        
        Self { config, client }
    }

    pub fn with_defaults() -> Self {
        Self::new(MarketplaceConfig::default())
    }

    /// Search skills by query
    pub async fn search(&self, query: &str) -> Result<Vec<MarketplaceSkill>, String> {
        let url = format!("{}/skills/search", self.config.base_url);
        
        let mut request = self.client.get(&url).query(&[("q", query)]);
        
        if let Some(ref key) = self.config.api_key {
            request = request.header("Authorization", format!("Bearer {}", key));
        }
        
        let response = request.send().await
            .map_err(|e| format!("Request failed: {}", e))?;
        
        if !response.status().is_success() {
            return Err(format!("API returned status: {}", response.status()));
        }
        
        response.json::<SearchResponse>().await
            .map(|r| r.results)
            .map_err(|e| format!("Failed to parse response: {}", e))
    }

    /// Get skill details
    pub async fn get(&self, skill_id: &str) -> Result<MarketplaceSkill, String> {
        let url = format!("{}/skills/{}", self.config.base_url, skill_id);
        
        let mut request = self.client.get(&url);
        
        if let Some(ref key) = self.config.api_key {
            request = request.header("Authorization", format!("Bearer {}", key));
        }
        
        let response = request.send().await
            .map_err(|e| format!("Request failed: {}", e))?;
        
        if !response.status().is_success() {
            return Err(format!("Skill not found: {}", skill_id));
        }
        
        response.json::<MarketplaceSkill>().await
            .map_err(|e| format!("Failed to parse skill: {}", e))
    }

    /// Download skill content (SKILL.md)
    pub async fn download(&self, skill_id: &str) -> Result<String, String> {
        let url = format!("{}/skills/{}/download", self.config.base_url, skill_id);
        
        let mut request = self.client.get(&url);
        
        if let Some(ref key) = self.config.api_key {
            request = request.header("Authorization", format!("Bearer {}", key));
        }
        
        let response = request.send().await
            .map_err(|e| format!("Download failed: {}", e))?;
        
        if !response.status().is_success() {
            return Err(format!("Download failed with status: {}", response.status()));
        }
        
        response.text().await
            .map_err(|e| format!("Failed to read content: {}", e))
    }

    /// Get featured/trending skills
    pub async fn featured(&self) -> Result<Vec<MarketplaceSkill>, String> {
        let url = format!("{}/skills/featured", self.config.base_url);
        
        let response = self.client.get(&url).send().await
            .map_err(|e| format!("Request failed: {}", e))?;
        
        if !response.status().is_success() {
            return Err(format!("API returned status: {}", response.status()));
        }
        
        response.json::<FeaturedResponse>().await
            .map(|r| r.skills)
            .map_err(|e| format!("Failed to parse response: {}", e))
    }
}

#[derive(Debug, Deserialize)]
struct SearchResponse {
    results: Vec<MarketplaceSkill>,
}

#[derive(Debug, Deserialize)]
struct FeaturedResponse {
    skills: Vec<MarketplaceSkill>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_marketplace_config_default() {
        let config = MarketplaceConfig::default();
        assert_eq!(config.timeout_secs, 30);
    }
}
