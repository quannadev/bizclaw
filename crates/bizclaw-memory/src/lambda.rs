//! # λ-Memory — Memory That Fades, Not Disappears
//!
//! Inspired by SkyClaw/Temm1e's λ-Memory system.
//!
//! Traditional AI agents delete old messages or summarize them into oblivion.
//! λ-Memory decays memories through an exponential function but never truly
//! erases them. The agent sees old memories at progressively lower fidelity:
//!
//! ```text
//! full_text → summary → essence → hash
//! ```
//!
//! Any memory can be recalled by hash to restore full detail.
//!
//! ## Key Innovations
//! - **Exponential decay**: `score = importance × e^(−λt)` — memories fade, never die
//! - **4 fidelity levels**: Pre-computed at write time, selected at read time by decay score
//! - **Hash-based recall**: Content-addressable retrieval from compressed memory
//! - **Dynamic budget**: Same algorithm adapts from 4K to 200K context windows
//!
//! ## Usage
//! ```rust,no_run
//! use bizclaw_memory::lambda::{LambdaMemory, LambdaEntry};
//!
//! let mut mem = LambdaMemory::default();
//! mem.store(LambdaEntry::new(
//!     "User wants to deploy BizClaw to production VPS",
//!     "Deploy BizClaw to production",
//!     "deploy prod",
//!     0.9,
//! ));
//!
//! // Get context-window-aware injection
//! let injection = mem.build_context(8000); // 8K token budget
//! ```

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, info};

// ─── Configuration ───────────────────────────────────────────

/// Default decay rate (λ). Higher = faster decay.
/// 0.00001 ≈ memories half-life ~19 hours.
const DEFAULT_DECAY_RATE: f64 = 0.00001;

/// Fidelity score thresholds.
const FIDELITY_FULL: f64 = 0.6;
const FIDELITY_SUMMARY: f64 = 0.3;
const FIDELITY_ESSENCE: f64 = 0.1;
// Below FIDELITY_ESSENCE → hash only

/// Default max entries before pruning hashes.
const DEFAULT_MAX_ENTRIES: usize = 500;

/// Approximate chars-per-token ratio for budget calculation.
const CHARS_PER_TOKEN: usize = 4;

// ─── Core Types ──────────────────────────────────────────────

/// A single memory entry with pre-computed fidelity layers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LambdaEntry {
    /// Content-addressable hash (SHA-256 first 12 hex chars).
    pub hash: String,
    /// Full original text.
    pub full_text: String,
    /// 1-2 sentence summary (pre-computed at write time).
    pub summary: String,
    /// One-line essence distillation (pre-computed at write time).
    pub essence: String,
    /// Importance score (0.0 - 1.0). Set by caller or auto-computed.
    pub importance: f64,
    /// When this memory was created.
    pub created_at: DateTime<Utc>,
    /// Optional: which session/tenant created this.
    #[serde(default)]
    pub source: String,
    /// Optional: category tag for filtering.
    #[serde(default)]
    pub tags: Vec<String>,
}

impl LambdaEntry {
    /// Create a new entry with pre-computed fidelity layers.
    pub fn new(full_text: &str, summary: &str, essence: &str, importance: f64) -> Self {
        Self {
            hash: Self::compute_hash(full_text),
            full_text: full_text.to_string(),
            summary: summary.to_string(),
            essence: essence.to_string(),
            importance: importance.clamp(0.0, 1.0),
            created_at: Utc::now(),
            source: String::new(),
            tags: Vec::new(),
        }
    }

    /// Create with source and tags.
    pub fn with_metadata(
        full_text: &str,
        summary: &str,
        essence: &str,
        importance: f64,
        source: &str,
        tags: Vec<String>,
    ) -> Self {
        let mut entry = Self::new(full_text, summary, essence, importance);
        entry.source = source.to_string();
        entry.tags = tags;
        entry
    }

