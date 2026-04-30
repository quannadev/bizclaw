//! Trace Analyzer - Phân tích execution logs để tìm patterns
//! 
//! Giống AGNT SkillForge trace analyzer.
//! Đọc execution traces và trích xuất thông tin về tool usage,
//! success patterns, error patterns, và context information.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap as StdHashMap, HashSet};
use chrono::{DateTime, Utc};

/// Một entry trong execution trace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceEntry {
    pub timestamp: DateTime<Utc>,
    pub level: TraceLevel,
    pub category: String,
    pub message: String,
    pub metadata: StdHashMap<String, serde_json::Value>,
}

/// Trace levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Copy)]
pub enum TraceLevel {
    Debug,
    Info,
    Warn,
    Error,
    Skill,
    Tool,
    Goal,
    Llm,
}

impl Ord for TraceLevel {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let self_idx = match self {
            TraceLevel::Debug => 0,
            TraceLevel::Info => 1,
            TraceLevel::Warn => 2,
            TraceLevel::Error => 3,
            TraceLevel::Skill => 4,
            TraceLevel::Tool => 5,
            TraceLevel::Goal => 6,
            TraceLevel::Llm => 7,
        };
        let other_idx = match other {
            TraceLevel::Debug => 0,
            TraceLevel::Info => 1,
            TraceLevel::Warn => 2,
            TraceLevel::Error => 3,
            TraceLevel::Skill => 4,
            TraceLevel::Tool => 5,
            TraceLevel::Goal => 6,
            TraceLevel::Llm => 7,
        };
        self_idx.cmp(&other_idx)
    }
}

impl PartialOrd for TraceLevel {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl TraceLevel {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "debug" => TraceLevel::Debug,
            "info" => TraceLevel::Info,
            "warn" | "warning" => TraceLevel::Warn,
            "error" | "err" => TraceLevel::Error,
            "skill" => TraceLevel::Skill,
            "tool" => TraceLevel::Tool,
            "goal" => TraceLevel::Goal,
            "llm" => TraceLevel::Llm,
            _ => TraceLevel::Info,
        }
    }
}

/// Phân tích một execution trace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceAnalysis {
    pub total_entries: usize,
    pub entries_by_level: BTreeMap<TraceLevel, usize>,
    pub tool_usage: StdHashMap<String, ToolUsageStats>,
    pub skill_usage: StdHashMap<String, SkillUsageStats>,
    pub error_patterns: Vec<ErrorPattern>,
    pub success_patterns: Vec<SuccessPattern>,
    pub context_hints: Vec<ContextHint>,
    pub duration_ms: u64,
    pub time_range: Option<(DateTime<Utc>, DateTime<Utc>)>,
}

/// Tool usage statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ToolUsageStats {
    pub count: u64,
    pub success_count: u64,
    pub failure_count: u64,
    pub avg_duration_ms: f64,
    pub total_duration_ms: u64,
    pub contexts: HashSet<String>,
}

/// Skill usage statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SkillUsageStats {
    pub count: u64,
    pub avg_duration_ms: f64,
    pub contexts: HashSet<String>,
    pub outcomes: Vec<String>,
}

/// Error pattern discovered
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorPattern {
    pub pattern: String,
    pub count: u64,
    pub tool: Option<String>,
    pub context: Option<String>,
    pub suggestion: String,
}

/// Success pattern discovered
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuccessPattern {
    pub pattern: String,
    pub count: u64,
    pub tool_chain: Vec<String>,
    pub context: Option<String>,
    pub success_rate: f32,
}

/// Context hint cho skill generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextHint {
    pub hint_type: ContextHintType,
    pub content: String,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContextHintType {
    RequiredTool,
    RecommendedTool,
    Workflow,
    BusinessDomain,
    ErrorRecovery,
}

/// Trace Analyzer chính
pub struct TraceAnalyzer {
    entries: Vec<TraceEntry>,
}

impl TraceAnalyzer {
    pub fn new() -> Self {
        Self { entries: Vec::new() }
    }

    /// Parse trace từ log text (multi-line format)
    pub fn parse_from_logs(&mut self, log_text: &str) {
        self.entries.clear();
        
        for line in log_text.lines() {
            if let Some(entry) = self.parse_line(line) {
                self.entries.push(entry);
            }
        }
    }

    /// Parse trace từ JSON lines
    pub fn parse_from_jsonl(&mut self, jsonl: &str) {
        self.entries.clear();
        
        for line in jsonl.lines() {
            if let Ok(entry) = serde_json::from_str::<TraceEntry>(line) {
                self.entries.push(entry);
            }
        }
    }

