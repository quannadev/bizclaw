//! Fact-Based Memory Store — structured knowledge extraction inspired by DeerFlow 2.0.
//!
//! Extracts discrete facts with category and confidence scoring from conversations,
//! enabling cross-session recall. Complements the existing λ-Memory (decay-based)
//! and FTS5 (keyword-based) backends.
//!
//! ## Architecture
//! - **Facts**: Discrete knowledge units with confidence, category, and source
//! - **Deduplication**: Whitespace-normalized content matching prevents duplicates
//! - **Injection**: Top-N facts injected into system prompt (configurable token cap)
//! - **Persistence**: Atomic JSON file I/O with temp-file + rename pattern
//!
//! ## Usage
//! ```rust,no_run
//! use bizclaw_memory::facts::{FactStore, Fact, FactCategory};
//!
//! let mut store = FactStore::new(100, 0.7);
//! store.add_fact("User prefers Rust", FactCategory::Preference, 0.95, "conversation");
//! let top = store.top_facts(15);
//! let prompt_ctx = store.to_prompt_context(2000);
//! ```

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Fact categories — aligned with DeerFlow's memory schema.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FactCategory {
    /// User preference (e.g., "prefers dark mode", "uses Rust")
    Preference,
    /// Domain knowledge (e.g., "BizClaw uses axum for gateway")
    Knowledge,
    /// Contextual information (e.g., "working on project X")
    Context,
    /// Behavioral pattern (e.g., "always commits before deploying")
    Behavior,
    /// User goal (e.g., "wants to launch by Q2")
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

/// A discrete fact with confidence scoring.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fact {
    /// Unique identifier.
    pub id: String,
    /// Fact content (e.g., "User prefers Vietnamese for UI").
    pub content: String,
    /// Fact category.
    pub category: FactCategory,
    /// Confidence score (0.0 - 1.0).
    pub confidence: f32,
    /// When this fact was first observed.
    pub created_at: DateTime<Utc>,
    /// Source of the fact (e.g., "conversation", "knowledge_base", "hand_result").
    pub source: String,
    /// Number of times this fact has been reinforced.
    #[serde(default = "default_one")]
    pub reinforcement_count: u32,
}

fn default_one() -> u32 {
    1
}

/// User context summary — high-level profile information.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UserContext {
    /// What the user is working on right now.
    #[serde(default)]
    pub work_context: String,
    /// Personal preferences and background.
    #[serde(default)]
    pub personal_context: String,
    /// Most important things to remember.
    #[serde(default)]
    pub top_of_mind: String,
}

/// Full memory data — persisted as JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactMemory {
    /// Structured user context.
    #[serde(default)]
    pub user_context: UserContext,
    /// Discrete facts with confidence scoring.
    #[serde(default)]
    pub facts: Vec<Fact>,
}

impl Default for FactMemory {
    fn default() -> Self {
        Self {
            user_context: UserContext::default(),
            facts: Vec::new(),
        }
    }
}

/// Fact Store — manages the lifecycle of facts.
pub struct FactStore {
    data: FactMemory,
    /// Maximum number of facts to store.
    max_facts: usize,
    /// Minimum confidence threshold for keeping facts.
    min_confidence: f32,
    /// File path for persistence.
    storage_path: Option<std::path::PathBuf>,
    /// Whether data has been modified since last save.
    dirty: bool,
}

impl FactStore {
    /// Create a new in-memory fact store.
    pub fn new(max_facts: usize, min_confidence: f32) -> Self {
        Self {
            data: FactMemory::default(),
            max_facts,
            min_confidence,
            storage_path: None,
            dirty: false,
        }
    }

    /// Create a fact store with file persistence.
    pub fn with_path(max_facts: usize, min_confidence: f32, path: impl Into<std::path::PathBuf>) -> Self {
        let path = path.into();
        let data = if path.exists() {
            match std::fs::read_to_string(&path) {
                Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
                Err(_) => FactMemory::default(),
            }
        } else {
            FactMemory::default()
        };

        Self {
            data,
            max_facts,
            min_confidence,
            storage_path: Some(path),
            dirty: false,
        }
    }

