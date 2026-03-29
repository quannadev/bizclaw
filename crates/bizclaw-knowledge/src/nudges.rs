//! Nudges engine — proactive suggestions based on knowledge base.
//!
//! Instead of waiting for user questions, nudges analyze the knowledge base
//! and conversation context to suggest relevant information proactively.
//!
//! ## Design Principles
//! - Privacy first: all processing is local, no data leaves the device
//! - Agent adapts to workflow: nudges are contextual, not generic
//! - Composable: nudges can be consumed by any agent in the orchestrator
//!
//! ## How it works
//! ```text
//! User message: "Chuẩn bị meeting với client ABC"
//!   ↓
//! NudgeEngine.generate_nudges(message, conversation_history)
//!   ↓ Analyzes KB for relevant docs
//!   ↓ Extracts key entities & topics
//!   ↓ Generates contextual suggestions
//! Nudges:
//!   💡 "Bạn có muốn xem lại hợp đồng ABC-2024?"
//!   💡 "Ghi chú từ meeting trước: cần follow-up về pricing"
//!   💡 "Liên hệ: Nguyễn Văn A - 0123456789 (phụ trách ABC)"
//! ```

use crate::search::SearchResult;
use serde::{Deserialize, Serialize};

/// A nudge — a proactive suggestion for the user.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Nudge {
    /// Unique ID for this nudge.
    pub id: String,
    /// The suggestion text.
    pub text: String,
    /// Category: "document", "contact", "action", "reminder", "insight"
    pub category: NudgeCategory,
    /// Relevance score (0.0 - 1.0).
    pub relevance: f64,
    /// Source document name (if applicable).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_doc: Option<String>,
    /// Suggested action the user can take.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<NudgeAction>,
}

/// Categories of nudges.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum NudgeCategory {
    /// Related document found in knowledge base.
    Document,
    /// Follow-up action suggested.
    Action,
    /// Relevant insight from past data.
    Insight,
    /// Related question the user might want to ask.
    Question,
}

/// An action that can be triggered from a nudge.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NudgeAction {
    /// Action type: "search", "open_doc", "ask_question"
    pub action_type: String,
    /// Payload for the action (e.g., search query, doc ID).
    pub payload: String,
}

/// Nudge engine configuration.
#[derive(Debug, Clone)]
pub struct NudgeConfig {
    /// Maximum number of nudges to generate per request.
    pub max_nudges: usize,
    /// Minimum relevance score to include a nudge.
    pub min_relevance: f64,
    /// Whether to generate follow-up questions.
    pub suggest_questions: bool,
}

impl Default for NudgeConfig {
    fn default() -> Self {
        Self {
            max_nudges: 3,
            min_relevance: 0.15,
            suggest_questions: true,
        }
    }
}

/// Nudge engine — generates proactive suggestions from the knowledge base.
///
/// This is intentionally lightweight: it uses keyword extraction and pattern
/// matching rather than LLM calls, so it can run on edge devices (Pi, Android).
pub struct NudgeEngine {
    config: NudgeConfig,
    /// Recent nudge history to avoid duplicates.
    recent_nudges: Vec<String>,
}

impl NudgeEngine {
    pub fn new(config: NudgeConfig) -> Self {
        Self {
            config,
            recent_nudges: Vec::new(),
        }
    }

