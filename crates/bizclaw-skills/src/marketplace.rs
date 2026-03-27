//! Skills marketplace — remote skill discovery and installation.

use serde::{Deserialize, Serialize};

/// A skill listing from the marketplace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillListing {
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub version: String,
    pub author: String,
    pub category: String,
    pub tags: Vec<String>,
    pub icon: String,
    pub downloads: u64,
    pub rating: f32,
    pub url: String,
}

/// Skills marketplace client.
pub struct SkillMarketplace {
    /// Base URL for the marketplace API.
    base_url: String,
    /// Cached listings.
    cache: Vec<SkillListing>,
}

impl SkillMarketplace {
    /// Create a new marketplace client.
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
            cache: Vec::new(),
        }
    }

    /// Default marketplace (BizClaw Hub).
    pub fn default_hub() -> Self {
        Self::new("https://hub.bizclaw.vn/api/v1/skills")
    }

    /// Search the marketplace (local cache for now).
    pub fn search(&self, query: &str) -> Vec<&SkillListing> {
        let q = query.to_lowercase();
        self.cache
            .iter()
            .filter(|s| {
                s.name.to_lowercase().contains(&q)
                    || s.description.to_lowercase().contains(&q)
                    || s.tags.iter().any(|t| t.to_lowercase().contains(&q))
            })
            .collect()
    }

    /// List all cached skills.
    pub fn list(&self) -> &[SkillListing] {
        &self.cache
    }

    /// Get by category.
    pub fn by_category(&self, category: &str) -> Vec<&SkillListing> {
        self.cache
            .iter()
            .filter(|s| s.category.eq_ignore_ascii_case(category))
            .collect()
    }

    /// Get the marketplace base URL.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Add a listing to the cache (for built-in/offline mode).
    pub fn add_listing(&mut self, listing: SkillListing) {
        self.cache.push(listing);
    }

    /// Count cached listings.
    pub fn count(&self) -> usize {
        self.cache.len()
    }

    /// Sort by downloads (most popular first).
    pub fn sort_by_popularity(&mut self) {
        self.cache.sort_by(|a, b| b.downloads.cmp(&a.downloads));
    }

    /// Sort by rating (highest first).
    pub fn sort_by_rating(&mut self) {
        self.cache.sort_by(|a, b| {
            b.rating
                .partial_cmp(&a.rating)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }

    /// Install a skill from the Hub by name.
    /// Downloads the skill archive, extracts it, and registers it.
    pub async fn install_from_hub(
        &mut self,
        skill_name: &str,
        install_dir: &std::path::Path,
    ) -> Result<SkillListing, String> {
        let url = format!("{}/{}/download", self.base_url, skill_name);
        tracing::info!("📦 Installing skill '{}' from {}", skill_name, url);

        // Fetch skill metadata first
        let meta_url = format!("{}/{}", self.base_url, skill_name);
        let client = reqwest::Client::new();
        let listing: SkillListing = client
            .get(&meta_url)
            .send()
            .await
            .map_err(|e| format!("Failed to fetch skill info: {}", e))?
            .json()
            .await
            .map_err(|e| format!("Failed to parse skill info: {}", e))?;

        // Download skill archive
        let archive_bytes = client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Failed to download skill: {}", e))?
            .bytes()
            .await
            .map_err(|e| format!("Failed to read skill archive: {}", e))?;

        // Create skill directory
        let skill_dir = install_dir.join(&listing.name);
        std::fs::create_dir_all(&skill_dir)
            .map_err(|e| format!("Failed to create skill dir: {}", e))?;

        // Write archive (for now, treat as tar.gz or single SKILL.md)
        let archive_path = skill_dir.join("archive.tar.gz");
        std::fs::write(&archive_path, &archive_bytes)
            .map_err(|e| format!("Failed to write archive: {}", e))?;

        // Add to cache
        self.cache.push(listing.clone());
        tracing::info!("✅ Installed skill '{}' v{}", listing.name, listing.version);
        Ok(listing)
    }

    /// Uninstall a skill by name.
    pub fn uninstall(
        &mut self,
        skill_name: &str,
        install_dir: &std::path::Path,
    ) -> Result<(), String> {
        let skill_dir = install_dir.join(skill_name);
        if skill_dir.exists() {
            std::fs::remove_dir_all(&skill_dir)
                .map_err(|e| format!("Failed to remove skill dir: {}", e))?;
        }
        self.cache.retain(|s| s.name != skill_name);
        tracing::info!("🗑️ Uninstalled skill '{}'", skill_name);
        Ok(())
    }

    /// Check for updates: compare local versions with Hub versions.
    pub async fn check_updates(&self) -> Vec<(String, String, String)> {
        // Returns: Vec<(name, local_version, hub_version)>
        let mut updates = Vec::new();
        let client = reqwest::Client::new();

        for skill in &self.cache {
            let url = format!("{}/{}", self.base_url, skill.name);
            if let Ok(resp) = client.get(&url).send().await {
                if let Ok(hub) = resp.json::<SkillListing>().await {
                    if hub.version != skill.version {
                        updates.push((
                            skill.name.clone(),
                            skill.version.clone(),
                            hub.version.clone(),
                        ));
                    }
                }
            }
        }
        updates
    }
}

impl Default for SkillMarketplace {
    fn default() -> Self {
        Self::default_hub()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_listing(name: &str, category: &str, downloads: u64) -> SkillListing {
        SkillListing {
            name: name.to_string(),
            display_name: name.replace('-', " "),
            description: format!("A {} skill", name),
            version: "1.0.0".to_string(),
            author: "BizClaw".to_string(),
            category: category.to_string(),
            tags: vec![category.to_string()],
            icon: "📦".to_string(),
            downloads,
            rating: 4.5,
            url: format!("https://hub.bizclaw.vn/skills/{}", name),
        }
    }

    #[test]
    fn test_marketplace_search() {
        let mut mp = SkillMarketplace::new("https://test");
        mp.add_listing(sample_listing("rust-dev", "coding", 100));
        mp.add_listing(sample_listing("python-ml", "data", 200));
        mp.add_listing(sample_listing("devops-k8s", "devops", 50));

        assert_eq!(mp.search("rust").len(), 1);
        assert_eq!(mp.search("coding").len(), 1);
        assert_eq!(mp.count(), 3);
    }

    #[test]
    fn test_marketplace_sort() {
        let mut mp = SkillMarketplace::new("https://test");
        mp.add_listing(sample_listing("a", "x", 10));
        mp.add_listing(sample_listing("b", "x", 100));
        mp.add_listing(sample_listing("c", "x", 50));

        mp.sort_by_popularity();
        assert_eq!(mp.list()[0].name, "b");
        assert_eq!(mp.list()[2].name, "a");
    }
}
