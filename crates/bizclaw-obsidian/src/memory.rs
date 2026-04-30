//! # Long-term Memory Store
//! 
//! Lưu trữ memory dài hạn trong Obsidian vault

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum MemoryType {
    Episodic,
    Semantic,
    Procedural,
    Working,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id: String,
    pub memory_type: MemoryType,
    pub content: String,
    pub summary: String,
    pub importance: f32,
    pub access_count: u32,
    pub last_accessed: DateTime<Utc>,
    pub created: DateTime<Utc>,
    pub metadata: HashMap<String, String>,
    pub linked_entities: Vec<String>,
    pub tags: Vec<String>,
}

pub struct MemoryStore {
    entries: HashMap<String, MemoryEntry>,
}

impl MemoryStore {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    pub async fn add(&mut self, entry: MemoryEntry) {
        self.entries.insert(entry.id.clone(), entry);
    }

    pub fn get(&self, id: &str) -> Option<&MemoryEntry> {
        self.entries.get(id)
    }

    pub async fn get_and_touch(&mut self, id: &str) -> Option<MemoryEntry> {
        if let Some(entry) = self.entries.get_mut(id) {
            entry.access_count += 1;
            entry.last_accessed = Utc::now();
            return Some(entry.clone());
        }
        None
    }

    pub fn search(&self, query: &str) -> Vec<&MemoryEntry> {
        let query_lower = query.to_lowercase();
        self.entries.values()
            .filter(|entry| {
                entry.content.to_lowercase().contains(&query_lower) ||
                entry.summary.to_lowercase().contains(&query_lower)
            })
            .collect()
    }

    pub fn get_recent(&self, limit: usize) -> Vec<&MemoryEntry> {
        let mut entries: Vec<_> = self.entries.values().collect();
        entries.sort_by(|a, b| b.created.cmp(&a.created));
        entries.into_iter().take(limit).collect()
    }

    pub fn count(&self) -> usize {
        self.entries.len()
    }
}

impl Default for MemoryStore {
    fn default() -> Self {
        Self::new()
    }
}
