//! Structured Long-Term Memory — fact extraction and profile building.
//!
//! Ported from DeerFlow 2.0's memory system.
//! Instead of storing raw text, this module extracts discrete facts
//! from conversations with confidence scoring and categorization.
//!
//! ## Features
//! - **Fact extraction**: Extract preference/knowledge/context/behavior/goal facts
//! - **Confidence scoring**: Each fact has a 0.0-1.0 confidence
//! - **Deduplication**: Whitespace-normalized fact content dedup
//! - **User profile**: Work context, personal context, top-of-mind
//! - **Configurable**: Max facts, confidence threshold, injection token limit
//!
//! ## Storage
//! Facts stored as JSON at `~/.bizclaw/memory.json` (matches DeerFlow's `memory.json`)

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;
use tracing::{debug, info, warn};

/// Default storage path.
const DEFAULT_MEMORY_PATH: &str = "~/.bizclaw/memory.json";

/// Maximum stored facts.
const DEFAULT_MAX_FACTS: usize = 100;

/// Minimum confidence to keep a fact.
const DEFAULT_CONFIDENCE_THRESHOLD: f32 = 0.7;

/// Max tokens to inject into system prompt.
const DEFAULT_MAX_INJECTION_TOKENS: usize = 2000;

/// Category of a memory fact (matches DeerFlow's categories).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum FactCategory {
    /// User preferences (language, style, tools, etc.)
    Preference,
    /// Domain knowledge the user has shared.
    Knowledge,
    /// Current context (project, task, etc.)
    Context,
    /// Behavioral patterns (communication style, etc.)
    Behavior,
    /// User goals and objectives.
    Goal,
}

impl std::fmt::Display for FactCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Preference => write!(f, "preference"),
            Self::Knowledge => write!(f, "knowledge"),
            Self::Context => write!(f, "context"),
            Self::Behavior => write!(f, "behavior"),
            Self::Goal => write!(f, "goal"),
        }
    }
}

/// A discrete fact extracted from conversations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryFact {
    /// Unique identifier.
    pub id: String,
    /// The fact content.
    pub content: String,
    /// Category of this fact.
    pub category: FactCategory,
    /// Confidence score (0.0 - 1.0).
    pub confidence: f32,
    /// When this fact was created.
    pub created_at: DateTime<Utc>,
    /// Session that created this fact.
    pub source: String,
}

/// User profile built from accumulated facts.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UserProfile {
    /// Work context summary (1-3 sentences).
    /// e.g., "Tech stack: Rust + Android + TypeScript. Works on BizClaw AI platform."
    #[serde(default)]
    pub work_context: String,
    /// Personal context summary.
    /// e.g., "Vietnamese speaker, prefers dark mode, concise answers."
    #[serde(default)]
    pub personal_context: String,
    /// Current top-of-mind (1-3 sentences).
    /// e.g., "Currently porting DeerFlow features to BizClaw Rust codebase."
    #[serde(default)]
    pub top_of_mind: String,
    /// Recent months context.
    #[serde(default)]
    pub recent_months: String,
    /// Earlier context summary.
    #[serde(default)]
    pub earlier_context: String,
    /// Long-term background.
    #[serde(default)]
    pub long_term_background: String,
}

/// Full memory state persisted to disk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryState {
    /// User profile summaries.
    pub profile: UserProfile,
    /// Extracted facts.
    pub facts: Vec<MemoryFact>,
    /// Last update timestamp.
    pub updated_at: DateTime<Utc>,
}

impl Default for MemoryState {
    fn default() -> Self {
        Self {
            profile: UserProfile::default(),
            facts: Vec::new(),
            updated_at: Utc::now(),
        }
    }
}

/// Configuration for the structured memory system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuredMemoryConfig {
    /// Whether memory system is enabled.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Whether to inject memory into system prompts.
    #[serde(default = "default_true")]
    pub injection_enabled: bool,
    /// Path to memory.json file.
    #[serde(default = "default_memory_path")]
    pub storage_path: String,
    /// Maximum facts to store.
    #[serde(default = "default_max_facts")]
    pub max_facts: usize,
    /// Minimum confidence to keep a fact.
    #[serde(default = "default_confidence_threshold")]
    pub fact_confidence_threshold: f32,
    /// Max tokens to inject into prompt.
    #[serde(default = "default_max_injection_tokens")]
    pub max_injection_tokens: usize,
}

