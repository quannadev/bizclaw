//! Layer 1: Hot KV Storage using redb
//! 
//! RsClaw-style persistent key-value store with:
//! - ACID transactions
//! - Sub-millisecond reads
//! - Automatic compaction
//! - Cross-session persistence

mod store;
mod types;

pub use store::RedbStore;
pub use types::*;
