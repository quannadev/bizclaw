//! Layer 2: Full-text search using tantivy
//! 
//! Features:
//! - BM25 ranking algorithm
//! - Fuzzy matching
//! - Phrase queries
//! - Boolean operators
//! - Faceted search
//! - Highlighting

mod index;
mod query;
mod schema;

pub use index::SearchIndex;
pub use query::{SearchQuery, SearchResult, SearchOptions, Highlight};
pub use schema::*;
