//! Multi-model embeddings — use multiple embedding models for better retrieval.
//!
//! Multi-model approach:
//! - Run embeddings through 2+ models simultaneously
//! - Combine scores using `dis_max` strategy (best of N)
//! - Don't force users to pick one model — let the system figure it out
//!
//! ## Design
//! ```text
//! Query: "chính sách nghỉ phép"
//!   ↓
//! Model 1 (nomic-embed-text)  → cosine_sim = 0.82
//! Model 2 (bge-m3)            → cosine_sim = 0.78
//!   ↓ dis_max: take max score
//! Final vector score = 0.82
//!   ↓ hybrid with BM25
//! Combined score = 0.3 * bm25 + 0.7 * 0.82
//! ```
//!
//! This is useful because different models are good at different things:
//! - `nomic-embed-text` → general English/Vietnamese
//! - `bge-m3` → multilingual, better for Vietnamese
//! - `all-minilm` → fast, small, great for edge devices

use crate::embeddings::{EmbeddingClient, EmbeddingConfig};
use std::collections::HashMap;

/// Multi-model embedding manager.
/// Coordinates multiple embedding clients in parallel.
pub struct MultiModelEmbedder {
    /// Named embedding clients.
    clients: Vec<(String, EmbeddingClient)>,
    /// Strategy for combining scores.
    pub strategy: CombineStrategy,
}

/// How to combine scores from multiple models.
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum CombineStrategy {
    /// Take the maximum score (dis_max approach).
    #[default]
    DisMax,
    /// Average all scores.
    Average,
    /// Weighted average — first model gets higher weight.
    WeightedFirst,
}

/// Result from multi-model embedding with per-model scores.
#[derive(Debug, Clone)]
pub struct MultiEmbedResult {
    /// Combined embedding vector (from the best-performing model).
    pub embedding: Vec<f32>,
    /// Score per model (model_name → cosine_similarity).
    pub model_scores: HashMap<String, f32>,
    /// Final combined score.
    pub combined_score: f32,
    /// Which model produced the best embedding.
    pub best_model: String,
}

impl MultiModelEmbedder {
    /// Create a multi-model embedder from a list of configs.
    pub fn new(configs: Vec<(String, EmbeddingConfig)>) -> Self {
        let clients = configs
            .into_iter()
            .map(|(name, cfg)| (name, EmbeddingClient::new(cfg)))
            .collect();

        Self {
            clients,
            strategy: CombineStrategy::DisMax,
        }
    }

    /// Create from a single config (backward compatible).
    pub fn single(name: &str, config: EmbeddingConfig) -> Self {
        Self::new(vec![(name.to_string(), config)])
    }