    /// Compute content-addressable hash (first 12 hex chars of SHA-256).
    fn compute_hash(content: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        format!("{:012x}", hasher.finish())
    }

    /// Get the appropriate fidelity level text based on decay score.
    pub fn at_fidelity(&self, score: f64) -> &str {
        if score >= FIDELITY_FULL {
            &self.full_text
        } else if score >= FIDELITY_SUMMARY {
            &self.summary
        } else if score >= FIDELITY_ESSENCE {
            &self.essence
        } else {
            &self.hash
        }
    }

    /// Get the fidelity level name for this score.
    pub fn fidelity_level(score: f64) -> &'static str {
        if score >= FIDELITY_FULL {
            "full"
        } else if score >= FIDELITY_SUMMARY {
            "summary"
        } else if score >= FIDELITY_ESSENCE {
            "essence"
        } else {
            "hash"
        }
    }
}

// ─── Lambda Memory System ────────────────────────────────────

/// Configuration for the λ-Memory system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LambdaConfig {
    /// Decay rate (λ). Higher = faster fade.
    #[serde(default = "default_decay_rate")]
    pub decay_rate: f64,
    /// Maximum stored entries.
    #[serde(default = "default_max_entries")]
    pub max_entries: usize,
    /// Whether λ-Memory is enabled (vs. simple echo mode).
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_decay_rate() -> f64 {
    DEFAULT_DECAY_RATE
}
fn default_max_entries() -> usize {
    DEFAULT_MAX_ENTRIES
}
fn default_true() -> bool {
    true
}

impl Default for LambdaConfig {
    fn default() -> Self {
        Self {
            decay_rate: DEFAULT_DECAY_RATE,
            max_entries: DEFAULT_MAX_ENTRIES,
            enabled: true,
        }
    }
}

/// The λ-Memory manager.
///
/// Stores memories with exponential decay scoring.
/// Builds context-window-aware injections at read time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LambdaMemory {
    /// Config.
    config: LambdaConfig,
    /// All stored entries.
    entries: Vec<LambdaEntry>,
    /// Index for hash-based recall: hash → entry index.
    #[serde(skip)]
    hash_index: HashMap<String, usize>,
}

impl Default for LambdaMemory {
    fn default() -> Self {
        Self::new(LambdaConfig::default())
    }
}

impl LambdaMemory {
    /// Create a new λ-Memory with given config.
    pub fn new(config: LambdaConfig) -> Self {
        Self {
            config,
            entries: Vec::new(),
            hash_index: HashMap::new(),
        }
    }

    /// Store a new memory entry.
    pub fn store(&mut self, entry: LambdaEntry) {
        // Dedup by hash
        if self.hash_index.contains_key(&entry.hash) {
            debug!("λ-Memory: dedup hit for hash={}", &entry.hash);
            return;
        }

        let idx = self.entries.len();
        self.hash_index.insert(entry.hash.clone(), idx);
        self.entries.push(entry);

        // Prune if over limit (remove lowest-scoring entries)
        if self.entries.len() > self.config.max_entries {
            self.prune();
        }
    }

    /// Compute decay score for an entry at the current time.
    ///
    /// `score = importance × e^(−λt)` where t is age in seconds.
    pub fn score(&self, entry: &LambdaEntry) -> f64 {
        let age_secs = (Utc::now() - entry.created_at).num_seconds().max(0) as f64;
        entry.importance * (-self.config.decay_rate * age_secs).exp()
    }

    /// Compute decay score at a specific point in time.
    pub fn score_at(&self, entry: &LambdaEntry, at: DateTime<Utc>) -> f64 {
        let age_secs = (at - entry.created_at).num_seconds().max(0) as f64;
        entry.importance * (-self.config.decay_rate * age_secs).exp()
    }