    /// Add a single entry
    pub fn add_entry(&mut self, entry: TraceEntry) {
        self.entries.push(entry);
    }

    /// Parse một log line
    fn parse_line(&self, line: &str) -> Option<TraceEntry> {
        let line = line.trim();
        if line.is_empty() {
            return None;
        }

        // Parse format: [TIMESTAMP] LEVEL CATEGORY: message
        // Ví dụ: [2026-04-15T17:05:00.000Z] INFO tool: browser_navigate
        let timestamp_regex = regex::Regex::new(r"^\[([^\]]+)\]").ok()?;
        let level_regex = regex::Regex::new(r"\]\s+(\w+)").ok()?;
        
        let timestamp_str = timestamp_regex.captures(line)?.get(1)?.as_str();
        let level_str = level_regex.captures(line)?.get(1)?.as_str();
        
        let rest = line.splitn(2, |c| c == ']' || c == ':').last()?.trim();
        let parts: Vec<&str> = rest.splitn(2, ':').collect();
        
        let category = parts.first()?.trim().to_string();
        let message = parts.get(1).map(|s| s.trim()).unwrap_or("").to_string();
        
        let timestamp = DateTime::parse_from_rfc3339(timestamp_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());
        
        Some(TraceEntry {
            timestamp,
            level: TraceLevel::from_str(level_str),
            category,
            message,
            metadata: StdHashMap::new(),
        })
    }

    /// Phân tích tất cả entries
    pub fn analyze(&self) -> TraceAnalysis {
        let mut analysis = TraceAnalysis {
            total_entries: self.entries.len(),
            entries_by_level: BTreeMap::new(),
            tool_usage: StdHashMap::new(),
            skill_usage: StdHashMap::new(),
            error_patterns: Vec::new(),
            success_patterns: Vec::new(),
            context_hints: Vec::new(),
            duration_ms: 0,
            time_range: None,
        };

        // Count by level
        for entry in &self.entries {
            *analysis.entries_by_level.entry(entry.level).or_insert(0) += 1;
        }

        // Extract tool and skill usage
        let mut tool_chain: Vec<String> = Vec::new();
        for entry in &self.entries {
            match entry.level {
                TraceLevel::Tool => {
                    tool_chain.push(entry.category.clone());
                    self.update_tool_stats(&mut analysis, &entry);
                }
                TraceLevel::Skill => {
                    self.update_skill_stats(&mut analysis, &entry);
                }
                TraceLevel::Error => {
                    self.extract_error_pattern(&mut analysis, &entry);
                }
                TraceLevel::Goal => {
                    tool_chain.clear();
                }
                _ => {}
            }
        }

        // Extract success patterns from tool chains
        self.extract_success_patterns(&mut analysis, &tool_chain);

        // Generate context hints
        self.generate_context_hints(&mut analysis);

        // Calculate duration
        if let Some(first) = self.entries.first() {
            if let Some(last) = self.entries.last() {
                analysis.duration_ms = (last.timestamp - first.timestamp).num_milliseconds() as u64;
                analysis.time_range = Some((first.timestamp, last.timestamp));
            }
        }

        analysis
    }

    fn update_tool_stats(&self, analysis: &mut TraceAnalysis, entry: &TraceEntry) {
        let stats = analysis.tool_usage.entry(entry.category.clone()).or_default();
        stats.count += 1;
        
        if entry.level == TraceLevel::Error {
            stats.failure_count += 1;
        } else {
            stats.success_count += 1;
        }
        
        stats.contexts.insert(entry.message.clone());
    }

    fn update_skill_stats(&self, analysis: &mut TraceAnalysis, entry: &TraceEntry) {
        let stats = analysis.skill_usage.entry(entry.category.clone()).or_default();
        stats.count += 1;
        stats.contexts.insert(entry.message.clone());
        stats.outcomes.push(entry.message.clone());
    }

    fn extract_error_pattern(&self, analysis: &mut TraceAnalysis, entry: &TraceEntry) {
        let pattern = entry.message.clone();
        if pattern.len() < 10 {
            return;
        }

        // Tìm pattern tương tự
        if let Some(existing) = analysis.error_patterns.iter_mut().find(|p| {
            self.similarity(&p.pattern, &pattern) > 0.7
        }) {
            existing.count += 1;
        } else {
            analysis.error_patterns.push(ErrorPattern {
                count: 1,
                pattern: pattern.clone(),
                tool: if entry.category == "tool" {
                    Some(entry.message.clone())
                } else {
                    None
                },
                context: None,
                suggestion: self.generate_error_suggestion(&pattern),
            });
        }
    }