fn default_true() -> bool { true }
fn default_memory_path() -> String { DEFAULT_MEMORY_PATH.into() }
fn default_max_facts() -> usize { DEFAULT_MAX_FACTS }
fn default_confidence_threshold() -> f32 { DEFAULT_CONFIDENCE_THRESHOLD }
fn default_max_injection_tokens() -> usize { DEFAULT_MAX_INJECTION_TOKENS }

impl Default for StructuredMemoryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            injection_enabled: true,
            storage_path: DEFAULT_MEMORY_PATH.into(),
            max_facts: DEFAULT_MAX_FACTS,
            fact_confidence_threshold: DEFAULT_CONFIDENCE_THRESHOLD,
            max_injection_tokens: DEFAULT_MAX_INJECTION_TOKENS,
        }
    }
}

/// The structured memory manager.
pub struct StructuredMemory {
    config: StructuredMemoryConfig,
    state: MemoryState,
    storage_path: PathBuf,
}

impl StructuredMemory {
    /// Create or load from disk.
    pub fn new(config: StructuredMemoryConfig) -> Self {
        let storage_path = Self::resolve_path(&config.storage_path);

        let state = if storage_path.exists() {
            match std::fs::read_to_string(&storage_path) {
                Ok(content) => {
                    serde_json::from_str(&content).unwrap_or_else(|e| {
                        warn!("Failed to parse memory.json: {e}, starting fresh");
                        MemoryState::default()
                    })
                }
                Err(e) => {
                    warn!("Failed to read memory.json: {e}, starting fresh");
                    MemoryState::default()
                }
            }
        } else {
            MemoryState::default()
        };

        info!(
            "🧠 Structured memory loaded: {} facts, profile: {}",
            state.facts.len(),
            if state.profile.work_context.is_empty() {
                "empty"
            } else {
                "populated"
            }
        );

        Self {
            config,
            state,
            storage_path,
        }
    }

    /// Load with default config.
    pub fn load_default() -> Self {
        Self::new(StructuredMemoryConfig::default())
    }

    /// Resolve storage path (expand ~).
    fn resolve_path(path: &str) -> PathBuf {
        let expanded = shellexpand::tilde(path);
        PathBuf::from(expanded.as_ref())
    }

    /// Add a fact (with deduplication).
    ///
    /// DeerFlow deduplicates by normalizing whitespace before comparing content.
    pub fn add_fact(&mut self, fact: MemoryFact) -> bool {
        if fact.confidence < self.config.fact_confidence_threshold {
            debug!(
                "Fact rejected (confidence {:.2} < threshold {:.2}): {}",
                fact.confidence, self.config.fact_confidence_threshold, fact.content
            );
            return false;
        }

        // Whitespace-normalized dedup (matches DeerFlow behavior)
        let normalized = normalize_whitespace(&fact.content);
        let exists = self.state.facts.iter().any(|f| {
            normalize_whitespace(&f.content) == normalized
        });

        if exists {
            debug!("Fact already exists (dedup): {}", fact.content);
            return false;
        }

        self.state.facts.push(fact);

        // Enforce max facts limit — remove lowest confidence first
        if self.state.facts.len() > self.config.max_facts {
            self.state.facts.sort_by(|a, b| {
                b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal)
            });
            self.state.facts.truncate(self.config.max_facts);
        }