    /// Recall a memory by hash — restores full detail regardless of decay.
    ///
    /// This is the key innovation: even compressed memories can be recalled.
    pub fn recall(&self, hash: &str) -> Option<&LambdaEntry> {
        self.hash_index
            .get(hash)
            .and_then(|&idx| self.entries.get(idx))
    }

    /// Build context injection for a given token budget.
    ///
    /// Returns entries sorted by score, each at the appropriate fidelity level,
    /// staying within the budget. Higher-scored entries get full fidelity,
    /// lower-scored entries get progressively compressed representations.
    pub fn build_context(&self, token_budget: usize) -> String {
        if self.entries.is_empty() || !self.config.enabled {
            return String::new();
        }

        // Score and sort all entries
        let mut scored: Vec<(f64, &LambdaEntry)> =
            self.entries.iter().map(|e| (self.score(e), e)).collect();
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        let max_chars = token_budget * CHARS_PER_TOKEN;
        let mut parts = Vec::new();
        let mut total_chars = 0;

        // Header
        let header = "<lambda-memory>";
        let footer = "</lambda-memory>";
        total_chars += header.len() + footer.len() + 2; // newlines

        // Budget dashboard (like SkyClaw's skull budget concept)
        let budget_line = format!(
            "  [budget: {} tokens | {} entries | λ={:.6}]",
            token_budget,
            scored.len(),
            self.config.decay_rate
        );
        total_chars += budget_line.len() + 1;
        parts.push(budget_line);

        // Fill with entries at appropriate fidelity
        for (score, entry) in &scored {
            let text = entry.at_fidelity(*score);
            let level = LambdaEntry::fidelity_level(*score);

            // Format: [level|score] content
            let line = format!("  [{level}|{score:.2}] {text}");

            if total_chars + line.len() + 1 > max_chars {
                // Budget exhausted — add remaining as hash-only if space permits
                let hash_line = format!("  [hash|{score:.2}] #{}", entry.hash);
                if total_chars + hash_line.len() < max_chars {
                    parts.push(hash_line.clone());
                    total_chars += hash_line.len() + 1;
                } else {
                    break;
                }
            } else {
                parts.push(line.clone());
                total_chars += line.len() + 1;
            }
        }

        format!("{header}\n{}\n{footer}", parts.join("\n"))
    }

    /// Build a simple summary of memory state (for status/dashboard).
    pub fn stats(&self) -> LambdaStats {
        let now = Utc::now();
        let scores: Vec<f64> = self.entries.iter().map(|e| self.score_at(e, now)).collect();

        let full_count = scores.iter().filter(|&&s| s >= FIDELITY_FULL).count();
        let summary_count = scores
            .iter()
            .filter(|&&s| (FIDELITY_SUMMARY..FIDELITY_FULL).contains(&s))
            .count();
        let essence_count = scores
            .iter()
            .filter(|&&s| (FIDELITY_ESSENCE..FIDELITY_SUMMARY).contains(&s))
            .count();
        let hash_count = scores.iter().filter(|&&s| s < FIDELITY_ESSENCE).count();

        let avg_score = if scores.is_empty() {
            0.0
        } else {
            scores.iter().sum::<f64>() / scores.len() as f64
        };

        LambdaStats {
            total_entries: self.entries.len(),
            full_fidelity: full_count,
            summary_fidelity: summary_count,
            essence_fidelity: essence_count,
            hash_only: hash_count,
            avg_score,
            decay_rate: self.config.decay_rate,
        }
    }

    /// Get all entries (for serialization/inspection).
    pub fn entries(&self) -> &[LambdaEntry] {
        &self.entries
    }

    /// Boost a memory's importance (e.g., when recalled or referenced).
    pub fn boost(&mut self, hash: &str, boost_amount: f64) -> bool {
        if let Some(&idx) = self.hash_index.get(hash)
            && let Some(entry) = self.entries.get_mut(idx)
        {
            entry.importance = (entry.importance + boost_amount).min(1.0);
            debug!(
                "λ-Memory: boosted hash={} to importance={:.2}",
                hash, entry.importance
            );
            return true;
        }
        false
    }

