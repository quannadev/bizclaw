//! # Simple Search Engine
//! 
//! Basic keyword search without Tantivy for simplicity

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct SearchQuery {
    pub text: String,
    pub limit: usize,
}

impl Default for SearchQuery {
    fn default() -> Self {
        Self {
            text: String::new(),
            limit: 10,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub note_id: String,
    pub title: String,
    pub snippet: String,
    pub score: f32,
    pub folder: String,
}

pub struct SearchEngine {
    notes: Vec<SimpleNote>,
}

#[derive(Debug, Clone)]
struct SimpleNote {
    pub id: String,
    pub title: String,
    pub content: String,
    pub folder: String,
}

impl SearchEngine {
    pub fn new() -> Self {
        Self { notes: Vec::new() }
    }

    pub fn index_notes(&mut self, notes: Vec<(String, String, String, String)>) {
        self.notes.clear();
        for (id, title, content, folder) in notes {
            self.notes.push(SimpleNote {
                id,
                title,
                content,
                folder,
            });
        }
    }

    pub fn search(&self, query: SearchQuery) -> Vec<SearchResult> {
        let query_lower = query.text.to_lowercase();
        if query_lower.is_empty() {
            return Vec::new();
        }

        let mut results: Vec<(f32, SearchResult)> = self.notes
            .iter()
            .filter_map(|note| {
                let title_match = note.title.to_lowercase().contains(&query_lower);
                let content_match = note.content.to_lowercase().contains(&query_lower);
                
                if !title_match && !content_match {
                    return None;
                }

                let score = if title_match { 2.0 } else { 0.0 } 
                    + if content_match { 1.0 } else { 0.0 };

                let snippet = Self::create_snippet(&note.content, &query.text, 150);

                Some((score, SearchResult {
                    note_id: note.id.clone(),
                    title: note.title.clone(),
                    snippet,
                    score,
                    folder: note.folder.clone(),
                }))
            })
            .collect();

        results.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        
        results.into_iter()
            .map(|(_, r)| r)
            .take(query.limit)
            .collect()
    }

    fn create_snippet(content: &str, query: &str, max_len: usize) -> String {
        let content_lower = content.to_lowercase();
        let query_lower = query.to_lowercase();
        
        if let Some(pos) = content_lower.find(&query_lower) {
            let start = pos.saturating_sub(50);
            let end = (pos + query.len() + 100).min(content.len());
            let mut snippet: String = content.chars().skip(start).take(end - start).collect();
            
            if start > 0 {
                snippet = format!("...{}", snippet);
            }
            if end < content.len() {
                snippet = format!("{}...", snippet);
            }
            
            snippet
        } else {
            let end = max_len.min(content.len());
            let snippet: String = content.chars().take(end).collect();
            if end < content.len() {
                format!("{}...", snippet)
            } else {
                snippet
            }
        }
    }
}

impl Default for SearchEngine {
    fn default() -> Self {
        Self::new()
    }
}
