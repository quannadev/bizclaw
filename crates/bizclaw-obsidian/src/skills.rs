//! # SKILL.md Management

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillReference {
    pub id: String,
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub version: String,
    pub category: String,
    pub tags: Vec<String>,
    pub tools: Vec<String>,
    pub confidence: f32,
    pub auto_generated: bool,
    pub path: String,
    pub created: DateTime<Utc>,
}

pub struct SkillManager {
    skills_dir: std::path::PathBuf,
    skills: HashMap<String, SkillReference>,
}

impl SkillManager {
    pub fn new(vault_path: &std::path::Path) -> Self {
        Self {
            skills_dir: vault_path.join("skills"),
            skills: HashMap::new(),
        }
    }

    pub async fn index_skills(&mut self) -> Result<(), SkillError> {
        if !self.skills_dir.exists() {
            return Ok(());
        }

        for entry in walkdir::WalkDir::new(&self.skills_dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "md"))
        {
            if let Ok(content) = std::fs::read_to_string(entry.path()) {
                if let Some(skill) = self.parse_skill_md(&content) {
                    self.skills.insert(skill.id.clone(), skill);
                }
            }
        }
        Ok(())
    }

    fn parse_skill_md(&self, content: &str) -> Option<SkillReference> {
        if !content.starts_with("---") {
            return None;
        }

        let rest = content.strip_prefix("---")?;
        let end = rest.find("---")?;
        let yaml_str = &rest[..end];

        let yaml: serde_yaml::Value = serde_yaml::from_str(yaml_str).ok()?;
        
        let id = yaml.get("name")?.as_str()?.to_string();
        let display_name = yaml.get("display_name")
            .and_then(|v| v.as_str())
            .unwrap_or(&id)
            .to_string();
        let description = yaml.get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let version = yaml.get("version")
            .and_then(|v| v.as_str())
            .unwrap_or("1.0.0")
            .to_string();
        let category = yaml.get("category")
            .and_then(|v| v.as_str())
            .unwrap_or("general")
            .to_string();
        let confidence = yaml.get("confidence")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.5) as f32;
        let auto_generated = yaml.get("auto_generated")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        Some(SkillReference {
            id,
            name: display_name.clone(),
            display_name,
            description,
            version,
            category,
            tags: vec![],
            tools: vec![],
            confidence,
            auto_generated,
            path: String::new(),
            created: Utc::now(),
        })
    }

    pub fn get_skill(&self, id: &str) -> Option<&SkillReference> {
        self.skills.get(id)
    }

    pub fn get_all_skills(&self) -> Vec<&SkillReference> {
        self.skills.values().collect()
    }

    pub fn search_skills(&self, keyword: &str) -> Vec<&SkillReference> {
        let keyword_lower = keyword.to_lowercase();
        self.skills.values()
            .filter(|skill| {
                skill.name.to_lowercase().contains(&keyword_lower) ||
                skill.description.to_lowercase().contains(&keyword_lower)
            })
            .collect()
    }

    pub fn count(&self) -> usize {
        self.skills.len()
    }
}

#[derive(Debug)]
pub enum SkillError {
    IoError(std::io::Error),
}

impl From<std::io::Error> for SkillError {
    fn from(e: std::io::Error) -> Self {
        SkillError::IoError(e)
    }
}