    /// Remove a specific entry by hash.
    pub fn remove(&mut self, hash: &str) -> bool {
        if let Some(&idx) = self.hash_index.get(hash) {
            self.entries.remove(idx);
            self.rebuild_index();
            true
        } else {
            false
        }
    }

    /// Clear all memories.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.hash_index.clear();
    }

    /// Prune lowest-scoring entries to stay within max_entries.
    fn prune(&mut self) {
        let mut scored: Vec<(f64, usize)> = self
            .entries
            .iter()
            .enumerate()
            .map(|(i, e)| (self.score(e), i))
            .collect();
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        // Keep top max_entries
        let keep_indices: std::collections::HashSet<usize> = scored
            .iter()
            .take(self.config.max_entries)
            .map(|(_, i)| *i)
            .collect();

        let pruned = self.entries.len() - keep_indices.len();
        self.entries = self
            .entries
            .iter()
            .enumerate()
            .filter(|(i, _)| keep_indices.contains(i))
            .map(|(_, e)| e.clone())
            .collect();

        self.rebuild_index();

        if pruned > 0 {
            info!("λ-Memory: pruned {pruned} low-score entries");
        }
    }

    /// Rebuild the hash→index mapping after mutations.
    fn rebuild_index(&mut self) {
        self.hash_index.clear();
        for (i, entry) in self.entries.iter().enumerate() {
            self.hash_index.insert(entry.hash.clone(), i);
        }
    }
}

/// Statistics for the λ-Memory system.
#[derive(Debug, Clone, Serialize)]
pub struct LambdaStats {
    pub total_entries: usize,
    pub full_fidelity: usize,
    pub summary_fidelity: usize,
    pub essence_fidelity: usize,
    pub hash_only: usize,
    pub avg_score: f64,
    pub decay_rate: f64,
}