    /// Generate nudges based on search results and user context.
    ///
    /// This is the main entry point. Call it after a knowledge search
    /// to produce proactive suggestions.
    pub fn generate_nudges(
        &mut self,
        user_message: &str,
        search_results: &[SearchResult],
        conversation_context: Option<&str>,
    ) -> Vec<Nudge> {
        let mut nudges = Vec::new();

        // 1. Document nudges — related docs found in KB
        self.add_document_nudges(user_message, search_results, &mut nudges);

        // 2. Insight nudges — patterns found in search results
        self.add_insight_nudges(user_message, search_results, &mut nudges);

        // 3. Question nudges — follow-up questions
        if self.config.suggest_questions {
            self.add_question_nudges(
                user_message,
                search_results,
                conversation_context,
                &mut nudges,
            );
        }

        // Sort by relevance, take top N
        nudges.sort_by(|a, b| {
            b.relevance
                .partial_cmp(&a.relevance)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        nudges.truncate(self.config.max_nudges);

        // Filter out recently shown nudges
        nudges.retain(|n| !self.recent_nudges.contains(&n.id));

        // Remember these nudges
        for n in &nudges {
            self.recent_nudges.push(n.id.clone());
        }
        // Keep recent history bounded
        if self.recent_nudges.len() > 50 {
            self.recent_nudges.drain(..25);
        }

        nudges
    }

    /// Add nudges for documents that are highly relevant but not directly answering the query.
    fn add_document_nudges(
        &self,
        _user_message: &str,
        search_results: &[SearchResult],
        nudges: &mut Vec<Nudge>,
    ) {
        // Group results by document, find docs with multiple matching chunks
        let mut doc_scores: std::collections::HashMap<String, (f64, usize, String)> =
            std::collections::HashMap::new();

        for r in search_results {
            let entry = doc_scores
                .entry(r.doc_name.clone())
                .or_insert((0.0, 0, String::new()));
            entry.0 += r.score.abs();
            entry.1 += 1;
            if entry.2.is_empty() {
                // Store first snippet for context
                entry.2 = r.content.chars().take(100).collect();
            }
        }

        // Documents with multiple matching chunks are likely very relevant
        for (doc_name, (total_score, match_count, snippet)) in &doc_scores {
            if *match_count >= 2 {
                let relevance = (*total_score / *match_count as f64).min(1.0);
                if relevance >= self.config.min_relevance {
                    nudges.push(Nudge {
                        id: format!("doc_{}", doc_name.replace('.', "_")),
                        text: format!(
                            "📄 Tài liệu \"{}\" có {} đoạn liên quan: \"{}...\"",
                            doc_name, match_count, snippet
                        ),
                        category: NudgeCategory::Document,
                        relevance,
                        source_doc: Some(doc_name.clone()),
                        action: Some(NudgeAction {
                            action_type: "open_doc".into(),
                            payload: doc_name.clone(),
                        }),
                    });
                }
            }
        }
    }

    /// Add insight nudges from patterns in the search results.
    fn add_insight_nudges(
        &self,
        user_message: &str,
        search_results: &[SearchResult],
        nudges: &mut Vec<Nudge>,
    ) {
        // Look for numbers, dates, and key entities in results
        let message_lower = user_message.to_lowercase();

        for r in search_results.iter().take(3) {
            let content_lower = r.content.to_lowercase();

            // Detect financial data mentions
            if (message_lower.contains("doanh")
                || message_lower.contains("revenue")
                || message_lower.contains("chi phí")
                || message_lower.contains("giá"))
                && contains_numbers(&content_lower)
            {
                nudges.push(Nudge {
                    id: format!("insight_financial_{}", r.chunk_idx),
                    text: format!(
                        "💰 Dữ liệu tài chính liên quan trong \"{}\": {}",
                        r.doc_name,
                        extract_snippet(&r.content, 120)
                    ),
                    category: NudgeCategory::Insight,
                    relevance: r.score.abs().min(1.0) * 0.8,
                    source_doc: Some(r.doc_name.clone()),
                    action: Some(NudgeAction {
                        action_type: "search".into(),
                        payload: format!("{} {}", user_message, r.doc_name),
                    }),
                });
            }

            // Detect date/deadline mentions
            if contains_date_pattern(&content_lower) {
                nudges.push(Nudge {
                    id: format!("insight_date_{}", r.chunk_idx),
                    text: format!(
                        "📅 Có thông tin ngày/deadline trong \"{}\": {}",
                        r.doc_name,
                        extract_snippet(&r.content, 120)
                    ),
                    category: NudgeCategory::Insight,
                    relevance: r.score.abs().min(1.0) * 0.7,
                    source_doc: Some(r.doc_name.clone()),
                    action: None,
                });
            }
        }
    }

    /// Add follow-up question nudges.
    fn add_question_nudges(
        &self,
        user_message: &str,
        search_results: &[SearchResult],
        _conversation_context: Option<&str>,
        nudges: &mut Vec<Nudge>,
    ) {
        let message_lower = user_message.to_lowercase();

        // Generate contextual follow-up questions based on the topic
        let questions = generate_follow_up_questions(&message_lower, search_results);

        for (i, question) in questions.iter().enumerate().take(2) {
            nudges.push(Nudge {
                id: format!("question_{}_{}", hash_string(&message_lower), i),
                text: format!("❓ {}", question),
                category: NudgeCategory::Question,
                relevance: 0.5 - (i as f64 * 0.1),
                source_doc: None,
                action: Some(NudgeAction {
                    action_type: "ask_question".into(),
                    payload: question.clone(),
                }),
            });
        }
    }

    /// Clear nudge history (e.g., on new conversation).
    pub fn clear_history(&mut self) {
        self.recent_nudges.clear();
    }
}

/// Generate follow-up questions based on topic keywords.
fn generate_follow_up_questions(message: &str, results: &[SearchResult]) -> Vec<String> {
    let mut questions = Vec::new();

    // Topic-based question templates
    if message.contains("chính sách") || message.contains("policy") {
        questions.push("Chính sách này áp dụng từ khi nào?".into());
        questions.push("Có ngoại lệ nào cho chính sách này không?".into());
    }

    if message.contains("khách hàng") || message.contains("client") || message.contains("customer")
    {
        questions.push("Lịch sử giao dịch gần đây với khách hàng này?".into());
        questions.push("Ai là người liên hệ chính?".into());
    }

    if message.contains("sản phẩm") || message.contains("product") {
        questions.push("Giá và tồn kho hiện tại của sản phẩm này?".into());
        questions.push("Có sản phẩm thay thế hoặc bổ sung nào không?".into());
    }

    if message.contains("meeting") || message.contains("họp") {
        questions.push("Ghi chú từ cuộc họp trước là gì?".into());
        questions.push("Cần chuẩn bị tài liệu gì cho cuộc họp?".into());
    }

    if message.contains("báo cáo") || message.contains("report") {
        questions.push("So sánh với kỳ trước thì ra sao?".into());
        questions.push("Có KPI nào cần chú ý đặc biệt?".into());
    }

    // If we have search results, suggest exploring related documents
    if questions.is_empty() && !results.is_empty() {
        let doc_names: Vec<_> = results
            .iter()
            .map(|r| r.doc_name.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .take(2)
            .collect();

        for name in doc_names {
            questions.push(format!("Còn thông tin gì khác trong \"{}\"?", name));
        }
    }

    questions
}

/// Check if text contains number patterns (prices, quantities, etc.).
fn contains_numbers(text: &str) -> bool {
    text.chars().any(|c| c.is_ascii_digit())
        && (text.contains("triệu")
            || text.contains("tỷ")
            || text.contains("vnđ")
            || text.contains("usd")
            || text.contains("đồng")
            || text.contains('%')
            || text.contains("vnd"))
}

/// Check if text contains date-like patterns.
fn contains_date_pattern(text: &str) -> bool {
    // Simple Vietnamese date patterns
    text.contains("tháng") || text.contains("ngày")
        || text.contains("deadline") || text.contains("hạn")
        || text.contains("trước ngày") || text.contains("từ ngày")
        // ISO date pattern (rough check)
        || text.contains("2024") || text.contains("2025") || text.contains("2026")
}

/// Extract a clean snippet from content.
fn extract_snippet(content: &str, max_len: usize) -> String {
    let clean: String = content
        .chars()
        .filter(|c| !c.is_control())
        .take(max_len)
        .collect();
    if content.len() > max_len {
        format!("{}...", clean.trim())
    } else {
        clean.trim().to_string()
    }
}

/// Simple string hash for deduplication IDs.
fn hash_string(s: &str) -> u32 {
    let mut hash: u32 = 5381;
    for byte in s.bytes() {
        hash = hash.wrapping_mul(33).wrapping_add(byte as u32);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_result(name: &str, content: &str, score: f64) -> SearchResult {
        SearchResult {
            doc_name: name.to_string(),
            chunk_idx: 0,
            content: content.to_string(),
            score,
            mimetype: None,
            owner: None,
        }
    }

    #[test]
    fn test_generate_nudges_empty() {
        let mut engine = NudgeEngine::new(NudgeConfig::default());
        let nudges = engine.generate_nudges("hello", &[], None);
        assert!(nudges.is_empty() || nudges.iter().all(|n| n.category == NudgeCategory::Question));
    }

    #[test]
    fn test_document_nudge_multi_chunk() {
        let mut engine = NudgeEngine::new(NudgeConfig::default());
        let results = vec![
            make_result("policy.md", "Chính sách nghỉ phép 12 ngày", 0.8),
            make_result("policy.md", "Nghỉ phép năm cộng dồn tối đa 5 ngày", 0.6),
            make_result("rules.md", "Quy tắc nội bộ", 0.3),
        ];

        let nudges = engine.generate_nudges("chính sách nghỉ phép", &results, None);
        // Should have a document nudge for policy.md (2 matching chunks)
        let doc_nudge = nudges
            .iter()
            .find(|n| n.category == NudgeCategory::Document);
        assert!(doc_nudge.is_some(), "Should suggest policy.md document");
    }

    #[test]
    fn test_question_nudges() {
        let mut engine = NudgeEngine::new(NudgeConfig::default());
        let nudges = engine.generate_nudges("meeting với khách hàng ABC", &[], None);
        let q_nudges: Vec<_> = nudges
            .iter()
            .filter(|n| n.category == NudgeCategory::Question)
            .collect();
        assert!(!q_nudges.is_empty(), "Should suggest follow-up questions");
    }

    #[test]
    fn test_no_duplicate_nudges() {
        let mut engine = NudgeEngine::new(NudgeConfig::default());
        let results = vec![
            make_result("doc.md", "Content A", 0.8),
            make_result("doc.md", "Content B", 0.7),
        ];

        let nudges1 = engine.generate_nudges("test", &results, None);
        let nudges2 = engine.generate_nudges("test", &results, None);
        // Second call should not repeat nudges
        for n in &nudges2 {
            assert!(
                !nudges1.iter().any(|n1| n1.id == n.id),
                "Should not repeat nudge: {}",
                n.id
            );
        }
    }

    #[test]
    fn test_insight_nudge_financial() {
        let mut engine = NudgeEngine::new(NudgeConfig::default());
        let results = vec![make_result(
            "report.md",
            "Doanh thu tháng 3: 500 triệu VNĐ",
            0.9,
        )];
        let nudges = engine.generate_nudges("doanh thu tháng này", &results, None);
        let insight = nudges.iter().find(|n| n.category == NudgeCategory::Insight);
        assert!(insight.is_some(), "Should detect financial data");
    }

    #[test]
    fn test_contains_numbers() {
        assert!(contains_numbers("500 triệu"));
        assert!(contains_numbers("20%"));
        assert!(!contains_numbers("hello world"));
    }

    #[test]
    fn test_contains_date_pattern() {
        assert!(contains_date_pattern("deadline ngày 15 tháng 3"));
        assert!(contains_date_pattern("trước ngày 2025-01-01"));
        assert!(!contains_date_pattern("hello world"));
    }
}