    fn extract_success_patterns(&self, analysis: &mut TraceAnalysis, tool_chain: &[String]) {
        if tool_chain.len() < 2 {
            return;
        }

        let chain_key = tool_chain.join(" → ");
        
        if let Some(existing) = analysis.success_patterns.iter_mut().find(|p| {
            p.tool_chain == tool_chain
        }) {
            existing.count += 1;
        } else {
            analysis.success_patterns.push(SuccessPattern {
                pattern: chain_key.clone(),
                count: 1,
                tool_chain: tool_chain.to_vec(),
                context: None,
                success_rate: 1.0,
            });
        }
    }

    fn generate_context_hints(&self, analysis: &mut TraceAnalysis) {
        // Required tools based on frequency
        for (tool, stats) in &analysis.tool_usage {
            if stats.count >= 3 {
                analysis.context_hints.push(ContextHint {
                    hint_type: ContextHintType::RequiredTool,
                    content: format!("Tool '{}' được sử dụng {} lần - nên include trong skill", tool, stats.count),
                    confidence: (stats.count as f32 / 10.0).min(1.0),
                });
            }
        }

        // Error recovery hints
        if !analysis.error_patterns.is_empty() {
            analysis.context_hints.push(ContextHint {
                hint_type: ContextHintType::ErrorRecovery,
                content: format!("Có {} error patterns cần handle - nên thêm error handling vào skill", analysis.error_patterns.len()),
                confidence: 0.8,
            });
        }

        // Workflow hints từ success patterns
        for pattern in &analysis.success_patterns {
            if pattern.count >= 2 && pattern.tool_chain.len() >= 2 {
                analysis.context_hints.push(ContextHint {
                    hint_type: ContextHintType::Workflow,
                    content: format!("Workflow pattern: {} (x{})", pattern.pattern, pattern.count),
                    confidence: 0.7,
                });
            }
        }
    }

    fn similarity(&self, s1: &str, s2: &str) -> f32 {
        let words1: HashSet<&str> = s1.split_whitespace().collect();
        let words2: HashSet<&str> = s2.split_whitespace().collect();
        
        let intersection = words1.intersection(&words2).count() as f32;
        let union = words1.union(&words2).count() as f32;
        
        if union == 0.0 {
            return 0.0;
        }
        
        intersection / union
    }

    fn generate_error_suggestion(&self, error: &str) -> String {
        let error_lower = error.to_lowercase();
        
        if error_lower.contains("timeout") {
            "Consider adding retry logic with exponential backoff".to_string()
        } else if error_lower.contains("permission") || error_lower.contains("access") {
            "Check permissions or add authentication step".to_string()
        } else if error_lower.contains("not found") || error_lower.contains("404") {
            "Add validation for resource existence before access".to_string()
        } else if error_lower.contains("parse") || error_lower.contains("format") {
            "Add input validation and error handling for parsing".to_string()
        } else {
            "Add try-catch wrapper and user-friendly error message".to_string()
        }
    }
}

impl Default for TraceAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_log_line() {
        let analyzer = TraceAnalyzer::new();
        let line = "[2026-04-15T17:05:00.000Z] INFO tool: browser_navigate url=https://example.com";
        
        let entry = analyzer.parse_line(line).unwrap();
        assert_eq!(entry.level, TraceLevel::Info);
        assert_eq!(entry.category, "tool");
        assert!(entry.message.contains("browser_navigate"));
    }

    #[test]
    fn test_error_pattern_extraction() {
        let mut analyzer = TraceAnalyzer::new();
        
        analyzer.add_entry(TraceEntry {
            timestamp: Utc::now(),
            level: TraceLevel::Error,
            category: "tool".to_string(),
            message: "Connection timeout after 30s".to_string(),
            metadata: HashMap::new(),
        });
        
        analyzer.add_entry(TraceEntry {
            timestamp: Utc::now(),
            level: TraceLevel::Error,
            category: "tool".to_string(),
            message: "Connection timeout after 30s".to_string(),
            metadata: HashMap::new(),
        });
        
        let analysis = analyzer.analyze();
        assert_eq!(analysis.error_patterns.len(), 1);
        assert_eq!(analysis.error_patterns[0].count, 2);
    }
}
