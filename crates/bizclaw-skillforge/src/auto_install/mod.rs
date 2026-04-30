//! Auto-Skill Installer - Tự động cài đặt skills khi cần
//! 
//! Đây là tính năng AGENTIC chính: Khi mama tổng quản yêu cầu,
//! agent có thể tự động tìm và cài đặt skills phù hợp từ:
//! - Local skill directory
//! - Built-in skills
//! - Marketplace (OpenHub)
//! - SkillForge generated skills
//! 
//! ## Flow:
//! 1. Mama tổng quản nhận yêu cầu từ user
//! 2. Agent phân tích yêu cầu
//! 3. AutoSkillInstaller tìm skills phù hợp
//! 4. Nếu chưa có → tìm từ marketplace hoặc tạo mới
//! 5. Cài đặt và load vào agent context

use crate::analyzer::TraceAnalyzer;
use crate::extractor::PatternExtractor;
use crate::generator::SkillGenerator;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::sync::RwLock;
use std::sync::Arc;

pub mod marketplace_client;
pub use marketplace_client::MarketplaceClient;

/// Kết quả của auto-install process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallResult {
    pub success: bool,
    pub skill_name: String,
    pub source: InstallSource,
    pub action: InstallAction,
    pub message: String,
    pub skill_path: Option<String>,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum InstallSource {
    Local,
    Builtin,
    Marketplace,
    SkillForge,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum InstallAction {
    AlreadyInstalled,
    Installed,
    Updated,
    Generated,
    NotFound,
}

/// Yêu cầu cài đặt skill
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillRequest {
    pub task: String,
    pub context: Option<String>,
    pub required_tools: Vec<String>,
    pub business_domain: Option<String>,
    pub keywords: Vec<String>,
}

/// Auto-Skill Installer
pub struct AutoSkillInstaller {
    local_skills_dir: PathBuf,
    builtin_skills: HashMap<String, SkillInfo>,
    marketplace_client: Option<MarketplaceClient>,
    skill_analyzer: TraceAnalyzer,
    pattern_extractor: PatternExtractor,
    skill_generator: SkillGenerator,
    installed_skills: Arc<RwLock<HashMap<String, InstallResult>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillInfo {
    pub name: String,
    pub description: String,
    pub category: String,
    pub tags: Vec<String>,
    pub tools: Vec<String>,
    pub path: Option<PathBuf>,
}

impl AutoSkillInstaller {
    pub fn new(local_skills_dir: PathBuf) -> Self {
        Self {
            local_skills_dir,
            builtin_skills: Self::load_builtin_skills(),
            marketplace_client: None,
            skill_analyzer: TraceAnalyzer::new(),
            pattern_extractor: PatternExtractor::with_defaults(),
            skill_generator: SkillGenerator::with_defaults(),
            installed_skills: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn with_marketplace(mut self, client: MarketplaceClient) -> Self {
        self.marketplace_client = Some(client);
        self
    }

    /// Main entry point: Tự động tìm và cài đặt skill cho task
    pub async fn auto_install(&self, request: &SkillRequest) -> InstallResult {
        // 1. Check local skills
        if let Some(result) = self.check_local(&request.task, &request.keywords) {
            tracing::info!("[skillforge] Found skill locally: {}", result.skill_name);
            return result;
        }

        // 2. Check built-in skills
        if let Some(result) = self.check_builtin(&request.task, &request.keywords) {
            tracing::info!("[skillforge] Found built-in skill: {}", result.skill_name);
            return result;
        }

        // 3. Check marketplace
        if let Some(ref client) = self.marketplace_client {
            if let Some(result) = self.search_marketplace(client, &request.task, &request.keywords).await {
                tracing::info!("[skillforge] Found skill in marketplace: {}", result.skill_name);
                return result;
            }
        }

        // 4. Generate new skill from patterns
        if let Some(result) = self.generate_skill(request) {
            tracing::info!("[skillforge] Generated new skill: {}", result.skill_name);
            return result;
        }

        // 5. Not found
        InstallResult {
            success: false,
            skill_name: request.task.clone(),
            source: InstallSource::Local,
            action: InstallAction::NotFound,
            message: format!("No skill found for: {}", request.task),
            skill_path: None,
            confidence: 0.0,
        }
    }

    /// Check local skills directory
    fn check_local(&self, task: &str, keywords: &[String]) -> Option<InstallResult> {
        let skill_dir = &self.local_skills_dir;
        
        if !skill_dir.exists() {
            return None;
        }

        // Scan for SKILL.md files
        if let Ok(entries) = std::fs::read_dir(skill_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                
                // Check both flat .md files and subdirectory/SKILL.md
                let skill_path = if path.is_dir() {
                    path.join("SKILL.md")
                } else if path.extension().is_some_and(|e| e == "md") {
                    path.clone()
                } else {
                    continue;
                };

                if skill_path.exists() {
                    if let Ok(content) = std::fs::read_to_string(&skill_path) {
                        if self.matches_skill(&content, task, keywords) {
                            let skill_name = path.file_stem()
                                .and_then(|s| s.to_str())
                                .unwrap_or("unknown")
                                .to_string();
                            
                            return Some(InstallResult {
                                success: true,
                                skill_name,
                                source: InstallSource::Local,
                                action: InstallAction::AlreadyInstalled,
                                message: format!("Skill already installed at {:?}", skill_path),
                                skill_path: Some(skill_path.to_string_lossy().to_string()),
                                confidence: 0.9,
                            });
                        }
                    }
                }
            }
        }

        None
    }

    /// Check built-in skills
    fn check_builtin(&self, task: &str, keywords: &[String]) -> Option<InstallResult> {
        let task_lower = task.to_lowercase();
        let keyword_match = |tags: &[String]| {
            keywords.iter().any(|k| {
                tags.iter().any(|t| t.to_lowercase().contains(&k.to_lowercase()))
            })
        };

        for (name, info) in &self.builtin_skills {
            let name_match = name.to_lowercase().contains(&task_lower);
            let desc_match = info.description.to_lowercase().contains(&task_lower);
            let tag_match = keyword_match(&info.tags);

            if name_match || desc_match || tag_match {
                return Some(InstallResult {
                    success: true,
                    skill_name: name.clone(),
                    source: InstallSource::Builtin,
                    action: InstallAction::Installed,
                    message: format!("Installed built-in skill: {}", info.description),
                    skill_path: None,
                    confidence: if name_match { 0.95 } else { 0.7 },
                });
            }
        }

        None
    }

    /// Search marketplace
    async fn search_marketplace(
        &self,
        client: &MarketplaceClient,
        task: &str,
        keywords: &[String],
    ) -> Option<InstallResult> {
        let query = if !keywords.is_empty() {
            keywords.join(" ")
        } else {
            task.to_string()
        };

        match client.search(&query).await {
            Ok(results) => {
                if let Some(skill) = results.into_iter().next() {
                    // Download and install
                    match client.download(&skill.id).await {
                        Ok(content) => {
                            let path = self.local_skills_dir.join(format!("{}.md", skill.id));
                            if let Ok(()) = std::fs::write(&path, &content) {
                                return Some(InstallResult {
                                    success: true,
                                    skill_name: skill.name.clone(),
                                    source: InstallSource::Marketplace,
                                    action: InstallAction::Installed,
                                    message: format!("Downloaded from marketplace: {}", skill.name),
                                    skill_path: Some(path.to_string_lossy().to_string()),
                                    confidence: skill.rating,
                                });
                            }
                        }
                        Err(e) => {
                            tracing::warn!("[skillforge] Failed to download skill: {}", e);
                        }
                    }
                }
            }
            Err(e) => {
                tracing::warn!("[skillforge] Marketplace search failed: {}", e);
            }
        }

        None
    }

    /// Generate new skill from task analysis
    fn generate_skill(&self, request: &SkillRequest) -> Option<InstallResult> {
        // Only generate if we have enough context
        if request.required_tools.is_empty() && request.keywords.is_empty() {
            return None;
        }

        // Create a basic pattern from the request
        let pattern = crate::extractor::ExtractedPattern {
            pattern_id: format!("auto_{}", request.task.replace(' ', "_").to_lowercase()),
            pattern_type: crate::extractor::PatternType::Workflow,
            name: request.task.clone(),
            description: request.context.clone().unwrap_or_else(|| request.task.clone()),
            tools_required: request.required_tools.clone(),
            steps: request.required_tools.iter().enumerate().map(|(i, tool)| {
                crate::extractor::PatternStep {
                    step_number: (i + 1) as u32,
                    action: format!("Execute {}", tool),
                    tool: Some(tool.clone()),
                    description: format!("Execute {} for task: {}", tool, request.task),
                    on_error: Some(format!("Handle {} failure", tool)),
                }
            }).collect(),
            error_handling: vec![
                "Log all errors".to_string(),
                "Retry on transient failures".to_string(),
            ],
            confidence: 0.5,
            examples: vec![],
            category: request.business_domain.clone().unwrap_or_else(|| "auto".to_string()),
            business_domain: request.business_domain.clone(),
        };

        // Generate SKILL.md
        let skill = self.skill_generator.generate(&pattern);
        
        // Save to local skills directory
        let path = self.local_skills_dir.join(format!("{}.md", pattern.pattern_id));
        
        if let Err(e) = std::fs::write(&path, &skill.raw_skill_md) {
            tracing::warn!("[skillforge] Failed to write generated skill: {}", e);
            return None;
        }

        Some(InstallResult {
            success: true,
            skill_name: pattern.name.clone(),
            source: InstallSource::SkillForge,
            action: InstallAction::Generated,
            message: format!("Generated new skill from SkillForge: {}", pattern.name),
            skill_path: Some(path.to_string_lossy().to_string()),
            confidence: 0.5,
        })
    }

    /// Check if skill content matches task/keywords
    fn matches_skill(&self, content: &str, task: &str, keywords: &[String]) -> bool {
        let content_lower = content.to_lowercase();
        let task_lower = task.to_lowercase();

        // Check task match
        if content_lower.contains(&task_lower) {
            return true;
        }

        // Check keyword matches
        for keyword in keywords {
            if content_lower.contains(&keyword.to_lowercase()) {
                return true;
            }
        }

        false
    }

    /// Load built-in skills info
    fn load_builtin_skills() -> HashMap<String, SkillInfo> {
        let mut skills = HashMap::new();

        // Add common built-in skills
        skills.insert("rust-expert".to_string(), SkillInfo {
            name: "rust-expert".to_string(),
            description: "Deep expertise in Rust programming, ownership, async, traits".to_string(),
            category: "coding".to_string(),
            tags: vec!["rust".to_string(), "programming".to_string(), "systems".to_string()],
            tools: vec!["cargo".to_string(), "rustc".to_string()],
            path: None,
        });

        skills.insert("python-analyst".to_string(), SkillInfo {
            name: "python-analyst".to_string(),
            description: "Python data analysis, pandas, numpy, visualization".to_string(),
            category: "data".to_string(),
            tags: vec!["python".to_string(), "data".to_string(), "analytics".to_string()],
            tools: vec!["python".to_string(), "pandas".to_string()],
            path: None,
        });

        skills.insert("web-developer".to_string(), SkillInfo {
            name: "web-developer".to_string(),
            description: "Full-stack web development with modern frameworks".to_string(),
            category: "coding".to_string(),
            tags: vec!["web".to_string(), "javascript".to_string(), "html".to_string(), "css".to_string()],
            tools: vec!["node".to_string(), "npm".to_string()],
            path: None,
        });

        skills.insert("content-writer".to_string(), SkillInfo {
            name: "content-writer".to_string(),
            description: "Professional writing for blogs, marketing, social media".to_string(),
            category: "writing".to_string(),
            tags: vec!["writing".to_string(), "content".to_string(), "marketing".to_string()],
            tools: vec![],
            path: None,
        });

        skills
    }

    /// Get installed skills history
    pub async fn get_installed_history(&self) -> Vec<InstallResult> {
        let guard = self.installed_skills.read().await;
        guard.values().cloned().collect()
    }
}

/// Integration với Mama - tự động gọi khi mama nhận yêu cầu
impl AutoSkillInstaller {
    /// Called by Mama when a new request comes in
    pub async fn mama_hook(&self, request: &SkillRequest) -> Option<InstallResult> {
        tracing::info!("[skillforge] Mama hook: checking skill for task '{}'", request.task);
        
        let result = self.auto_install(request).await;
        
        // Record in history
        {
            let mut guard = self.installed_skills.write().await;
            guard.insert(result.skill_name.clone(), result.clone());
        }
        
        if result.success {
            Some(result)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_auto_install_finds_builtin() {
        let installer = AutoSkillInstaller::new(PathBuf::from("/tmp/test_skills"));
        
        let request = SkillRequest {
            task: "Write Rust code".to_string(),
            context: Some("I need help with async Rust".to_string()),
            required_tools: vec!["cargo".to_string()],
            business_domain: Some("development".to_string()),
            keywords: vec!["rust".to_string(), "programming".to_string()],
        };
        
        let result = installer.auto_install(&request).await;
        
        assert!(result.success);
        assert_eq!(result.source, InstallSource::Builtin);
    }

    #[tokio::test]
    async fn test_auto_install_not_found() {
        let installer = AutoSkillInstaller::new(PathBuf::from("/tmp/test_skills"));
        
        let request = SkillRequest {
            task: "Quantum computing".to_string(),
            context: None,
            required_tools: vec![],
            business_domain: None,
            keywords: vec!["quantum".to_string()],
        };
        
        let result = installer.auto_install(&request).await;
        
        // Should fail because no tools and low match
        assert!(!result.success || result.action == InstallAction::NotFound);
    }
}