        self.state.updated_at = Utc::now();
        true
    }

    /// Add multiple facts from a parsed LLM response.
    pub fn add_facts(&mut self, facts: Vec<MemoryFact>) -> usize {
        let mut added = 0;
        for fact in facts {
            if self.add_fact(fact) {
                added += 1;
            }
        }
        if added > 0 {
            info!("🧠 Added {} new facts to memory", added);
        }
        added
    }

    /// Update user profile sections.
    pub fn update_profile(&mut self, profile: UserProfile) {
        if !profile.work_context.is_empty() {
            self.state.profile.work_context = profile.work_context;
        }
        if !profile.personal_context.is_empty() {
            self.state.profile.personal_context = profile.personal_context;
        }
        if !profile.top_of_mind.is_empty() {
            self.state.profile.top_of_mind = profile.top_of_mind;
        }
        if !profile.recent_months.is_empty() {
            self.state.profile.recent_months = profile.recent_months;
        }
        if !profile.earlier_context.is_empty() {
            self.state.profile.earlier_context = profile.earlier_context;
        }
        if !profile.long_term_background.is_empty() {
            self.state.profile.long_term_background = profile.long_term_background;
        }
        self.state.updated_at = Utc::now();
    }

    /// Build the memory injection text for system prompt.
    ///
    /// Returns text wrapped in `<memory>` tags (matches DeerFlow format).
    /// Top 15 facts + profile context, capped at max_injection_tokens.
    pub fn build_injection(&self) -> Option<String> {
        if !self.config.injection_enabled {
            return None;
        }

        let profile = &self.state.profile;
        let has_profile = !profile.work_context.is_empty()
            || !profile.personal_context.is_empty()
            || !profile.top_of_mind.is_empty();

        if !has_profile && self.state.facts.is_empty() {
            return None;
        }

        let mut parts = Vec::new();
        parts.push("<memory>".to_string());

        // Profile context
        if !profile.work_context.is_empty() {
            parts.push(format!("Work: {}", profile.work_context));
        }
        if !profile.personal_context.is_empty() {
            parts.push(format!("Personal: {}", profile.personal_context));
        }
        if !profile.top_of_mind.is_empty() {
            parts.push(format!("Current focus: {}", profile.top_of_mind));
        }

        // Top 15 facts by confidence
        if !self.state.facts.is_empty() {
            parts.push("\nKnown facts:".to_string());
            let mut sorted: Vec<&MemoryFact> = self.state.facts.iter().collect();
            sorted.sort_by(|a, b| {
                b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal)
            });

            for fact in sorted.iter().take(15) {
                parts.push(format!("- [{}] {}", fact.category, fact.content));
            }
        }

        parts.push("</memory>".to_string());

        let injection = parts.join("\n");

        // Token limit check (rough: 4 chars ≈ 1 token)
        let estimated_tokens = injection.len() / 4;
        if estimated_tokens > self.config.max_injection_tokens {
            // Truncate to fit
            let max_chars = self.config.max_injection_tokens * 4;
            let truncated: String = injection.chars().take(max_chars).collect();
            return Some(format!("{}\n</memory>", truncated));
        }

        Some(injection)
    }

    /// Persist to disk (atomic: temp file + rename).
    pub fn save(&self) -> Result<(), std::io::Error> {
        if let Some(parent) = self.storage_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let json = serde_json::to_string_pretty(&self.state)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        // Atomic write: temp file + rename (matches DeerFlow pattern)
        let tmp_path = self.storage_path.with_extension("tmp");
        std::fs::write(&tmp_path, &json)?;
        std::fs::rename(&tmp_path, &self.storage_path)?;

        debug!("💾 Memory saved: {} facts, {} bytes", self.state.facts.len(), json.len());
        Ok(())
    }

    /// Reload from disk.
    pub fn reload(&mut self) -> Result<(), std::io::Error> {
        if self.storage_path.exists() {
            let content = std::fs::read_to_string(&self.storage_path)?;
            self.state = serde_json::from_str(&content)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            info!("🧠 Memory reloaded: {} facts", self.state.facts.len());
        }
        Ok(())
    }

    /// Get all facts.
    pub fn facts(&self) -> &[MemoryFact] {
        &self.state.facts
    }

    /// Get facts by category.
    pub fn facts_by_category(&self, category: &FactCategory) -> Vec<&MemoryFact> {
        self.state.facts.iter().filter(|f| &f.category == category).collect()
    }

    /// Get user profile.
    pub fn profile(&self) -> &UserProfile {
        &self.state.profile
    }

    /// Get full state (for API responses).
    pub fn state(&self) -> &MemoryState {
        &self.state
    }

    /// Search facts by keyword.
    pub fn search_facts(&self, query: &str) -> Vec<&MemoryFact> {
        let query_lower = query.to_lowercase();
        self.state.facts
            .iter()
            .filter(|f| f.content.to_lowercase().contains(&query_lower))
            .collect()
    }

    /// Remove a fact by ID.
    pub fn remove_fact(&mut self, id: &str) -> bool {
        let before = self.state.facts.len();
        self.state.facts.retain(|f| f.id != id);
        self.state.facts.len() < before
    }

    /// Clear all facts.
    pub fn clear_facts(&mut self) {
        self.state.facts.clear();
        self.state.updated_at = Utc::now();
    }

    /// Get memory statistics.
    pub fn stats(&self) -> MemoryStats {
        let mut by_category = std::collections::HashMap::new();
        for fact in &self.state.facts {
            *by_category.entry(fact.category.clone()).or_insert(0) += 1;
        }
        let avg_confidence = if self.state.facts.is_empty() {
            0.0
        } else {
            self.state.facts.iter().map(|f| f.confidence).sum::<f32>()
                / self.state.facts.len() as f32
        };

        MemoryStats {
            total_facts: self.state.facts.len(),
            by_category,
            avg_confidence,
            has_profile: !self.state.profile.work_context.is_empty(),
            last_updated: self.state.updated_at,
        }
    }
}

