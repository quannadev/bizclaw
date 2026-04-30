//! Skills Marketplace - ClawHub-style skill registry
//! 
//! Features:
//! - Skill registration and metadata
//! - Search and filtering
//! - Version management
//! - Rating and reviews
//! - Categories and tags
//! - Install/uninstall workflows

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub author: SkillAuthor,
    pub category: SkillCategory,
    pub tags: Vec<String>,
    pub files: Vec<SkillFile>,
    pub dependencies: Vec<SkillDependency>,
    pub metadata: SkillMetadata,
    pub stats: SkillStats,
    pub reviews: Vec<Review>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillAuthor {
    pub id: String,
    pub name: String,
    pub avatar: Option<String>,
    pub verified: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SkillCategory {
    Developer,
    Business,
    Creative,
    Data,
    Automation,
    Communication,
    Research,
    Education,
    Other,
}

impl SkillCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            SkillCategory::Developer => "developer",
            SkillCategory::Business => "business",
            SkillCategory::Creative => "creative",
            SkillCategory::Data => "data",
            SkillCategory::Automation => "automation",
            SkillCategory::Communication => "communication",
            SkillCategory::Research => "research",
            SkillCategory::Education => "education",
            SkillCategory::Other => "other",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillFile {
    pub path: String,
    pub size: u64,
    pub checksum: String,
    pub file_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDependency {
    pub name: String,
    pub version: String,
    pub optional: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillMetadata {
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub downloads: u64,
    pub rating: f32,
    pub rating_count: u32,
    pub min_bizclaw_version: String,
    pub license: String,
    pub repository: Option<String>,
    pub homepage: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillStats {
    pub installs: u64,
    pub active_users: u64,
    pub avg_response_time_ms: u64,
    pub success_rate: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Review {
    pub id: String,
    pub user_id: String,
    pub user_name: String,
    pub rating: u8,
    pub comment: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillSearchQuery {
    pub query: Option<String>,
    pub category: Option<SkillCategory>,
    pub tags: Option<Vec<String>>,
    pub author: Option<String>,
    pub min_rating: Option<f32>,
    pub sort_by: SortOption,
    pub page: u32,
    pub per_page: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SortOption {
    Popular,
    Recent,
    Rating,
    Name,
    Downloads,
}

impl Default for SkillSearchQuery {
    fn default() -> Self {
        Self {
            query: None,
            category: None,
            tags: None,
            author: None,
            min_rating: None,
            sort_by: SortOption::Popular,
            page: 1,
            per_page: 20,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub skills: Vec<Skill>,
    pub total: u64,
    pub page: u32,
    pub per_page: u32,
    pub total_pages: u32,
    pub facets: SearchFacets,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchFacets {
    pub categories: Vec<FacetCount>,
    pub tags: Vec<FacetCount>,
    pub authors: Vec<FacetCount>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FacetCount {
    pub value: String,
    pub count: u64,
}

pub struct Marketplace {
    skills: HashMap<String, Skill>,
    categories: HashMap<SkillCategory, Vec<String>>,
    tags: HashMap<String, Vec<String>>,
}

impl Marketplace {
    pub fn new() -> Self {
        Self {
            skills: HashMap::new(),
            categories: HashMap::new(),
            tags: HashMap::new(),
        }
    }

    pub fn register(&mut self, skill: Skill) -> Result<(), String> {
        if self.skills.contains_key(&skill.id) {
            return Err(format!("Skill {} already exists", skill.id));
        }

        // Add to categories
        self.categories
            .entry(skill.category.clone())
            .or_insert_with(Vec::new)
            .push(skill.id.clone());

        // Add to tags
        for tag in &skill.tags {
            self.tags
                .entry(tag.clone())
                .or_insert_with(Vec::new)
                .push(skill.id.clone());
        }

        self.skills.insert(skill.id.clone(), skill);
        Ok(())
    }

    pub fn get(&self, id: &str) -> Option<&Skill> {
        self.skills.get(id)
    }

    pub fn search(&self, query: SkillSearchQuery) -> SearchResult {
        let mut results: Vec<&Skill> = self.skills.values().collect();

        // Filter by query
        if let Some(ref q) = query.query {
            let q_lower = q.to_lowercase();
            results.retain(|s| {
                s.name.to_lowercase().contains(&q_lower)
                    || s.description.to_lowercase().contains(&q_lower)
                    || s.tags.iter().any(|t| t.to_lowercase().contains(&q_lower))
            });
        }

        // Filter by category
        if let Some(ref cat) = query.category {
            results.retain(|s| s.category == *cat);
        }

        // Filter by tags
        if let Some(ref tags) = query.tags {
            results.retain(|s| tags.iter().all(|t| s.tags.contains(t)));
        }

        // Filter by author
        if let Some(ref author) = query.author {
            results.retain(|s| s.author.id == *author);
        }

        // Filter by rating
        if let Some(min_rating) = query.min_rating {
            results.retain(|s| s.metadata.rating >= min_rating);
        }

        // Sort
        match query.sort_by {
            SortOption::Popular => results.sort_by(|a, b| {
                b.metadata.downloads.cmp(&a.metadata.downloads)
            }),
            SortOption::Recent => results.sort_by(|a, b| {
                b.metadata.updated_at.cmp(&a.metadata.updated_at)
            }),
            SortOption::Rating => results.sort_by(|a, b| {
                b.metadata.rating.partial_cmp(&a.metadata.rating).unwrap()
            }),
            SortOption::Name => results.sort_by(|a, b| a.name.cmp(&b.name)),
            SortOption::Downloads => results.sort_by(|a, b| {
                b.stats.installs.cmp(&a.stats.installs)
            }),
        }

        let total = results.len() as u64;
        let total_pages = ((total as f32) / (query.per_page as f32)).ceil() as u32;

        // Pagination
        let offset = ((query.page - 1) * query.per_page) as usize;
        let skills: Vec<Skill> = results
            .into_iter()
            .skip(offset)
            .take(query.per_page as usize)
            .cloned()
            .collect();

        SearchResult {
            skills,
            total,
            page: query.page,
            per_page: query.per_page,
            total_pages,
            facets: SearchFacets {
                categories: self.get_category_facets(),
                tags: self.get_tag_facets(),
                authors: Vec::new(),
            },
        }
    }

    pub fn list_by_category(&self, category: &SkillCategory) -> Vec<&Skill> {
        self.categories
            .get(category)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.skills.get(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn list_by_tag(&self, tag: &str) -> Vec<&Skill> {
        self.tags
            .get(tag)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.skills.get(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn list_popular(&self, limit: usize) -> Vec<&Skill> {
        let mut skills: Vec<&Skill> = self.skills.values().collect();
        skills.sort_by(|a, b| b.metadata.downloads.cmp(&a.metadata.downloads));
        skills.truncate(limit);
        skills
    }

    pub fn list_recent(&self, limit: usize) -> Vec<&Skill> {
        let mut skills: Vec<&Skill> = self.skills.values().collect();
        skills.sort_by(|a, b| b.metadata.created_at.cmp(&a.metadata.created_at));
        skills.truncate(limit);
        skills
    }

    fn get_category_facets(&self) -> Vec<FacetCount> {
        self.categories
            .iter()
            .map(|(cat, ids)| FacetCount {
                value: cat.as_str().to_string(),
                count: ids.len() as u64,
            })
            .collect()
    }

    fn get_tag_facets(&self) -> Vec<FacetCount> {
        self.tags
            .iter()
            .map(|(tag, ids)| FacetCount {
                value: tag.clone(),
                count: ids.len() as u64,
            })
            .collect()
    }

    pub fn add_review(&mut self, skill_id: &str, review: Review) -> Result<(), String> {
        let skill = self.skills.get_mut(skill_id)
            .ok_or_else(|| "Skill not found".to_string())?;
        
        skill.reviews.push(review);
        
        // Update average rating
        let total: u32 = skill.reviews.iter().map(|r| r.rating as u32).sum();
        skill.metadata.rating_count = skill.reviews.len() as u32;
        skill.metadata.rating = total as f32 / skill.metadata.rating_count as f32;
        
        Ok(())
    }

    pub fn increment_downloads(&mut self, skill_id: &str) -> Result<(), String> {
        let skill = self.skills.get_mut(skill_id)
            .ok_or_else(|| "Skill not found".to_string())?;
        
        skill.metadata.downloads += 1;
        skill.stats.installs += 1;
        
        Ok(())
    }
}

impl Default for Marketplace {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_skill() -> Skill {
        Skill {
            id: "test-skill".to_string(),
            name: "Test Skill".to_string(),
            description: "A test skill for testing".to_string(),
            version: "1.0.0".to_string(),
            author: SkillAuthor {
                id: "author1".to_string(),
                name: "Test Author".to_string(),
                avatar: None,
                verified: true,
            },
            category: SkillCategory::Developer,
            tags: vec!["test".to_string(), "example".to_string()],
            files: vec![],
            dependencies: vec![],
            metadata: SkillMetadata {
                created_at: Utc::now(),
                updated_at: Utc::now(),
                downloads: 100,
                rating: 4.5,
                rating_count: 10,
                min_bizclaw_version: "1.0.0".to_string(),
                license: "MIT".to_string(),
                repository: None,
                homepage: None,
            },
            stats: SkillStats {
                installs: 50,
                active_users: 10,
                avg_response_time_ms: 100,
                success_rate: 0.95,
            },
            reviews: vec![],
        }
    }

    #[test]
    fn test_register_and_get() {
        let mut marketplace = Marketplace::new();
        let skill = create_test_skill();
        
        marketplace.register(skill.clone()).unwrap();
        
        let retrieved = marketplace.get("test-skill").unwrap();
        assert_eq!(retrieved.name, "Test Skill");
    }

    #[test]
    fn test_search() {
        let mut marketplace = Marketplace::new();
        marketplace.register(create_test_skill()).unwrap();
        
        let results = marketplace.search(SkillSearchQuery {
            query: Some("test".to_string()),
            ..Default::default()
        });
        
        assert_eq!(results.total, 1);
    }

    #[test]
    fn test_category_filter() {
        let mut marketplace = Marketplace::new();
        marketplace.register(create_test_skill()).unwrap();
        
        let results = marketplace.search(SkillSearchQuery {
            category: Some(SkillCategory::Developer),
            ..Default::default()
        });
        
        assert_eq!(results.total, 1);
    }
}
