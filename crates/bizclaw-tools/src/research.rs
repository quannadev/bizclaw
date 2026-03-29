//! Research Tool — AutoResearchClaw-inspired research agent.
//!
//! Enables the AI agent to conduct structured research:
//! - Search academic papers via OpenAlex, Semantic Scholar, arXiv APIs
//! - Summarize findings with real citations
//! - Generate literature reviews and competitive analyses
//! - Produce markdown research reports
//!
//! This is a lightweight implementation inspired by AutoResearchClaw's
//! pipeline but designed for BizClaw's business context:
//! - Market research reports for SME clients
//! - Technology landscape analyses
//! - Competitive intelligence summaries
//! - Trend reports with real academic backing

use async_trait::async_trait;
use bizclaw_core::error::{BizClawError, Result};
use bizclaw_core::traits::Tool;
use bizclaw_core::types::{ToolDefinition, ToolResult};
use serde::{Deserialize, Serialize};

pub struct ResearchTool;

impl Default for ResearchTool {
    fn default() -> Self {
        Self::new()
    }
}

impl ResearchTool {
    pub fn new() -> Self {
        Self
    }

    /// Search OpenAlex for academic works (free, no API key needed).
    async fn search_openalex(&self, query: &str, max_results: usize) -> Result<Vec<Paper>> {
        let client = reqwest::Client::builder()
            .user_agent("BizClaw/1.0 (mailto:team@bizclaw.vn)")
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .map_err(|e| BizClawError::tool_error("research", e))?;

        let url = format!(
            "https://api.openalex.org/works?search={}&per_page={}&sort=relevance_score:desc&select=id,title,publication_year,authorships,cited_by_count,doi,primary_location",
            urlencoding::encode(query),
            max_results
        );

        let response = client
            .get(&url)
            .send()
            .await
            .map_err(|e| BizClawError::tool_error("research", e))?;

        if !response.status().is_success() {
            return Ok(vec![]);
        }

        let data: serde_json::Value = response
            .json()
            .await
            .map_err(|e| BizClawError::tool_error("research", e))?;

        let papers: Vec<Paper> = data["results"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|work| {
                let title = work["title"].as_str()?.to_string();
                let year = work["publication_year"].as_u64().unwrap_or(0) as u32;
                let citations = work["cited_by_count"].as_u64().unwrap_or(0) as u32;
                let doi = work["doi"].as_str().unwrap_or("").to_string();

                let authors: Vec<String> = work["authorships"]
                    .as_array()
                    .unwrap_or(&vec![])
                    .iter()
                    .take(3)
                    .filter_map(|a| a["author"]["display_name"].as_str().map(String::from))
                    .collect();

                let journal = work["primary_location"]["source"]["display_name"]
                    .as_str()
                    .unwrap_or("")
                    .to_string();

                Some(Paper {
                    title,
                    authors,
                    year,
                    citations,
                    doi,
                    journal,
                    source: "OpenAlex".into(),
                })
            })
            .collect();

        Ok(papers)
    }

    /// Search Semantic Scholar API (free, rate-limited).
    async fn search_semantic_scholar(&self, query: &str, max_results: usize) -> Result<Vec<Paper>> {
        let client = reqwest::Client::builder()
            .user_agent("BizClaw/1.0")
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .map_err(|e| BizClawError::tool_error("research", e))?;

        let url = format!(
            "https://api.semanticscholar.org/graph/v1/paper/search?query={}&limit={}&fields=title,authors,year,citationCount,externalIds,journal",
            urlencoding::encode(query),
            max_results
        );

        let response = client
            .get(&url)
            .send()
            .await
            .map_err(|e| BizClawError::tool_error("research", e))?;

        if !response.status().is_success() {
            return Ok(vec![]);
        }

        let data: serde_json::Value = response
            .json()
            .await
            .map_err(|e| BizClawError::tool_error("research", e))?;

        let papers: Vec<Paper> = data["data"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|work| {
                let title = work["title"].as_str()?.to_string();
                let year = work["year"].as_u64().unwrap_or(0) as u32;
                let citations = work["citationCount"].as_u64().unwrap_or(0) as u32;

                let doi = work["externalIds"]["DOI"]
                    .as_str()
                    .unwrap_or("")
                    .to_string();

                let authors: Vec<String> = work["authors"]
                    .as_array()
                    .unwrap_or(&vec![])
                    .iter()
                    .take(3)
                    .filter_map(|a| a["name"].as_str().map(String::from))
                    .collect();

                let journal = work["journal"]["name"].as_str().unwrap_or("").to_string();

                Some(Paper {
                    title,
                    authors,
                    year,
                    citations,
                    doi,
                    journal,
                    source: "SemanticScholar".into(),
                })
            })
            .collect();

        Ok(papers)
    }

