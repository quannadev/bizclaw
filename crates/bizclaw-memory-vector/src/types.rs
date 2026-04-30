//! Type definitions for vector storage

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorDocument {
    pub id: String,
    pub vector: Vec<f32>,
    pub metadata: DocumentMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentMetadata {
    pub doc_type: String,
    pub content: String,
    pub title: Option<String>,
    pub tags: Vec<String>,
    pub source: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub access_count: u64,
}

impl VectorDocument {
    pub fn new(id: String, vector: Vec<f32>, content: String) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            id,
            vector,
            metadata: DocumentMetadata {
                doc_type: "memory".to_string(),
                content,
                title: None,
                tags: Vec::new(),
                source: None,
                created_at: now,
                updated_at: now,
                access_count: 0,
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum DistanceMetric {
    Cosine,
    Euclidean,
    DotProduct,
}

impl Default for DistanceMetric {
    fn default() -> Self {
        Self::Cosine
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub id: String,
    pub score: f32,
    pub metadata: DocumentMetadata,
}

impl SearchResult {
    pub fn distance(&self) -> f32 {
        self.score
    }
    
    pub fn similarity(&self) -> f32 {
        1.0 - self.score
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchOptions {
    pub limit: usize,
    pub ef_construction: usize,
    pub m: usize,
    pub metric: DistanceMetric,
    pub include_vectors: bool,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            limit: 10,
            ef_construction: 200,
            m: 16,
            metric: DistanceMetric::Cosine,
            include_vectors: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexStats {
    pub dimension: usize,
    pub total_vectors: usize,
    pub max_level: usize,
    pub metric: DistanceMetric,
    pub size_bytes: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchResult {
    pub indexed: usize,
    pub failed: usize,
    pub errors: Vec<String>,
}
