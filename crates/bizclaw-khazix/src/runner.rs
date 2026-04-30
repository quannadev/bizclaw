// Skill Runner
// Execute skills

use crate::skill::{SkillContext, SkillResult, SkillMetadata};
use crate::registry::SkillRegistry;

pub struct SkillRunner {
    registry: SkillRegistry,
}

impl SkillRunner {
    pub fn new() -> Self {
        Self {
            registry: SkillRegistry::new(),
        }
    }
    
    pub fn execute(&self, input: &str, context: SkillContext) -> Option<SkillResult> {
        let skill = self.registry.find_by_input(input)?;
        Some(skill.execute(&context))
    }
    
    pub fn execute_skill(&self, skill_id: &str, context: SkillContext) -> Option<SkillResult> {
        let skill = self.registry.get(skill_id)?;
        Some(skill.execute(&context))
    }
    
    pub fn list_skills(&self) -> Vec<SkillMetadata> {
        self.registry.all()
            .iter()
            .map(|s| s.metadata().clone())
            .collect()
    }
    
    pub fn search(&self, query: &str) -> Vec<SkillMetadata> {
        self.registry.search(query)
            .iter()
            .map(|s| s.metadata().clone())
            .collect()
    }
    
    pub fn get_skill(&self, id: &str) -> Option<SkillMetadata> {
        let skill = self.registry.get(id)?;
        Some(skill.metadata().clone())
    }
    
    pub fn get_prompt(&self, id: &str) -> Option<String> {
        let skill = self.registry.get(id)?;
        Some(skill.system_prompt())
    }
}

impl Default for SkillRunner {
    fn default() -> Self {
        Self::new()
    }
}