    /// Add a fact with deduplication.
    ///
    /// If a fact with the same normalized content exists, reinforce it
    /// instead of creating a duplicate (inspired by DeerFlow's dedup logic).
    pub fn add_fact(
        &mut self,
        content: &str,
        category: FactCategory,
        confidence: f32,
        source: &str,
    ) -> bool {
        let normalized = normalize_content(content);

        // Check for duplicate — reinforce if found
        for existing in &mut self.data.facts {
            if normalize_content(&existing.content) == normalized {
                existing.reinforcement_count += 1;
                // Boost confidence slightly on reinforcement (capped at 1.0)
                existing.confidence = (existing.confidence + 0.05).min(1.0);
                self.dirty = true;
                return false; // Not a new fact
            }
        }

        // Skip if below confidence threshold
        if confidence < self.min_confidence {
            return false;
        }

        let fact = Fact {
            id: format!("f_{}", uuid::Uuid::new_v4().to_string().split('-').next().unwrap_or("0")),
            content: content.to_string(),
            category,
            confidence,
            created_at: Utc::now(),
            source: source.to_string(),
            reinforcement_count: 1,
        };

        self.data.facts.push(fact);

        // Evict lowest-confidence facts if over limit
        if self.data.facts.len() > self.max_facts {
            self.data.facts.sort_by(|a, b| {
                b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal)
            });
            self.data.facts.truncate(self.max_facts);
        }

        self.dirty = true;
        true // New fact added
    }

    /// Get top N facts sorted by confidence (highest first).
    pub fn top_facts(&self, n: usize) -> Vec<&Fact> {
        let mut sorted: Vec<&Fact> = self.data.facts.iter().collect();
        sorted.sort_by(|a, b| {
            b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal)
        });
        sorted.truncate(n);
        sorted
    }

    /// Build a prompt context string from top facts (DeerFlow-style injection).
    ///
    /// Format: `<memory>` tags wrapping user context + facts, capped at `max_tokens` chars.
    pub fn to_prompt_context(&self, max_chars: usize) -> Option<String> {
        if self.data.facts.is_empty() && self.data.user_context.work_context.is_empty() {
            return None;
        }

        let mut ctx = String::from("<memory>\n");

        // Inject user context if available
        if !self.data.user_context.work_context.is_empty() {
            ctx.push_str(&format!("Work: {}\n", self.data.user_context.work_context));
        }
        if !self.data.user_context.personal_context.is_empty() {
            ctx.push_str(&format!("Personal: {}\n", self.data.user_context.personal_context));
        }
        if !self.data.user_context.top_of_mind.is_empty() {
            ctx.push_str(&format!("Priority: {}\n", self.data.user_context.top_of_mind));
        }

        // Inject top facts
        let top = self.top_facts(15);
        if !top.is_empty() {
            ctx.push_str("\nKnown facts:\n");
            for fact in &top {
                let entry = format!(
                    "- [{}] {} (confidence: {:.0}%)\n",
                    fact.category,
                    fact.content,
                    fact.confidence * 100.0
                );
                if ctx.len() + entry.len() > max_chars - 12 {
                    break;
                }
                ctx.push_str(&entry);
            }
        }

        ctx.push_str("</memory>");
        Some(ctx)
    }

    /// Update user context.
    pub fn update_context(&mut self, work: Option<&str>, personal: Option<&str>, top_of_mind: Option<&str>) {
        if let Some(w) = work {
            self.data.user_context.work_context = w.to_string();
        }
        if let Some(p) = personal {
            self.data.user_context.personal_context = p.to_string();
        }
        if let Some(t) = top_of_mind {
            self.data.user_context.top_of_mind = t.to_string();
        }
        self.dirty = true;
    }

    /// Facts count.
    pub fn len(&self) -> usize {
        self.data.facts.len()
    }

    /// Whether the store is empty.
    pub fn is_empty(&self) -> bool {
        self.data.facts.is_empty()
    }

    /// Persist to disk (atomic: write to temp, then rename).
    pub fn save(&mut self) -> std::result::Result<(), String> {
        let path = self.storage_path.as_ref().ok_or("No storage path configured")?;

        if !self.dirty {
            return Ok(());
        }

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("mkdir: {e}"))?;
        }

        // Atomic write: temp file + rename
        let tmp_path = path.with_extension("tmp");
        let json = serde_json::to_string_pretty(&self.data).map_err(|e| format!("serialize: {e}"))?;
        std::fs::write(&tmp_path, &json).map_err(|e| format!("write: {e}"))?;
        std::fs::rename(&tmp_path, path).map_err(|e| format!("rename: {e}"))?;

        self.dirty = false;
        Ok(())
    }

    /// Get all facts.
    pub fn facts(&self) -> &[Fact] {
        &self.data.facts
    }

    /// Get user context.
    pub fn user_context(&self) -> &UserContext {
        &self.data.user_context
    }
}

