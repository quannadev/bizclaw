//! Marketplace API Routes
//! 
//! REST API endpoints for Skills Marketplace

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};

use crate::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/marketplace/stats", get(get_stats))
        .route("/api/marketplace/categories", get(get_categories))
        .route("/api/marketplace/featured", get(get_featured))
        .route("/api/marketplace/skills", get(search_skills))
        .route("/api/marketplace/skills/:slug", get(get_skill))
        .route("/api/marketplace/skills/:slug/install", post(install_skill))
        .route("/api/marketplace/skills/:slug/reviews", get(get_reviews))
        .route("/api/marketplace/skills/:slug/reviews", post(create_review))
}

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub q: Option<String>,
    pub category: Option<String>,
    pub tags: Option<String>,
    pub sort: Option<String>,
    pub page: Option<u32>,
    pub per_page: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: T,
    pub error: Option<String>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data,
            error: None,
        }
    }

    pub fn error(message: String) -> Self {
        Self {
            success: false,
            data: panic!("Cannot create error response without data"),
            error: Some(message),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct StatsResponse {
    pub total_skills: u64,
    pub total_installs: u64,
    pub total_reviews: u64,
    pub avg_rating: f32,
}

async fn get_stats() -> Json<ApiResponse<StatsResponse>> {
    let stats = StatsResponse {
        total_skills: 50,
        total_installs: 10000,
        total_reviews: 500,
        avg_rating: 4.2,
    };
    Json(ApiResponse::success(stats))
}

#[derive(Debug, Serialize)]
pub struct CategoryResponse {
    pub id: String,
    pub name: String,
    pub description: String,
    pub icon: String,
    pub skill_count: u32,
}

async fn get_categories() -> Json<ApiResponse<Vec<CategoryResponse>>> {
    let categories = vec![
        CategoryResponse {
            id: "developer".to_string(),
            name: "Developer".to_string(),
            description: "Programming and development skills".to_string(),
            icon: "💻".to_string(),
            skill_count: 25,
        },
        CategoryResponse {
            id: "business".to_string(),
            name: "Business".to_string(),
            description: "Business writing and communication".to_string(),
            icon: "💼".to_string(),
            skill_count: 15,
        },
        CategoryResponse {
            id: "creative".to_string(),
            name: "Creative".to_string(),
            description: "Content creation and design".to_string(),
            icon: "🎨".to_string(),
            skill_count: 10,
        },
        CategoryResponse {
            id: "data".to_string(),
            name: "Data".to_string(),
            description: "Data analysis and processing".to_string(),
            icon: "📊".to_string(),
            skill_count: 12,
        },
        CategoryResponse {
            id: "automation".to_string(),
            name: "Automation".to_string(),
            description: "Workflow and process automation".to_string(),
            icon: "⚡".to_string(),
            skill_count: 8,
        },
        CategoryResponse {
            id: "communication".to_string(),
            name: "Communication".to_string(),
            description: "Communication and collaboration".to_string(),
            icon: "💬".to_string(),
            skill_count: 6,
        },
        CategoryResponse {
            id: "research".to_string(),
            name: "Research".to_string(),
            description: "Research and analysis".to_string(),
            icon: "🔬".to_string(),
            skill_count: 5,
        },
        CategoryResponse {
            id: "education".to_string(),
            name: "Education".to_string(),
            description: "Teaching and learning".to_string(),
            icon: "📚".to_string(),
            skill_count: 4,
        },
    ];
    Json(ApiResponse::success(categories))
}

#[derive(Debug, Serialize)]
pub struct FeaturedResponse {
    pub trending: Vec<SkillSummary>,
    pub new: Vec<SkillSummary>,
    pub top_rated: Vec<SkillSummary>,
}

#[derive(Debug, Serialize)]
pub struct SkillSummary {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub description: String,
    pub author_name: String,
    pub author_verified: bool,
    pub category: String,
    pub downloads: u64,
    pub rating: f32,
    pub rating_count: u32,
    pub icon: String,
}

async fn get_featured() -> Json<ApiResponse<FeaturedResponse>> {
    let trending = vec![
        SkillSummary {
            id: "web-developer".to_string(),
            name: "Web Developer".to_string(),
            slug: "web-developer".to_string(),
            description: "Web development assistance, code review, debugging".to_string(),
            author_name: "BizClaw".to_string(),
            author_verified: true,
            category: "developer".to_string(),
            downloads: 1500,
            rating: 4.8,
            rating_count: 120,
            icon: "💻".to_string(),
        },
        SkillSummary {
            id: "python-analyst".to_string(),
            name: "Python Analyst".to_string(),
            slug: "python-analyst".to_string(),
            description: "Python data analysis, ML, automation".to_string(),
            author_name: "BizClaw".to_string(),
            author_verified: true,
            category: "data".to_string(),
            downloads: 1200,
            rating: 4.7,
            rating_count: 95,
            icon: "🐍".to_string(),
        },
        SkillSummary {
            id: "vietnamese-business".to_string(),
            name: "Vietnamese Business".to_string(),
            slug: "vietnamese-business".to_string(),
            description: "Vietnamese business writing and communication".to_string(),
            author_name: "BizClaw".to_string(),
            author_verified: true,
            category: "business".to_string(),
            downloads: 800,
            rating: 4.9,
            rating_count: 65,
            icon: "🇻🇳".to_string(),
        },
    ];
    
    let featured = FeaturedResponse {
        trending: trending.clone(),
        new: trending.clone(),
        top_rated: trending,
    };
    
    Json(ApiResponse::success(featured))
}

#[derive(Debug, Serialize)]
pub struct SearchResponse {
    pub skills: Vec<SkillSummary>,
    pub total: u64,
    pub page: u32,
    pub per_page: u32,
    pub total_pages: u32,
    pub facets: SearchFacetsResponse,
}

#[derive(Debug, Serialize)]
pub struct SearchFacetsResponse {
    pub categories: Vec<FacetValue>,
    pub tags: Vec<FacetValue>,
}

#[derive(Debug, Serialize)]
pub struct FacetValue {
    pub value: String,
    pub count: u64,
}

async fn search_skills(Query(query): Query<SearchQuery>) -> Json<ApiResponse<SearchResponse>> {
    let skills = vec![
        SkillSummary {
            id: "web-developer".to_string(),
            name: "Web Developer".to_string(),
            slug: "web-developer".to_string(),
            description: "Web development assistance, code review, debugging".to_string(),
            author_name: "BizClaw".to_string(),
            author_verified: true,
            category: "developer".to_string(),
            downloads: 1500,
            rating: 4.8,
            rating_count: 120,
            icon: "💻".to_string(),
        },
    ];
    
    let response = SearchResponse {
        skills,
        total: 50,
        page: query.page.unwrap_or(1),
        per_page: query.per_page.unwrap_or(20),
        total_pages: 3,
        facets: SearchFacetsResponse {
            categories: vec![
                FacetValue { value: "developer".to_string(), count: 25 },
                FacetValue { value: "business".to_string(), count: 15 },
            ],
            tags: vec![
                FacetValue { value: "python".to_string(), count: 20 },
                FacetValue { value: "rust".to_string(), count: 15 },
            ],
        },
    };
    
    Json(ApiResponse::success(response))
}

#[derive(Debug, Serialize)]
pub struct SkillDetail {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub description: String,
    pub version: String,
    pub author: AuthorDetail,
    pub category: String,
    pub tags: Vec<String>,
    pub downloads: u64,
    pub rating: f32,
    pub rating_count: u32,
    pub review_count: u32,
    pub verified: bool,
    pub min_bizclaw_version: String,
    pub license: String,
    pub repository: Option<String>,
    pub homepage: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub install_count: u64,
    pub success_rate: f32,
    pub readme: String,
    pub screenshots: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct AuthorDetail {
    pub id: String,
    pub name: String,
    pub username: String,
    pub avatar: Option<String>,
    pub verified: bool,
    pub skill_count: u32,
}

async fn get_skill(Path(slug): Path<String>) -> Json<ApiResponse<SkillDetail>> {
    let skill = SkillDetail {
        id: slug.clone(),
        name: "Web Developer".to_string(),
        slug,
        description: "Web development assistance, code review, debugging. This skill helps you with HTML, CSS, JavaScript, React, Vue, and more.".to_string(),
        version: "1.0.0".to_string(),
        author: AuthorDetail {
            id: "bizclaw".to_string(),
            name: "BizClaw Team".to_string(),
            username: "bizclaw".to_string(),
            avatar: None,
            verified: true,
            skill_count: 10,
        },
        category: "developer".to_string(),
        tags: vec!["web".to_string(), "frontend".to_string(), "backend".to_string()],
        downloads: 1500,
        rating: 4.8,
        rating_count: 120,
        review_count: 95,
        verified: true,
        min_bizclaw_version: "1.1.0".to_string(),
        license: "MIT".to_string(),
        repository: Some("https://github.com/nguyenduchoai/bizclaw".to_string()),
        homepage: None,
        created_at: "2024-06-01T00:00:00Z".to_string(),
        updated_at: "2025-01-15T00:00:00Z".to_string(),
        install_count: 1200,
        success_rate: 0.95,
        readme: "# Web Developer Skill\n\nA comprehensive skill for web development...".to_string(),
        screenshots: vec![],
    };
    
    Json(ApiResponse::success(skill))
}

#[derive(Debug, Deserialize)]
pub struct InstallRequest {
    pub version: Option<String>,
    pub force: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct InstallResponse {
    pub success: bool,
    pub skill_id: String,
    pub version: String,
    pub message: String,
    pub warnings: Vec<String>,
}

async fn install_skill(
    Path(slug): Path<String>,
    Json(req): Json<InstallRequest>,
) -> Json<ApiResponse<InstallResponse>> {
    let response = InstallResponse {
        success: true,
        skill_id: slug,
        version: req.version.unwrap_or_else(|| "1.0.0".to_string()),
        message: "Skill installed successfully!".to_string(),
        warnings: vec![],
    };
    
    Json(ApiResponse::success(response))
}

#[derive(Debug, Serialize)]
pub struct ReviewListResponse {
    pub reviews: Vec<ReviewDetail>,
    pub total: u32,
    pub page: u32,
    pub per_page: u32,
}

#[derive(Debug, Serialize)]
pub struct ReviewDetail {
    pub id: String,
    pub user_name: String,
    pub user_avatar: Option<String>,
    pub rating: u8,
    pub title: String,
    pub content: String,
    pub pros: Vec<String>,
    pub cons: Vec<String>,
    pub helpful_count: u32,
    pub created_at: String,
}

async fn get_reviews(Path(slug): Path<String>) -> Json<ApiResponse<ReviewListResponse>> {
    let reviews = vec![
        ReviewDetail {
            id: "review-1".to_string(),
            user_name: "John Doe".to_string(),
            user_avatar: None,
            rating: 5,
            title: "Excellent skill!".to_string(),
            content: "This skill has saved me hours of work. Highly recommended!".to_string(),
            pros: vec!["Fast responses".to_string(), "Accurate code".to_string()],
            cons: vec![],
            helpful_count: 15,
            created_at: "2025-01-10T00:00:00Z".to_string(),
        },
    ];
    
    let response = ReviewListResponse {
        reviews,
        total: 120,
        page: 1,
        per_page: 20,
    };
    
    Json(ApiResponse::success(response))
}

#[derive(Debug, Deserialize)]
pub struct CreateReviewRequest {
    pub rating: u8,
    pub title: String,
    pub content: String,
    pub pros: Option<Vec<String>>,
    pub cons: Option<Vec<String>>,
}

async fn create_review(
    Path(_slug): Path<String>,
    Json(_req): Json<CreateReviewRequest>,
) -> Json<ApiResponse<ReviewDetail>> {
    let review = ReviewDetail {
        id: "review-new".to_string(),
        user_name: "You".to_string(),
        user_avatar: None,
        rating: 5,
        title: "Great skill!".to_string(),
        content: "Thank you for this amazing skill!".to_string(),
        pros: vec![],
        cons: vec![],
        helpful_count: 0,
        created_at: chrono::Utc::now().to_rfc3339(),
    };
    
    Json(ApiResponse::success(review))
}
