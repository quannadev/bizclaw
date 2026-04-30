//! Type definitions for redb storage

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredValue {
    pub key: String,
    pub value: Vec<u8>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub ttl_seconds: Option<u64>,
    pub access_count: u64,
}

impl StoredValue {
    pub fn new(key: String, value: Vec<u8>) -> Self {
        let now = Utc::now();
        Self {
            key,
            value,
            created_at: now,
            updated_at: now,
            ttl_seconds: None,
            access_count: 0,
        }
    }

    pub fn with_ttl(mut self, ttl_seconds: u64) -> Self {
        self.ttl_seconds = Some(ttl_seconds);
        self
    }

    pub fn is_expired(&self) -> bool {
        if let Some(ttl) = self.ttl_seconds {
            let age = (Utc::now() - self.created_at).num_seconds() as u64;
            age > ttl
        } else {
            false
        }
    }

    pub fn increment_access(&mut self) {
        self.access_count += 1;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyPrefix {
    pub prefix: String,
    pub count: u64,
    pub oldest: DateTime<Utc>,
    pub newest: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DataType {
    Session,
    Memory,
    Config,
    Cache,
    Temporary,
}

impl DataType {
    pub fn prefix(&self) -> &'static str {
        match self {
            DataType::Session => "session:",
            DataType::Memory => "memory:",
            DataType::Config => "config:",
            DataType::Cache => "cache:",
            DataType::Temporary => "temp:",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageStats {
    pub total_keys: u64,
    pub total_size_bytes: u64,
    pub by_type: Vec<(String, u64)>,
    pub oldest_key: Option<DateTime<Utc>>,
    pub newest_key: Option<DateTime<Utc>>,
    pub compaction_candidates: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanOptions {
    pub prefix: Option<String>,
    pub limit: Option<usize>,
    pub include_expired: bool,
    pub order: ScanOrder,
}

impl Default for ScanOptions {
    fn default() -> Self {
        Self {
            prefix: None,
            limit: Some(100),
            include_expired: false,
            order: ScanOrder::Forward,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScanOrder {
    Forward,
    Backward,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchOperation {
    pub puts: Vec<(String, Vec<u8>)>,
    pub deletes: Vec<String>,
    pub update_ttl: Vec<(String, Option<u64>)>,
}

impl BatchOperation {
    pub fn new() -> Self {
        Self {
            puts: Vec::new(),
            deletes: Vec::new(),
            update_ttl: Vec::new(),
        }
    }

    pub fn put(&mut self, key: String, value: Vec<u8>) -> &mut Self {
        self.puts.push((key, value));
        self
    }

    pub fn delete(&mut self, key: String) -> &mut Self {
        self.deletes.push(key);
        self
    }

    pub fn set_ttl(&mut self, key: String, ttl_seconds: Option<u64>) -> &mut Self {
        self.update_ttl.push((key, ttl_seconds));
        self
    }

    pub fn is_empty(&self) -> bool {
        self.puts.is_empty() && self.deletes.is_empty() && self.update_ttl.is_empty()
    }
}

impl Default for BatchOperation {
    fn default() -> Self {
        Self::new()
    }
}
