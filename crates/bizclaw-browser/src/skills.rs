use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserSkill {
    pub name: String,
    pub domain: String,
    pub description: String,
    pub selectors: HashMap<String, String>,
    pub workflows: Vec<SkillWorkflow>,
    pub notes: Vec<String>,
    pub examples: Vec<SkillExample>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillWorkflow {
    pub name: String,
    pub steps: Vec<WorkflowStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    pub action: String,
    pub selector: Option<String>,
    pub value: Option<String>,
    pub wait_for: Option<String>,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillExample {
    pub task: String,
    pub steps: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillMatch {
    pub skill: String,
    pub confidence: f32,
    pub matched_selectors: Vec<String>,
}

pub struct SkillRegistry {
    skills: Arc<RwLock<HashMap<String, BrowserSkill>>>,
    pattern_matcher: Arc<RwLock<PatternMatcher>>,
}

struct PatternMatcher {
    patterns: HashMap<String, Vec<String>>,
}

impl Default for PatternMatcher {
    fn default() -> Self {
        let mut patterns = HashMap::new();
        
        patterns.insert("github".to_string(), vec![
            "github.com".to_string(),
            "githubusercontent.com".to_string(),
        ]);
        
        patterns.insert("linkedin".to_string(), vec![
            "linkedin.com".to_string(),
            "licdn.com".to_string(),
        ]);
        
        patterns.insert("google".to_string(), vec![
            "google.com".to_string(),
            "google.co".to_string(),
        ]);
        
        patterns.insert("twitter".to_string(), vec![
            "twitter.com".to_string(),
            "x.com".to_string(),
        ]);
        
        patterns.insert("youtube".to_string(), vec![
            "youtube.com".to_string(),
            "youtu.be".to_string(),
        ]);
        
        patterns.insert("amazon".to_string(), vec![
            "amazon.com".to_string(),
            "amazon.co".to_string(),
        ]);
        
        patterns.insert("zalo".to_string(), vec![
            "zalo.me".to_string(),
            "zaloapp.com".to_string(),
        ]);
        
        Self { patterns }
    }
}

impl Default for SkillRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl SkillRegistry {
    pub fn new() -> Self {
        let registry = Self {
            skills: Arc::new(RwLock::new(HashMap::new())),
            pattern_matcher: Arc::new(RwLock::new(PatternMatcher::default())),
        };
        
        registry.register_builtin_skills();
        registry
    }
    
    fn register_builtin_skills(&self) {
        self.register(GitHubSkill::get_skill());
        self.register(LinkedInSkill::get_skill());
        self.register(GoogleSearchSkill::get_skill());
        self.register(TwitterSkill::get_skill());
        self.register(AmazonSkill::get_skill());
    }
    
    pub fn register(&self, skill: BrowserSkill) {
        info!("Registering skill: {} for domain: {}", skill.name, skill.domain);
        if let Ok(mut guard) = self.skills.write() {
            guard.insert(skill.domain.clone(), skill);
        }
    }
    
    pub fn get(&self, domain: &str) -> Option<BrowserSkill> {
        self.skills.read().ok()?.get(domain).cloned()
    }
    
    pub fn find_matching_skills(&self, url: &str) -> Vec<SkillMatch> {
        let mut matches = Vec::new();
        let url_lower = url.to_lowercase();
        
        let skills_guard = match self.skills.read() {
            Ok(g) => g,
            Err(_) => return matches,
        };
        
        let patterns_guard = match self.pattern_matcher.read() {
            Ok(g) => g,
            Err(_) => return matches,
        };
        
        for (domain, skill) in skills_guard.iter() {
            let mut matched_selectors = Vec::new();
            let mut score: f32 = 0.0;
            
            if let Some(patterns) = patterns_guard.patterns.get(domain) {
                for pattern in patterns {
                    if url_lower.contains(&pattern.to_lowercase()) {
                        score += 0.8;
                        matched_selectors.push(pattern.clone());
                    }
                }
            }
            
            if score > 0.0 {
                matches.push(SkillMatch {
                    skill: skill.name.clone(),
                    confidence: score.min(1.0),
                    matched_selectors,
                });
            }
        }
        
        matches.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());
        matches
    }
    
    pub fn list_skills(&self) -> Vec<BrowserSkill> {
        match self.skills.read() {
            Ok(guard) => guard.values().cloned().collect(),
            Err(_) => vec![],
        }
    }
    
    pub fn add_pattern(&self, domain: &str, pattern: &str) {
        if let Ok(mut guard) = self.pattern_matcher.write() {
            guard.patterns
                .entry(domain.to_string())
                .or_insert_with(Vec::new)
                .push(pattern.to_string());
        }
    }
    
    pub fn learn_from_task(&self, domain: &str, task: &str, steps: Vec<String>) {
        if let Ok(mut guard) = self.skills.write() {
            if let Some(skill) = guard.get_mut(domain) {
                skill.examples.push(SkillExample {
                    task: task.to_string(),
                    steps,
                });
            }
        }
    }
}

struct GitHubSkill;

impl GitHubSkill {
    fn get_skill() -> BrowserSkill {
        let mut selectors = HashMap::new();
        selectors.insert("search_input".to_string(), "[data-testid='search-input'], .header-search-input, input[name='query']".to_string());
        selectors.insert("search_button".to_string(), "[data-testid='search-button'], button[type='submit']".to_string());
        selectors.insert("repository_name".to_string(), ".repo, .repository-name, [itemprop='name']".to_string());
        selectors.insert("readme".to_string(), "#readme, .markdown-body".to_string());
        
        BrowserSkill {
            name: "GitHub".to_string(),
            domain: "github".to_string(),
            description: "GitHub repository navigation, code browsing, and issue management".to_string(),
            selectors,
            workflows: vec![],
            notes: vec!["Use Cmd+K for command palette".to_string()],
            examples: vec![],
        }
    }
}

struct LinkedInSkill;

impl LinkedInSkill {
    fn get_skill() -> BrowserSkill {
        let mut selectors = HashMap::new();
        selectors.insert("search_input".to_string(), "input[placeholder*='Search'], .search-global-typeahead__input".to_string());
        selectors.insert("message_input".to_string(), ".msg-form__contenteditable".to_string());
        
        BrowserSkill {
            name: "LinkedIn".to_string(),
            domain: "linkedin".to_string(),
            description: "LinkedIn networking, messaging, and profile management".to_string(),
            selectors,
            workflows: vec![],
            notes: vec![],
            examples: vec![],
        }
    }
}

struct GoogleSearchSkill;

impl GoogleSearchSkill {
    fn get_skill() -> BrowserSkill {
        let mut selectors = HashMap::new();
        selectors.insert("search_input".to_string(), "textarea[name='q'], input[name='q']".to_string());
        selectors.insert("search_button".to_string(), "input[name='btnK']".to_string());
        selectors.insert("results".to_string(), ".g".to_string());
        
        BrowserSkill {
            name: "Google Search".to_string(),
            domain: "google".to_string(),
            description: "Google search and web navigation".to_string(),
            selectors,
            workflows: vec![],
            notes: vec![],
            examples: vec![],
        }
    }
}

struct TwitterSkill;

impl TwitterSkill {
    fn get_skill() -> BrowserSkill {
        let mut selectors = HashMap::new();
        selectors.insert("compose_tweet".to_string(), "[data-testid='tweetTextarea_0']".to_string());
        selectors.insert("tweet_button".to_string(), "[data-testid='tweetButtonInline']".to_string());
        
        BrowserSkill {
            name: "Twitter/X".to_string(),
            domain: "twitter".to_string(),
            description: "Twitter posting, engagement, and social interactions".to_string(),
            selectors,
            workflows: vec![],
            notes: vec![],
            examples: vec![],
        }
    }
}

struct AmazonSkill;

impl AmazonSkill {
    fn get_skill() -> BrowserSkill {
        let mut selectors = HashMap::new();
        selectors.insert("search_input".to_string(), "#twotabsearchtextbox".to_string());
        selectors.insert("product_title".to_string(), "#productTitle".to_string());
        selectors.insert("price".to_string(), ".a-price .a-offscreen".to_string());
        
        BrowserSkill {
            name: "Amazon".to_string(),
            domain: "amazon".to_string(),
            description: "Amazon product search, review reading, and purchasing".to_string(),
            selectors,
            workflows: vec![],
            notes: vec![],
            examples: vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_skill_registry() {
        let registry = SkillRegistry::new();
        
        assert!(registry.get("github").is_some());
        assert!(registry.get("linkedin").is_some());
        assert!(registry.get("google").is_some());
        
        let matches = registry.find_matching_skills("https://github.com/user/repo");
        assert!(!matches.is_empty());
        assert_eq!(matches[0].skill, "GitHub");
    }
}