/// Normalize content for deduplication (whitespace collapse + trim + lowercase).
fn normalize_content(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_and_retrieve_facts() {
        let mut store = FactStore::new(100, 0.7);
        assert!(store.add_fact("User prefers Rust", FactCategory::Preference, 0.95, "conversation"));
        assert!(store.add_fact("BizClaw uses axum", FactCategory::Knowledge, 0.9, "code"));
        assert_eq!(store.len(), 2);

        let top = store.top_facts(10);
        assert_eq!(top.len(), 2);
        assert_eq!(top[0].content, "User prefers Rust"); // highest confidence
    }

    #[test]
    fn test_deduplication_by_content() {
        let mut store = FactStore::new(100, 0.7);
        assert!(store.add_fact("User likes dark mode", FactCategory::Preference, 0.8, "conv"));
        // Same content with different whitespace — should be deduplicated
        assert!(!store.add_fact("  User  likes  dark  mode  ", FactCategory::Preference, 0.8, "conv"));
        assert_eq!(store.len(), 1);
        // Confidence should be boosted
        assert!(store.facts()[0].confidence > 0.8);
        assert_eq!(store.facts()[0].reinforcement_count, 2);
    }

    #[test]
    fn test_low_confidence_rejected() {
        let mut store = FactStore::new(100, 0.7);
        assert!(!store.add_fact("Maybe true", FactCategory::Context, 0.3, "guess"));
        assert!(store.is_empty());
    }

    #[test]
    fn test_eviction_at_max_capacity() {
        let mut store = FactStore::new(3, 0.5);
        store.add_fact("Fact A", FactCategory::Knowledge, 0.9, "test");
        store.add_fact("Fact B", FactCategory::Knowledge, 0.7, "test");
        store.add_fact("Fact C", FactCategory::Knowledge, 0.8, "test");
        store.add_fact("Fact D", FactCategory::Knowledge, 0.95, "test"); // should evict lowest

        assert_eq!(store.len(), 3);
        // Lowest confidence (0.7) should have been evicted
        let contents: Vec<&str> = store.facts().iter().map(|f| f.content.as_str()).collect();
        assert!(!contents.contains(&"Fact B"));
    }

    #[test]
    fn test_prompt_context_generation() {
        let mut store = FactStore::new(100, 0.5);
        store.update_context(Some("Building BizClaw platform"), None, Some("Ship v1.1"));
        store.add_fact("User speaks Vietnamese", FactCategory::Preference, 0.95, "conv");
        store.add_fact("Uses Rust + Tokio stack", FactCategory::Knowledge, 0.9, "code");

        let ctx = store.to_prompt_context(2000).unwrap();
        assert!(ctx.starts_with("<memory>"));
        assert!(ctx.ends_with("</memory>"));
        assert!(ctx.contains("Building BizClaw platform"));
        assert!(ctx.contains("User speaks Vietnamese"));
        assert!(ctx.contains("Ship v1.1"));
    }

    #[test]
    fn test_prompt_context_none_when_empty() {
        let store = FactStore::new(100, 0.7);
        assert!(store.to_prompt_context(2000).is_none());
    }

    #[test]
    fn test_fact_categories_display() {
        assert_eq!(FactCategory::Preference.to_string(), "preference");
        assert_eq!(FactCategory::Goal.to_string(), "goal");
        assert_eq!(FactCategory::Knowledge.to_string(), "knowledge");
    }

    #[test]
    fn test_persistence_roundtrip() {
        let tmp_dir = std::env::temp_dir().join("bizclaw_facts_test");
        let _ = std::fs::create_dir_all(&tmp_dir);
        let path = tmp_dir.join("test_facts.json");

        {
            let mut store = FactStore::with_path(100, 0.7, &path);
            store.add_fact("Persistent fact", FactCategory::Knowledge, 0.9, "test");
            store.update_context(Some("Testing"), None, None);
            store.save().unwrap();
        }

        // Load from disk
        let store2 = FactStore::with_path(100, 0.7, &path);
        assert_eq!(store2.len(), 1);
        assert_eq!(store2.facts()[0].content, "Persistent fact");
        assert_eq!(store2.user_context().work_context, "Testing");

        // Cleanup
        let _ = std::fs::remove_dir_all(&tmp_dir);
    }

    #[test]
    fn test_reinforcement_boosts_confidence() {
        let mut store = FactStore::new(100, 0.7);
        store.add_fact("User uses Neovim", FactCategory::Preference, 0.8, "conv");
        store.add_fact("User uses Neovim", FactCategory::Preference, 0.8, "conv");
        store.add_fact("User uses Neovim", FactCategory::Preference, 0.8, "conv");

        assert_eq!(store.len(), 1);
        assert_eq!(store.facts()[0].reinforcement_count, 3);
        // 0.8 + 0.05 + 0.05 = 0.9
        assert!((store.facts()[0].confidence - 0.9).abs() < 0.01);
    }
}
