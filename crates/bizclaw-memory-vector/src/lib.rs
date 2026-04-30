//! Layer 3: Vector/semantic search using HNSW
//! 
//! Features:
//! - HNSW (Hierarchical Navigable Small World) index
//! - Approximate nearest neighbor search
//! - Cosine/Euclidean distance metrics
//! - Batch indexing
//! - Incremental updates

mod hnsw;
mod index;
mod types;

pub use hnsw::*;
pub use index::VectorIndex;
pub use types::*;
