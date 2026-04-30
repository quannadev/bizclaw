// Khazix Skill Trait and Implementation
// Base trait for all skills

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillMetadata {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub triggers: Vec<String>,
    pub category: String,
    pub tags: Vec<String>,
    pub author: Option<String>,
    pub examples: Vec<String>,
    pub requires_tools: Vec<String>,
    pub auto_generated: bool,
}

#[derive(Debug, Clone)]
pub struct SkillContext {
    pub user_input: String,
    pub conversation_history: Vec<ConversationEntry>,
    pub project_root: Option<String>,
    pub tools: Vec<String>,
    pub metadata: std::collections::HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationEntry {
    pub role: String,
    pub content: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct SkillResult {
    pub success: bool,
    pub output: String,
    pub changes_made: Vec<Change>,
    pub warnings: Vec<String>,
    pub execution_time_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Change {
    pub change_type: ChangeType,
    pub path: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ChangeType {
    Created,
    Modified,
    Deleted,
    Updated,
}

impl std::fmt::Display for ChangeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChangeType::Created => write!(f, "Created"),
            ChangeType::Modified => write!(f, "Modified"),
            ChangeType::Deleted => write!(f, "Deleted"),
            ChangeType::Updated => write!(f, "Updated"),
        }
    }
}

pub trait KhazixSkill: Send + Sync {
    fn metadata(&self) -> &SkillMetadata;
    
    fn matches(&self, input: &str) -> bool {
        let input_lower = input.to_lowercase();
        self.metadata().triggers.iter().any(|trigger| {
            input_lower.contains(&trigger.to_lowercase())
        })
    }
    
    fn execute(&self, context: &SkillContext) -> SkillResult;
    
    fn system_prompt(&self) -> String;
    
    fn required_tools(&self) -> Vec<String> {
        self.metadata().requires_tools.clone()
    }
}

#[macro_export]
macro_rules! define_skill {
    (
        name: $name:expr,
        description: $desc:expr,
        triggers: [$($trigger:expr),*],
        prompt: $prompt:expr
    ) => {
        use serde::{Deserialize, Serialize};
        use chrono::{DateTime, Utc};
        
        #[derive(Debug, Clone, Serialize, Deserialize)]
        pub struct SimpleSkill {
            metadata: SkillMetadata,
            prompt: String,
        }
        
        impl SimpleSkill {
            pub fn new() -> Self {
                Self {
                    metadata: SkillMetadata {
                        id: $name.to_string(),
                        name: $name.to_string(),
                        description: $desc.to_string(),
                        version: "1.0.0".to_string(),
                        triggers: vec![$($trigger.to_string()),*],
                        category: "general".to_string(),
                        tags: vec![],
                        author: Some("BizClaw".to_string()),
                        examples: vec![],
                        requires_tools: vec![],
                        auto_generated: false,
                    },
                    prompt: $prompt.to_string(),
                }
            }
        }
        
        impl KhazixSkill for SimpleSkill {
            fn metadata(&self) -> &SkillMetadata {
                &self.metadata
            }
            
            fn system_prompt(&self) -> String {
                self.prompt.clone()
            }
            
            fn execute(&self, context: &SkillContext) -> SkillResult {
                let start = std::time::Instant::now();
                SkillResult {
                    success: true,
                    output: format!("Executed {} skill with input: {}", self.metadata.name, context.user_input),
                    changes_made: vec![],
                    warnings: vec![],
                    execution_time_ms: start.elapsed().as_millis() as u64,
                }
            }
        }
    };
}
