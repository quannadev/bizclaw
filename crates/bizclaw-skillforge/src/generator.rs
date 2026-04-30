//! Skill Generator - Tạo SKILL.md từ extracted patterns
//! 
//! Giống AGNT SkillForge skill generator.
//! Convert ExtractedPattern thành SKILL.md format với YAML frontmatter.

use crate::extractor::{ExtractedPattern, PatternType};
use serde::{Deserialize, Serialize};

/// Generated skill manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedSkill {
    pub metadata: SkillFrontmatter,
    pub content: String,
    pub raw_skill_md: String,
}

/// YAML frontmatter cho SKILL.md
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillFrontmatter {
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub version: String,
    pub author: String,
    pub category: String,
    pub tags: Vec<String>,
    pub tools: Vec<String>,
    pub patterns: Vec<String>,
    pub confidence: f32,
    pub auto_generated: bool,
    pub source_pattern_id: String,
    pub examples: Vec<String>,
}

/// Generator configuration
#[derive(Debug, Clone)]
pub struct GeneratorConfig {
    pub author_name: String,
    pub include_examples: bool,
    pub include_steps: bool,
    pub include_error_handling: bool,
    pub skill_directory: Option<String>,
}

impl Default for GeneratorConfig {
    fn default() -> Self {
        Self {
            author_name: "BizClaw SkillForge".to_string(),
            include_examples: true,
            include_steps: true,
            include_error_handling: true,
            skill_directory: None,
        }
    }
}

/// Skill Generator
pub struct SkillGenerator {
    config: GeneratorConfig,
}

impl SkillGenerator {
    pub fn new(config: GeneratorConfig) -> Self {
        Self { config }
    }

    pub fn with_defaults() -> Self {
        Self::new(GeneratorConfig::default())
    }

    /// Generate SKILL.md từ một pattern
    pub fn generate(&self, pattern: &ExtractedPattern) -> GeneratedSkill {
        let metadata = self.generate_frontmatter(pattern);
        let content = self.generate_content(pattern);
        let raw_md = format!("---\n{}\n---\n\n{}", 
            serde_yaml::to_string(&metadata).unwrap_or_default(),
            content
        );
        
        GeneratedSkill {
            metadata,
            content,
            raw_skill_md: raw_md,
        }
    }

    /// Generate nhiều skills từ patterns
    pub fn generate_batch(&self, patterns: &[ExtractedPattern]) -> Vec<GeneratedSkill> {
        patterns.iter().map(|p| self.generate(p)).collect()
    }

    fn generate_frontmatter(&self, pattern: &ExtractedPattern) -> SkillFrontmatter {
        SkillFrontmatter {
            name: pattern.pattern_id.clone(),
            display_name: pattern.name.clone(),
            description: pattern.description.clone(),
            version: "1.0.0".to_string(),
            author: self.config.author_name.clone(),
            category: pattern.category.clone(),
            tags: self.generate_tags(pattern),
            tools: pattern.tools_required.clone(),
            patterns: vec![pattern.pattern_type.to_string()],
            confidence: pattern.confidence,
            auto_generated: true,
            source_pattern_id: pattern.pattern_id.clone(),
            examples: pattern.examples.clone(),
        }
    }

    fn generate_content(&self, pattern: &ExtractedPattern) -> String {
        let mut md = String::new();
        
        // Header
        md.push_str(&format!("# {}\n\n", pattern.name));
        md.push_str(&format!("> Auto-generated skill from BizClaw SkillForge\n"));
        md.push_str(&format!("> Confidence: {:.0}% | Type: {:?}\n\n", pattern.confidence * 100.0, pattern.pattern_type));
        
        // Description
        md.push_str("## Description\n\n");
        md.push_str(&format!("{}\n\n", pattern.description));
        
        // Triggers
        md.push_str("## Triggers\n\n");
        md.push_str("- When ");
        md.push_str(&self.generate_triggers(pattern));
        md.push_str("\n\n");
        
        // Capabilities
        md.push_str("## Capabilities\n\n");
        for (i, step) in pattern.steps.iter().enumerate() {
            md.push_str(&format!("{}. {}\n", i + 1, step.description));
        }
        md.push_str("\n");
        
        // Steps (nếu được include)
        if self.config.include_steps && !pattern.steps.is_empty() {
            md.push_str("## Execution Steps\n\n");
            for step in &pattern.steps {
                md.push_str(&format!("### Step {}: {}\n\n", step.step_number, step.action));
                if let Some(ref tool) = step.tool {
                    md.push_str(&format!("**Tool:** `{}`\n\n", tool));
                }
                md.push_str(&format!("{}\n\n", step.description));
                if let Some(ref on_error) = step.on_error {
                    md.push_str(&format!("**On Error:** {}\n\n", on_error));
                }
            }
        }
        
        // Required Tools
        if !pattern.tools_required.is_empty() {
            md.push_str("## Required Tools\n\n");
            for tool in &pattern.tools_required {
                md.push_str(&format!("- `{}`\n", tool));
            }
            md.push_str("\n");
        }
        
        // Error Handling (nếu được include)
        if self.config.include_error_handling && !pattern.error_handling.is_empty() {
            md.push_str("## Error Handling\n\n");
            for handling in &pattern.error_handling {
                md.push_str(&format!("- {}\n", handling));
            }
            md.push_str("\n");
        }
        
        // Examples (nếu được include)
        if self.config.include_examples && !pattern.examples.is_empty() {
            md.push_str("## Usage Examples\n\n");
            for (i, example) in pattern.examples.iter().enumerate() {
                md.push_str(&format!("### Example {}\n\n", i + 1));
                md.push_str(&format!("```\n{}\n```\n\n", example));
            }
        }
        
        // Metadata
        md.push_str("---\n\n");
        md.push_str("## Metadata\n\n");
        md.push_str(&format!("| Property | Value |\n"));
        md.push_str(&format!("|-----------|-------|\n"));
        md.push_str(&format!("| Pattern ID | `{}` |\n", pattern.pattern_id));
        md.push_str(&format!("| Pattern Type | {:?} |\n", pattern.pattern_type));
        md.push_str(&format!("| Confidence | {:.0}% |\n", pattern.confidence * 100.0));
        md.push_str(&format!("| Auto-Generated | Yes |\n"));
        
        if let Some(ref domain) = pattern.business_domain {
            md.push_str(&format!("| Business Domain | {} |\n", domain));
        }
        
        md
    }