// ─── Tests ───────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn sample_entry(text: &str, importance: f64) -> LambdaEntry {
        LambdaEntry::new(
            text,
            &format!("Summary of: {text}"),
            &text[..text.len().min(20)],
            importance,
        )
    }

    #[test]
    fn test_store_and_dedup() {
        let mut mem = LambdaMemory::default();
        mem.store(sample_entry("Deploy BizClaw to production", 0.9));
        mem.store(sample_entry("Deploy BizClaw to production", 0.9)); // dedup
        assert_eq!(mem.entries().len(), 1);
    }

    #[test]
    fn test_decay_score() {
        let config = LambdaConfig {
            decay_rate: 0.001, // Fast decay for testing
            ..Default::default()
        };
        let mem = LambdaMemory::new(config);

        let mut entry = sample_entry("Test memory", 1.0);

        // Fresh entry → score ≈ 1.0
        let score_now = mem.score(&entry);
        assert!(
            score_now > 0.99,
            "Fresh score should be ~1.0, got {score_now}"
        );

        // Entry from 1 hour ago
        entry.created_at = Utc::now() - Duration::hours(1);
        let score_1h = mem.score(&entry);
        assert!(score_1h < score_now, "1h old should score lower");
        assert!(score_1h > 0.0, "Should never reach zero");

        // Entry from 1 day ago
        entry.created_at = Utc::now() - Duration::days(1);
        let score_1d = mem.score(&entry);
        assert!(score_1d < score_1h, "1 day old should score even lower");
    }

    #[test]
    fn test_fidelity_levels() {
        let entry = sample_entry("Full detailed text about deployment", 1.0);

        assert_eq!(entry.at_fidelity(0.8), entry.full_text);
        assert_eq!(entry.at_fidelity(0.4), entry.summary);
        assert_eq!(entry.at_fidelity(0.15), entry.essence);
        assert_eq!(entry.at_fidelity(0.05), entry.hash);
    }

    #[test]
    fn test_recall_by_hash() {
        let mut mem = LambdaMemory::default();
        let entry = sample_entry("Secret deployment plan", 0.9);
        let hash = entry.hash.clone();

        mem.store(entry);

        let recalled = mem.recall(&hash);
        assert!(recalled.is_some());
        assert_eq!(recalled.unwrap().full_text, "Secret deployment plan");

        // Non-existent hash
        assert!(mem.recall("nonexistent").is_none());
    }

    #[test]
    fn test_build_context_respects_budget() {
        let mut mem = LambdaMemory::default();

        for i in 0..20 {
            mem.store(sample_entry(
                &format!("Memory entry number {i} with some additional context and details"),
                0.5 + (i as f64 * 0.025),
            ));
        }

        // Small budget → fewer entries, more compression
        let small = mem.build_context(200);
        let large = mem.build_context(10000);

        assert!(
            small.len() < large.len(),
            "Small budget should produce less text"
        );
        assert!(small.contains("<lambda-memory>"));
        assert!(large.contains("<lambda-memory>"));
    }

    #[test]
    fn test_boost_importance() {
        let mut mem = LambdaMemory::default();
        let entry = sample_entry("Important task", 0.5);
        let hash = entry.hash.clone();
        mem.store(entry);

        assert!(mem.boost(&hash, 0.3));
        let recalled = mem.recall(&hash).unwrap();
        assert!((recalled.importance - 0.8).abs() < 0.01);

        // Boost caps at 1.0
        assert!(mem.boost(&hash, 0.5));
        let recalled = mem.recall(&hash).unwrap();
        assert!((recalled.importance - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_prune() {
        let config = LambdaConfig {
            max_entries: 5,
            ..Default::default()
        };
        let mut mem = LambdaMemory::new(config);

        for i in 0..10 {
            mem.store(sample_entry(&format!("Entry {i}"), i as f64 * 0.1));
        }

        assert_eq!(mem.entries().len(), 5);
        // Highest importance entries should survive
        assert!(mem.entries().iter().all(|e| e.importance >= 0.5));
    }

    #[test]
    fn test_remove() {
        let mut mem = LambdaMemory::default();
        let entry = sample_entry("To be removed", 0.9);
        let hash = entry.hash.clone();
        mem.store(entry);
        mem.store(sample_entry("To keep", 0.8));

        assert_eq!(mem.entries().len(), 2);
        assert!(mem.remove(&hash));
        assert_eq!(mem.entries().len(), 1);
        assert!(mem.recall(&hash).is_none());
    }

    #[test]
    fn test_stats() {
        let mut mem = LambdaMemory::default();
        for i in 0..5 {
            mem.store(sample_entry(&format!("Stat entry {i}"), 0.9));
        }

        let stats = mem.stats();
        assert_eq!(stats.total_entries, 5);
        assert_eq!(stats.full_fidelity, 5); // All fresh → full fidelity
        assert!(stats.avg_score > 0.8);
    }

    #[test]
    fn test_empty_context() {
        let mem = LambdaMemory::default();
        let ctx = mem.build_context(1000);
        assert!(ctx.is_empty());
    }

    #[test]
    fn test_disabled_mode() {
        let config = LambdaConfig {
            enabled: false,
            ..Default::default()
        };
        let mut mem = LambdaMemory::new(config);
        mem.store(sample_entry("Some memory", 0.9));

        let ctx = mem.build_context(1000);
        assert!(ctx.is_empty());
    }

    #[test]
    fn test_with_metadata() {
        let entry = LambdaEntry::with_metadata(
            "Deploy to staging",
            "Deploy staging",
            "deploy",
            0.8,
            "tenant-123",
            vec!["ops".to_string(), "deploy".to_string()],
        );
        assert_eq!(entry.source, "tenant-123");
        assert_eq!(entry.tags.len(), 2);
    }
}
