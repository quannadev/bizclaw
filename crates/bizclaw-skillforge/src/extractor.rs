//! Pattern Extractor - Trích xuất patterns từ trace analysis
//! 
//! Giống AGNT pattern extractor trong SkillForge.
//! Phân tích TraceAnalysis để trích xuất các patterns có thể
//! convert thành reusable skills.

use crate::analyzer::{TraceAnalysis, ErrorPattern, SuccessPattern, ContextHint};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Extracted pattern có thể dùng để tạo skill
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedPattern {
    pub pattern_id: String,
    pub pattern_type: PatternType,
    pub name: String,
    pub description: String,
    pub tools_required: Vec<String>,
    pub steps: Vec<PatternStep>,
    pub error_handling: Vec<String>,
    pub confidence: f32,
    pub examples: Vec<String>,
    pub category: String,
    pub business_domain: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PatternType {
    Workflow,
    ToolChain,
    ErrorRecovery,
    DataTransform,
    BusinessProcess,
}

impl std::fmt::Display for PatternType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PatternType::Workflow => write!(f, "Workflow"),
            PatternType::ToolChain => write!(f, "ToolChain"),
            PatternType::ErrorRecovery => write!(f, "ErrorRecovery"),
            PatternType::DataTransform => write!(f, "DataTransform"),
            PatternType::BusinessProcess => write!(f, "BusinessProcess"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternStep {
    pub step_number: u32,
    pub action: String,
    pub tool: Option<String>,
    pub description: String,
    pub on_error: Option<String>,
}

/// Extractor configuration
#[derive(Debug, Clone)]
pub struct ExtractorConfig {
    pub min_pattern_confidence: f32,
    pub min_tool_usage: u64,
    pub include_error_recovery: bool,
    pub include_business_context: bool,
}

impl Default for ExtractorConfig {
    fn default() -> Self {
        Self {
            min_pattern_confidence: 0.5,
            min_tool_usage: 2,
            include_error_recovery: true,
            include_business_context: true,
        }
    }
}

/// Pattern Extractor
pub struct PatternExtractor {
    config: ExtractorConfig,
}

impl PatternExtractor {
    pub fn new(config: ExtractorConfig) -> Self {
        Self { config }
    }

    pub fn with_defaults() -> Self {
        Self::new(ExtractorConfig::default())
    }

    /// Extract patterns từ trace analysis
    pub fn extract(&self, analysis: &TraceAnalysis) -> Vec<ExtractedPattern> {
        let mut patterns = Vec::new();

        // Extract workflow patterns từ success patterns
        patterns.extend(self.extract_workflow_patterns(analysis));

        // Extract tool chain patterns
        patterns.extend(self.extract_tool_chain_patterns(analysis));

        // Extract error recovery patterns
        if self.config.include_error_recovery {
            patterns.extend(self.extract_error_recovery_patterns(analysis));
        }

        // Extract data transformation patterns
        patterns.extend(self.extract_data_transform_patterns(analysis));

        // Sort by confidence
        patterns.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());
        
        patterns
    }

    fn extract_workflow_patterns(&self, analysis: &TraceAnalysis) -> Vec<ExtractedPattern> {
        analysis.success_patterns
            .iter()
            .filter(|p| p.count >= 2 && p.tool_chain.len() >= 2)
            .map(|p| {
                let pattern_id = format!("workflow_{}", p.tool_chain.join("_"));
                
                ExtractedPattern {
                    pattern_id: pattern_id.clone(),
                    pattern_type: PatternType::Workflow,
                    name: self.generate_workflow_name(&p.tool_chain),
                    description: format!("A {} step workflow using: {}", p.tool_chain.len(), p.tool_chain.join(" → ")),
                    tools_required: p.tool_chain.clone(),
                    steps: self.generate_steps_from_chain(&p.tool_chain),
                    error_handling: self.suggest_error_handling(&p.tool_chain),
                    confidence: (p.count as f32 / 10.0).min(1.0),
                    examples: vec![format!("Execute: {}", p.tool_chain.join(" → "))],
                    category: "workflow".to_string(),
                    business_domain: None,
                }
            })
            .collect()
    }

    fn extract_tool_chain_patterns(&self, analysis: &TraceAnalysis) -> Vec<ExtractedPattern> {
        let mut chains: HashMap<String, Vec<String>> = HashMap::new();
        
        for pattern in &analysis.success_patterns {
            if pattern.tool_chain.len() >= 2 {
                let key = pattern.tool_chain.join("→");
                chains.entry(key).or_insert_with(Vec::new).push(pattern.pattern.clone());
            }
        }

        chains
            .into_iter()
            .filter(|(_, examples)| examples.len() >= 2)
            .map(|(chain, examples)| {
                let tools: Vec<String> = chain.split("→").map(|s| s.trim().to_string()).collect();
                
                ExtractedPattern {
                    pattern_id: format!("chain_{}", chain.replace("→", "_")),
                    pattern_type: PatternType::ToolChain,
                    name: format!("{} Tool Chain", tools.first().unwrap_or(&"Unknown".to_string())),
                    description: format!("Common tool chain: {}", chain),
                    tools_required: tools.clone(),
                    steps: self.generate_steps_from_chain(&tools),
                    error_handling: vec![],
                    confidence: 0.6,
                    examples,
                    category: "toolchain".to_string(),
                    business_domain: None,
                }
            })
            .collect()
    }

    fn extract_error_recovery_patterns(&self, analysis: &TraceAnalysis) -> Vec<ExtractedPattern> {
        analysis.error_patterns
            .iter()
            .filter(|e| e.count >= 2)
            .map(|e| {
                let error_type = self.categorize_error(&e.pattern);
                
                ExtractedPattern {
                    pattern_id: format!("error_recovery_{}", error_type),
                    pattern_type: PatternType::ErrorRecovery,
                    name: format!("{} Recovery", error_type.replace('_', " ").to_title_case()),
                    description: e.suggestion.clone(),
                    tools_required: e.tool.clone().map(|t| vec![t]).unwrap_or_default(),
                    steps: vec![
                        PatternStep {
                            step_number: 1,
                            action: "Detect".to_string(),
                            tool: None,
                            description: format!("Detect error: {}", e.pattern.chars().take(50).collect::<String>()),
                            on_error: None,
                        },
                        PatternStep {
                            step_number: 2,
                            action: "Recover".to_string(),
                            tool: None,
                            description: e.suggestion.clone(),
                            on_error: Some("Log and escalate".to_string()),
                        },
                    ],
                    error_handling: vec![e.suggestion.clone()],
                    confidence: 0.7,
                    examples: vec![e.pattern.clone()],
                    category: "error_recovery".to_string(),
                    business_domain: None,
                }
            })
            .collect()
    }

    fn extract_data_transform_patterns(&self, analysis: &TraceAnalysis) -> Vec<ExtractedPattern> {
        let mut patterns = Vec::new();
        
        // Look for data-related tool usage
        let data_tools = ["json_parse", "csv_transform", "sql_query", "data_format", "convert"];
        
        for tool in data_tools {
            if let Some(stats) = analysis.tool_usage.get(tool) {
                if stats.count >= 3 {
                    patterns.push(ExtractedPattern {
                        pattern_id: format!("data_transform_{}", tool),
                        pattern_type: PatternType::DataTransform,
                        name: format!("{} Pattern", tool.replace('_', " ").to_title_case()),
                        description: format!("Data transformation using {}", tool),
                        tools_required: vec![tool.to_string()],
                        steps: vec![
                            PatternStep {
                                step_number: 1,
                                action: "Input".to_string(),
                                tool: Some(tool.to_string()),
                                description: "Receive input data".to_string(),
                                on_error: None,
                            },
                            PatternStep {
                                step_number: 2,
                                action: "Transform".to_string(),
                                tool: Some(tool.to_string()),
                                description: "Apply transformation".to_string(),
                                on_error: Some("Validate input format".to_string()),
                            },
                            PatternStep {
                                step_number: 3,
                                action: "Output".to_string(),
                                tool: None,
                                description: "Return transformed data".to_string(),
                                on_error: None,
                            },
                        ],
                        error_handling: vec!["Validate input".to_string(), "Handle null values".to_string()],
                        confidence: (stats.success_count as f32 / stats.count as f32).min(1.0),
                        examples: vec![],
                        category: "data".to_string(),
                        business_domain: None,
                    });
                }
            }
        }
        
        patterns
    }

    fn generate_workflow_name(&self, tool_chain: &[String]) -> String {
        if tool_chain.is_empty() {
            return "Unnamed Workflow".to_string();
        }
        
        let first = tool_chain.first().unwrap();
        let last = tool_chain.last().unwrap();
        
        if tool_chain.len() == 2 {
            format!("{} to {}", first.replace('_', " ").to_title_case(), last.replace('_', " ").to_title_case())
        } else {
            format!("{} Pipeline", first.replace('_', " ").to_title_case())
        }
    }

    fn generate_steps_from_chain(&self, tools: &[String]) -> Vec<PatternStep> {
        tools
            .iter()
            .enumerate()
            .map(|(i, tool)| PatternStep {
                step_number: (i + 1) as u32,
                action: format!("Execute {}", tool),
                tool: Some(tool.clone()),
                description: format!("Run {} tool", tool.replace('_', " ")),
                on_error: Some(format!("Handle {} failure", tool)),
            })
            .collect()
    }

    fn suggest_error_handling(&self, tools: &[String]) -> Vec<String> {
        let mut handling = Vec::new();
        
        for tool in tools {
            handling.push(format!("If {} fails, retry with exponential backoff", tool));
        }
        
        handling.push("Log all failures for analysis".to_string());
        
        handling
    }

    fn categorize_error(&self, error: &str) -> String {
        let error_lower = error.to_lowercase();
        
        if error_lower.contains("timeout") {
            "timeout".to_string()
        } else if error_lower.contains("permission") || error_lower.contains("access") || error_lower.contains("auth") {
            "authentication".to_string()
        } else if error_lower.contains("not found") || error_lower.contains("404") || error_lower.contains("missing") {
            "not_found".to_string()
        } else if error_lower.contains("parse") || error_lower.contains("format") || error_lower.contains("invalid") {
            "validation".to_string()
        } else if error_lower.contains("network") || error_lower.contains("connection") || error_lower.contains("dns") {
            "network".to_string()
        } else {
            "general".to_string()
        }
    }
}

trait ToTitleCase {
    fn to_title_case(&self) -> String;
}

impl ToTitleCase for str {
    fn to_title_case(&self) -> String {
        self.split_whitespace()
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    None => String::new(),
                    Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_categorize_error() {
        let extractor = PatternExtractor::with_defaults();
        
        assert_eq!(extractor.categorize_error("Connection timeout"), "timeout");
        assert_eq!(extractor.categorize_error("Permission denied"), "authentication");
        assert_eq!(extractor.categorize_error("Resource not found"), "not_found");
    }
}