    fn generate_tags(&self, pattern: &ExtractedPattern) -> Vec<String> {
        let mut tags = vec![
            "auto-generated".to_string(),
            "skillforge".to_string(),
            pattern.category.clone(),
        ];
        
        match pattern.pattern_type {
            PatternType::Workflow => tags.push("workflow".to_string()),
            PatternType::ToolChain => tags.push("toolchain".to_string()),
            PatternType::ErrorRecovery => tags.push("error-handling".to_string()),
            PatternType::DataTransform => tags.push("data".to_string()),
            PatternType::BusinessProcess => tags.push("business".to_string()),
        }
        
        // Add tool-based tags
        for tool in &pattern.tools_required {
            if tool.len() < 20 {
                tags.push(tool.clone());
            }
        }
        
        tags
    }

    fn generate_triggers(&self, pattern: &ExtractedPattern) -> String {
        match pattern.pattern_type {
            PatternType::Workflow => {
                format!("the user wants to execute: {}", pattern.tools_required.join(" → "))
            }
            PatternType::ToolChain => {
                format!("a task requires: {}", pattern.tools_required.join(", "))
            }
            PatternType::ErrorRecovery => {
                format!("an error related to {} occurs", pattern.pattern_id.replace("error_recovery_", ""))
            }
            PatternType::DataTransform => {
                format!("data needs to be transformed using {}", pattern.tools_required.join(", "))
            }
            PatternType::BusinessProcess => {
                "a business process needs to be automated".to_string()
            }
        }
    }

    /// Save generated skill to file
    pub fn save_to_file(&self, skill: &GeneratedSkill, path: &std::path::Path) -> Result<(), String> {
        std::fs::write(path, &skill.raw_skill_md)
            .map_err(|e| format!("Failed to write skill file: {}", e))
    }

    /// Get skill file path
    pub fn get_skill_path(&self, pattern: &ExtractedPattern) -> std::path::PathBuf {
        let dir = self.config.skill_directory
            .as_ref()
            .map(|d| std::path::PathBuf::from(d))
            .unwrap_or_else(|| {
                dirs::home_dir()
                    .unwrap_or_else(|| std::path::PathBuf::from("."))
                    .join(".bizclaw")
                    .join("skills")
            });
        
        dir.join(format!("{}.md", pattern.pattern_id))
    }
}

impl Default for SkillGenerator {
    fn default() -> Self {
        Self::with_defaults()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extractor::{ExtractedPattern, PatternType, PatternStep};

    #[test]
    fn test_generate_skill() {
        let generator = SkillGenerator::with_defaults();
        
        let pattern = ExtractedPattern {
            pattern_id: "test_workflow".to_string(),
            pattern_type: PatternType::Workflow,
            name: "Test Workflow".to_string(),
            description: "A test workflow".to_string(),
            tools_required: vec!["tool_a".to_string(), "tool_b".to_string()],
            steps: vec![
                PatternStep {
                    step_number: 1,
                    action: "Step 1".to_string(),
                    tool: Some("tool_a".to_string()),
                    description: "Do first thing".to_string(),
                    on_error: None,
                },
            ],
            error_handling: vec!["Handle error".to_string()],
            confidence: 0.8,
            examples: vec!["Example usage".to_string()],
            category: "test".to_string(),
            business_domain: None,
        };
        
        let skill = generator.generate(&pattern);
        
        assert!(skill.raw_skill_md.contains("---"));
        assert!(skill.raw_skill_md.contains("name: test_workflow"));
        assert!(skill.raw_skill_md.contains("# Test Workflow"));
    }
}
