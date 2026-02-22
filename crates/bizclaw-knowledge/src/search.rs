//! Search result types.

use serde::{Deserialize, Serialize};

/// A single search result from the knowledge base.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// Document name.
    pub doc_name: String,
    /// Chunk index within the document.
    pub chunk_idx: usize,
    /// The matching text content.
    pub content: String,
    /// BM25 relevance score (lower = more relevant in SQLite FTS5).
    pub score: f64,
}

impl SearchResult {
    /// Format as context for the Agent system prompt.
    pub fn as_context(&self) -> String {
        format!("[📄 {}] {}", self.doc_name, self.content)
    }
}

/// Format multiple search results as Agent context.
pub fn format_knowledge_context(results: &[SearchResult]) -> String {
    if results.is_empty() {
        return String::new();
    }
    let mut ctx = String::from("\n--- Knowledge Base Context ---\n");
    for (i, r) in results.iter().enumerate() {
        ctx.push_str(&format!("[{}] {}\n\n", i + 1, r.as_context()));
    }
    ctx.push_str("--- End Knowledge ---\n");
    ctx
}
