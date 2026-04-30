//! Query types and result handling

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    pub query_string: String,
    pub filters: Option<SearchFilters>,
    pub page: usize,
    pub per_page: usize,
}

impl Default for SearchQuery {
    fn default() -> Self {
        Self {
            query_string: String::new(),
            filters: None,
            page: 1,
            per_page: 20,
        }
    }
}

impl SearchQuery {
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            query_string: query.into(),
            ..Default::default()
        }
    }
    
    pub fn with_filters(mut self, filters: SearchFilters) -> Self {
        self.filters = Some(filters);
        self
    }
    
    pub fn with_page(mut self, page: usize) -> Self {
        self.page = page;
        self
    }
    
    pub fn with_per_page(mut self, per_page: usize) -> Self {
        self.per_page = per_page;
        self
    }
    
    pub fn offset(&self) -> usize {
        (self.page.saturating_sub(1)) * self.per_page
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchFilters {
    pub doc_types: Option<Vec<String>>,
    pub session_id: Option<String>,
    pub date_from: Option<i64>,
    pub date_to: Option<i64>,
    pub tags: Option<Vec<String>>,
    pub author: Option<String>,
    pub min_access_count: Option<i64>,
}

impl Default for SearchFilters {
    fn default() -> Self {
        Self {
            doc_types: None,
            session_id: None,
            date_from: None,
            date_to: None,
            tags: None,
            author: None,
            min_access_count: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchOptions {
    pub fuzzy: bool,
    pub fuzzy_distance: usize,
    pub boost_title: f32,
    pub boost_recent: f32,
    pub highlight_fragments: usize,
    pub include_snippets: bool,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            fuzzy: false,
            fuzzy_distance: 2,
            boost_title: 2.0,
            boost_recent: 1.0,
            highlight_fragments: 3,
            include_snippets: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub id: String,
    pub doc_type: String,
    pub title: Option<String>,
    pub content: String,
    pub tags: Vec<String>,
    pub score: f32,
    pub highlights: Vec<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub session_id: Option<String>,
    pub access_count: i64,
}

impl SearchResult {
    pub fn snippet(&self, max_len: usize) -> String {
        if self.content.len() <= max_len {
            return self.content.clone();
        }
        
        let mut snippet = self.content.chars().take(max_len - 3).collect::<String>();
        snippet.push_str("...");
        snippet
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    pub query: String,
    pub total: usize,
    pub page: usize,
    pub per_page: usize,
    pub total_pages: usize,
    pub results: Vec<SearchResult>,
    pub facets: Vec<FacetResult>,
    pub took_ms: u64,
}

impl SearchResponse {
    pub fn new(query: String, results: Vec<SearchResult>, total: usize, took_ms: u64) -> Self {
        let page = if results.is_empty() { 0 } else { 1 };
        Self {
            query,
            total,
            page,
            per_page: results.len(),
            total_pages: 0,
            results,
            facets: Vec::new(),
            took_ms,
        }
    }
    
    pub fn with_pagination(mut self, page: usize, per_page: usize) -> Self {
        self.page = page;
        self.per_page = per_page;
        self.total_pages = (self.total + per_page - 1) / per_page;
        self
    }
    
    pub fn with_facets(mut self, facets: Vec<FacetResult>) -> Self {
        self.facets = facets;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FacetResult {
    pub name: String,
    pub values: Vec<FacetValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FacetValue {
    pub value: String,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Highlight {
    pub pre_tag: String,
    pub post_tag: String,
    pub fragment_size: usize,
    pub number_of_fragments: usize,
}

impl Default for Highlight {
    fn default() -> Self {
        Self {
            pre_tag: "<mark>".to_string(),
            post_tag: "</mark>".to_string(),
            fragment_size: 150,
            number_of_fragments: 3,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutocompleteResult {
    pub suggestions: Vec<Suggestion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Suggestion {
    pub text: String,
    pub score: f32,
    pub doc_count: usize,
}