    /// Auto-detect available Ollama models for embeddings.
    /// Checks which embedding models are actually pulled on the local Ollama.
    pub async fn auto_detect_ollama(endpoint: &str) -> Vec<(String, EmbeddingConfig)> {
        let known_embed_models = [
            "nomic-embed-text",
            "bge-m3",
            "all-minilm",
            "mxbai-embed-large",
            "snowflake-arctic-embed",
        ];

        let url = format!("{}/api/tags", endpoint.trim_end_matches('/'));
        let client = reqwest::Client::new();

        let models: Vec<String> = match client.get(&url).send().await {
            Ok(resp) => {
                if let Ok(body) = resp.json::<serde_json::Value>().await {
                    body["models"]
                        .as_array()
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|m| m["name"].as_str().map(String::from))
                                .collect()
                        })
                        .unwrap_or_default()
                } else {
                    Vec::new()
                }
            }
            Err(_) => Vec::new(),
        };

        let mut configs = Vec::new();
        for model_name in &models {
            // Check if this is a known embedding model
            let base_name = model_name.split(':').next().unwrap_or(model_name);
            if known_embed_models.contains(&base_name) {
                configs.push((
                    base_name.to_string(),
                    EmbeddingConfig {
                        provider: "ollama".into(),
                        endpoint: endpoint.to_string(),
                        model: model_name.clone(),
                        api_key: String::new(),
                    },
                ));
            }
        }

        if configs.is_empty() {
            tracing::debug!("No embedding models found on Ollama at {}", endpoint);
        } else {
            tracing::info!(
                "🔍 Auto-detected {} embedding model(s): {}",
                configs.len(),
                configs
                    .iter()
                    .map(|(n, _)| n.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }

        configs
    }

    /// Get the number of models.
    pub fn model_count(&self) -> usize {
        self.clients.len()
    }

    /// Get model names.
    pub fn model_names(&self) -> Vec<&str> {
        self.clients.iter().map(|(name, _)| name.as_str()).collect()
    }

    /// Check if any models are available.
    pub fn is_available(&self) -> bool {
        self.clients.iter().any(|(_, c)| c.is_available())
    }

    /// Embed text using all available models.
    /// Returns embeddings keyed by model name.
    pub async fn embed_all(&mut self, text: &str) -> Result<HashMap<String, Vec<f32>>, String> {
        let mut results = HashMap::new();
        let mut errors = Vec::new();

        // Run models sequentially (for edge devices with limited memory)
        // On more powerful hardware, this could be parallelized with tokio::join!
        for (name, client) in &mut self.clients {
            if !client.is_available() {
                continue;
            }
            match client.embed_one(text).await {
                Ok(embedding) => {
                    results.insert(name.clone(), embedding);
                }
                Err(e) => {
                    tracing::warn!("⚠️ Model '{}' embedding failed: {}", name, e);
                    errors.push(format!("{}: {}", name, e));
                }
            }
        }

        if results.is_empty() {
            Err(format!(
                "All embedding models failed: {}",
                errors.join("; ")
            ))
        } else {
            Ok(results)
        }
    }

    /// Embed text with the primary (first available) model.
    /// Use this when you just need one embedding (for storage).
    pub async fn embed_primary(&mut self, text: &str) -> Result<Vec<f32>, String> {
        for (name, client) in &mut self.clients {
            if !client.is_available() {
                continue;
            }
            match client.embed_one(text).await {
                Ok(embedding) => return Ok(embedding),
                Err(e) => {
                    tracing::warn!("⚠️ Primary model '{}' failed: {}, trying next...", name, e);
                }
            }
        }
        Err("No embedding model available".into())
    }

    /// Combine scores from multiple models using the configured strategy.
    pub fn combine_scores(&self, scores: &HashMap<String, f32>) -> f32 {
        if scores.is_empty() {
            return 0.0;
        }

        match self.strategy {
            CombineStrategy::DisMax => {
                // Take the maximum score — dis_max approach
                scores.values().cloned().fold(0.0f32, f32::max)
            }
            CombineStrategy::Average => {
                let sum: f32 = scores.values().sum();
                sum / scores.len() as f32
            }
            CombineStrategy::WeightedFirst => {
                // First model gets 60% weight, rest share 40%
                let values: Vec<f32> = scores.values().cloned().collect();
                if values.len() == 1 {
                    return values[0];
                }
                let first_weight = 0.6;
                let rest_weight = 0.4 / (values.len() - 1) as f32;
                let mut combined = values[0] * first_weight;
                for v in &values[1..] {
                    combined += v * rest_weight;
                }
                combined
            }
        }
    }

    /// Get the best model name from a set of scores.
    pub fn best_model<'a>(&self, scores: &'a HashMap<String, f32>) -> Option<&'a str> {
        scores
            .iter()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(name, _)| name.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dis_max_strategy() {
        let embedder = MultiModelEmbedder {
            clients: Vec::new(),
            strategy: CombineStrategy::DisMax,
        };

        let mut scores = HashMap::new();
        scores.insert("model_a".to_string(), 0.8f32);
        scores.insert("model_b".to_string(), 0.6f32);
        scores.insert("model_c".to_string(), 0.9f32);

        assert!((embedder.combine_scores(&scores) - 0.9).abs() < 0.001);
    }

    #[test]
    fn test_average_strategy() {
        let embedder = MultiModelEmbedder {
            clients: Vec::new(),
            strategy: CombineStrategy::Average,
        };

        let mut scores = HashMap::new();
        scores.insert("a".to_string(), 0.6f32);
        scores.insert("b".to_string(), 0.8f32);

        assert!((embedder.combine_scores(&scores) - 0.7).abs() < 0.001);
    }

    #[test]
    fn test_best_model() {
        let embedder = MultiModelEmbedder {
            clients: Vec::new(),
            strategy: CombineStrategy::DisMax,
        };

        let mut scores = HashMap::new();
        scores.insert("nomic".to_string(), 0.7f32);
        scores.insert("bge".to_string(), 0.9f32);

        assert_eq!(embedder.best_model(&scores), Some("bge"));
    }

    #[test]
    fn test_single_model() {
        let config = EmbeddingConfig::default();
        let embedder = MultiModelEmbedder::single("test", config);
        assert_eq!(embedder.model_count(), 1);
        assert_eq!(embedder.model_names(), vec!["test"]);
    }

    #[test]
    fn test_empty_scores() {
        let embedder = MultiModelEmbedder {
            clients: Vec::new(),
            strategy: CombineStrategy::DisMax,
        };
        assert_eq!(embedder.combine_scores(&HashMap::new()), 0.0);
    }
}