/// Memory statistics for dashboard.
#[derive(Debug, Clone, Serialize)]
pub struct MemoryStats {
    pub total_facts: usize,
    pub by_category: std::collections::HashMap<FactCategory, usize>,
    pub avg_confidence: f32,
    pub has_profile: bool,
    pub last_updated: DateTime<Utc>,
}

/// Normalize whitespace for dedup comparison (matches DeerFlow).
fn normalize_whitespace(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ").trim().to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_fact(content: &str, category: FactCategory, confidence: f32) -> MemoryFact {
        MemoryFact {
            id: uuid::Uuid::new_v4().to_string(),
            content: content.into(),
            category,
            confidence,
            created_at: Utc::now(),
            source: "test".into(),
        }
    }

    #[test]
    fn test_add_fact_and_dedup() {
        let config = StructuredMemoryConfig {
            storage_path: "/tmp/test_memory.json".into(),
            ..Default::default()
        };
        let mut mem = StructuredMemory::new(config);

        let added = mem.add_fact(make_fact("User prefers Rust", FactCategory::Preference, 0.9));
        assert!(added);

        // Same content → dedup
        let added = mem.add_fact(make_fact("User prefers Rust", FactCategory::Preference, 0.95));
        assert!(!added);

        // Whitespace-normalized dedup
        let added = mem.add_fact(make_fact("  User  prefers  Rust  ", FactCategory::Preference, 0.8));
        assert!(!added);

        assert_eq!(mem.facts().len(), 1);
    }

    #[test]
    fn test_confidence_threshold() {
        let config = StructuredMemoryConfig {
            storage_path: "/tmp/test_memory2.json".into(),
            fact_confidence_threshold: 0.7,
            ..Default::default()
        };
        let mut mem = StructuredMemory::new(config);

        // Below threshold → rejected
        let added = mem.add_fact(make_fact("Low confidence fact", FactCategory::Context, 0.5));
        assert!(!added);
        assert_eq!(mem.facts().len(), 0);

        // Above threshold → accepted
        let added = mem.add_fact(make_fact("High confidence fact", FactCategory::Context, 0.8));
        assert!(added);
        assert_eq!(mem.facts().len(), 1);
    }

    #[test]
    fn test_max_facts_enforcement() {
        let config = StructuredMemoryConfig {
            storage_path: "/tmp/test_memory3.json".into(),
            max_facts: 3,
            fact_confidence_threshold: 0.0,
            ..Default::default()
        };
        let mut mem = StructuredMemory::new(config);

        for i in 0..5 {
            mem.add_fact(make_fact(
                &format!("Fact {i}"),
                FactCategory::Knowledge,
                i as f32 * 0.2,
            ));
        }

        assert_eq!(mem.facts().len(), 3);
        // Top 3 by confidence
        assert!(mem.facts().iter().all(|f| f.confidence >= 0.4));
    }

    #[test]
    fn test_build_injection() {
        let config = StructuredMemoryConfig {
            storage_path: "/tmp/test_memory4.json".into(),
            ..Default::default()
        };
        let mut mem = StructuredMemory::new(config);

        mem.update_profile(UserProfile {
            work_context: "Builds AI platform in Rust".into(),
            personal_context: "Vietnamese, prefers dark mode".into(),
            top_of_mind: "Porting DeerFlow features".into(),
            ..Default::default()
        });

        mem.add_fact(make_fact("Uses neovim", FactCategory::Preference, 0.9));
        mem.add_fact(make_fact("Expert in Rust", FactCategory::Knowledge, 0.95));

        let injection = mem.build_injection().unwrap();
        assert!(injection.contains("<memory>"));
        assert!(injection.contains("</memory>"));
        assert!(injection.contains("Builds AI platform"));
        assert!(injection.contains("Uses neovim"));
        assert!(injection.contains("[preference]"));
    }

    #[test]
    fn test_injection_disabled() {
        let config = StructuredMemoryConfig {
            storage_path: "/tmp/test_memory5.json".into(),
            injection_enabled: false,
            ..Default::default()
        };
        let mem = StructuredMemory::new(config);
        assert!(mem.build_injection().is_none());
    }

    #[test]
    fn test_search_facts() {
        let config = StructuredMemoryConfig {
            storage_path: "/tmp/test_memory6.json".into(),
            ..Default::default()
        };
        let mut mem = StructuredMemory::new(config);

        mem.add_fact(make_fact("Uses Rust for backend", FactCategory::Knowledge, 0.9));
        mem.add_fact(make_fact("Prefers TypeScript for frontend", FactCategory::Preference, 0.8));
        mem.add_fact(make_fact("Works on BizClaw platform", FactCategory::Context, 0.85));

        let results = mem.search_facts("Rust");
        assert_eq!(results.len(), 1);
        assert!(results[0].content.contains("Rust"));

        let results = mem.search_facts("bizclaw");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_facts_by_category() {
        let config = StructuredMemoryConfig {
            storage_path: "/tmp/test_memory7.json".into(),
            ..Default::default()
        };
        let mut mem = StructuredMemory::new(config);

        mem.add_fact(make_fact("Fact A", FactCategory::Preference, 0.9));
        mem.add_fact(make_fact("Fact B", FactCategory::Knowledge, 0.9));
        mem.add_fact(make_fact("Fact C", FactCategory::Preference, 0.9));

        let prefs = mem.facts_by_category(&FactCategory::Preference);
        assert_eq!(prefs.len(), 2);
    }

    #[test]
    fn test_remove_fact() {
        let config = StructuredMemoryConfig {
            storage_path: "/tmp/test_memory8.json".into(),
            ..Default::default()
        };
        let mut mem = StructuredMemory::new(config);

        let fact = make_fact("To remove", FactCategory::Context, 0.9);
        let id = fact.id.clone();
        mem.add_fact(fact);
        assert_eq!(mem.facts().len(), 1);

        assert!(mem.remove_fact(&id));
        assert_eq!(mem.facts().len(), 0);
    }

    #[test]
    fn test_save_and_reload() {
        let path = "/tmp/test_memory_persist.json";
        let config = StructuredMemoryConfig {
            storage_path: path.into(),
            ..Default::default()
        };
        let mut mem = StructuredMemory::new(config.clone());

        mem.add_fact(make_fact("Persistent fact", FactCategory::Knowledge, 0.9));
        mem.update_profile(UserProfile {
            work_context: "Test work context".into(),
            ..Default::default()
        });
        mem.save().unwrap();

        // Reload
        let mut mem2 = StructuredMemory::new(config);
        assert_eq!(mem2.facts().len(), 1);
        assert_eq!(mem2.profile().work_context, "Test work context");

        // Cleanup
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn test_stats() {
        let config = StructuredMemoryConfig {
            storage_path: "/tmp/test_memory_stats.json".into(),
            ..Default::default()
        };
        let mut mem = StructuredMemory::new(config);

        mem.add_fact(make_fact("A", FactCategory::Preference, 0.8));
        mem.add_fact(make_fact("B", FactCategory::Knowledge, 0.9));

        let stats = mem.stats();
        assert_eq!(stats.total_facts, 2);
        assert_eq!(stats.by_category.get(&FactCategory::Preference), Some(&1));
        assert!((stats.avg_confidence - 0.85).abs() < 0.01);
    }

    #[test]
    fn test_normalize_whitespace() {
        assert_eq!(normalize_whitespace("  hello   world  "), "hello world");
        assert_eq!(normalize_whitespace("Hi\n\tThere"), "hi there");
    }
}
