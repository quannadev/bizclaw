//! # BizClaw Obsidian Integration
//! 
//! Tích hợp Obsidian Vault vào BizClaw cho:
//! - 📁 Vault Management
//! - 🔍 RAG Search
//! - 📝 SKILL.md Management
//! - 🧠 Long-term Memory

pub mod vault;
pub mod search;
pub mod skills;
pub mod memory;

pub use vault::{ObsidianVault, VaultConfig, Note, NoteMetadata};
pub use search::{SearchEngine, SearchResult, SearchQuery};
pub use skills::{SkillManager, SkillReference};
pub use memory::{MemoryStore, MemoryEntry, MemoryType};