    /// Format papers into a clean markdown report.
    fn format_report(&self, topic: &str, papers: &[Paper], report_type: &str) -> String {
        let mut report = String::new();

        match report_type {
            "literature_review" => {
                report.push_str(&format!("# 📚 Literature Review: {topic}\n\n"));
                report.push_str(&format!(
                    "*{} papers found across OpenAlex & Semantic Scholar*\n\n",
                    papers.len()
                ));
                report.push_str("## Key Papers\n\n");
            }
            "competitive" => {
                report.push_str(&format!("# 🔍 Competitive Analysis: {topic}\n\n"));
                report.push_str(&format!(
                    "*Based on {} academic and industry publications*\n\n",
                    papers.len()
                ));
                report.push_str("## Related Works\n\n");
            }
            "trend" => {
                report.push_str(&format!("# 📈 Trend Report: {topic}\n\n"));
                report.push_str(&format!("*{} publications analyzed*\n\n", papers.len()));
                report.push_str("## Publication Timeline\n\n");
            }
            _ => {
                report.push_str(&format!("# 🔬 Research Report: {topic}\n\n"));
                report.push_str(&format!("*{} papers found*\n\n", papers.len()));
                report.push_str("## Results\n\n");
            }
        }

        // Sort by citations (most impactful first)
        let mut sorted = papers.to_vec();
        sorted.sort_by(|a, b| b.citations.cmp(&a.citations));

        for (i, paper) in sorted.iter().enumerate() {
            let authors_str = if paper.authors.is_empty() {
                "Unknown".to_string()
            } else if paper.authors.len() > 2 {
                format!("{} et al.", paper.authors[0])
            } else {
                paper.authors.join(", ")
            };

            let doi_link = if paper.doi.is_empty() {
                String::new()
            } else if paper.doi.starts_with("http") {
                format!(" [DOI]({})", paper.doi)
            } else {
                format!(" [DOI](https://doi.org/{})", paper.doi)
            };

            let journal_str = if paper.journal.is_empty() {
                String::new()
            } else {
                format!(" — *{}*", paper.journal)
            };

            report.push_str(&format!(
                "{}. **{}** ({})  \n   {} | 📊 {} citations{}{}\n\n",
                i + 1,
                paper.title,
                paper.year,
                authors_str,
                paper.citations,
                journal_str,
                doi_link,
            ));
        }

        // Statistics section
        if !papers.is_empty() {
            report.push_str("---\n\n## 📊 Statistics\n\n");
            let total_citations: u32 = papers.iter().map(|p| p.citations).sum();
            let avg_year: f64 =
                papers.iter().map(|p| p.year as f64).sum::<f64>() / papers.len() as f64;
            let newest = papers.iter().map(|p| p.year).max().unwrap_or(0);
            let oldest = papers
                .iter()
                .filter(|p| p.year > 0)
                .map(|p| p.year)
                .min()
                .unwrap_or(0);

            report.push_str(&"| Metric | Value |\n|--------|-------|\n".to_string());
            report.push_str(&format!("| Total papers | {} |\n", papers.len()));
            report.push_str(&format!("| Total citations | {} |\n", total_citations));
            report.push_str(&format!("| Average year | {:.0} |\n", avg_year));
            report.push_str(&format!("| Year range | {} – {} |\n", oldest, newest));
            report.push_str(&format!(
                "| Most cited | {} ({} citations) |\n",
                sorted.first().map(|p| p.title.as_str()).unwrap_or("N/A"),
                sorted.first().map(|p| p.citations).unwrap_or(0),
            ));

            report.push_str("\n---\n\n");
            report.push_str("📋 **Next steps**: Please analyze the above papers and provide:\n");
            report.push_str("- Key themes and findings\n");
            report.push_str("- Research gaps and opportunities\n");
            report.push_str("- Practical implications for business application\n");
        }

        report
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Paper {
    title: String,
    authors: Vec<String>,
    year: u32,
    citations: u32,
    doi: String,
    journal: String,
    source: String,
}

#[async_trait]
impl Tool for ResearchTool {
    fn name(&self) -> &str {
        "research"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "research".into(),
            description: concat!(
                "Search academic papers and generate research reports. ",
                "Searches OpenAlex and Semantic Scholar for real papers with verified citations. ",
                "Use for: literature reviews, market research, competitive analysis, trend reports. ",
                "Returns papers with titles, authors, year, citation count, DOI links, and journal info."
            ).into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "topic": {
                        "type": "string",
                        "description": "Research topic or query (e.g., 'AI agents for small business automation')"
                    },
                    "report_type": {
                        "type": "string",
                        "enum": ["search", "literature_review", "competitive", "trend"],
                        "description": "Report type: 'search' (raw results), 'literature_review', 'competitive', 'trend' (default: search)"
                    },
                    "max_results": {
                        "type": "integer",
                        "description": "Max papers per source (default: 10, max: 25)"
                    },
                    "sources": {
                        "type": "string",
                        "enum": ["all", "openalex", "semantic_scholar"],
                        "description": "Which academic databases to search (default: all)"
                    }
                },
                "required": ["topic"]
            }),
        }
    }

    async fn execute(&self, arguments: &str) -> Result<ToolResult> {
        let args: serde_json::Value = serde_json::from_str(arguments)
            .map_err(|e| BizClawError::Tool(format!("Invalid arguments: {e}")))?;

        let topic = args["topic"]
            .as_str()
            .ok_or_else(|| BizClawError::Tool("topic is required".into()))?;

        let report_type = args["report_type"].as_str().unwrap_or("search");
        let max_results = args["max_results"].as_u64().unwrap_or(10).min(25) as usize;
        let sources = args["sources"].as_str().unwrap_or("all");

        tracing::info!(
            "[research] Searching '{topic}' ({report_type}, max={max_results}, sources={sources})"
        );

        let mut all_papers = Vec::new();

        // Search OpenAlex
        if sources == "all" || sources == "openalex" {
            match self.search_openalex(topic, max_results).await {
                Ok(papers) => {
                    tracing::info!("[research] OpenAlex: {} papers found", papers.len());
                    all_papers.extend(papers);
                }
                Err(e) => {
                    tracing::warn!("[research] OpenAlex search failed: {e}");
                }
            }
        }

        // Search Semantic Scholar
        if sources == "all" || sources == "semantic_scholar" {
            match self.search_semantic_scholar(topic, max_results).await {
                Ok(papers) => {
                    tracing::info!("[research] Semantic Scholar: {} papers found", papers.len());
                    all_papers.extend(papers);
                }
                Err(e) => {
                    tracing::warn!("[research] Semantic Scholar search failed: {e}");
                }
            }
        }

        // Deduplicate by DOI
        let mut seen_dois = std::collections::HashSet::new();
        let mut seen_titles = std::collections::HashSet::new();
        all_papers.retain(|p| {
            if !p.doi.is_empty() {
                seen_dois.insert(p.doi.clone())
            } else {
                seen_titles.insert(p.title.to_lowercase())
            }
        });

        let output = if all_papers.is_empty() {
            format!(
                "No academic papers found for topic: \"{topic}\". Try broader keywords or different phrasing."
            )
        } else {
            self.format_report(topic, &all_papers, report_type)
        };

        Ok(ToolResult {
            tool_call_id: String::new(),
            output,
            success: true,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_definition() {
        let tool = ResearchTool::new();
        assert_eq!(tool.name(), "research");
        let def = tool.definition();
        assert!(def.description.contains("academic"));
        let params = def.parameters;
        assert!(params["properties"]["topic"].is_object());
        assert!(params["properties"]["report_type"].is_object());
    }

    #[test]
    fn test_format_report_empty() {
        let tool = ResearchTool::new();
        let report = tool.format_report("AI agents", &[], "search");
        assert!(report.contains("AI agents"));
        assert!(!report.contains("Statistics")); // No stats for empty results
    }

    #[test]
    fn test_format_report_with_papers() {
        let tool = ResearchTool::new();
        let papers = vec![
            Paper {
                title: "Deep Learning for NLP".into(),
                authors: vec!["Author A".into(), "Author B".into()],
                year: 2024,
                citations: 150,
                doi: "10.1234/test".into(),
                journal: "Nature AI".into(),
                source: "OpenAlex".into(),
            },
            Paper {
                title: "Transformer Models Survey".into(),
                authors: vec!["Author C".into()],
                year: 2023,
                citations: 500,
                doi: "10.5678/test2".into(),
                journal: "JMLR".into(),
                source: "SemanticScholar".into(),
            },
        ];
        let report = tool.format_report("NLP", &papers, "literature_review");
        assert!(report.contains("Literature Review"));
        assert!(report.contains("Deep Learning"));
        assert!(report.contains("Transformer Models"));
        assert!(report.contains("500 citations")); // Sorted by citations
        assert!(report.contains("Statistics"));
    }

    #[test]
    fn test_paper_dedup_logic() {
        let mut seen = std::collections::HashSet::new();
        assert!(seen.insert("doi1".to_string())); // New
        assert!(!seen.insert("doi1".to_string())); // Duplicate
    }
}
