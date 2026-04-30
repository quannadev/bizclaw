// Skill Registry
// Manage and discover skills

use std::collections::HashMap;
use std::sync::Arc;

use crate::skill::{KhazixSkill, SkillContext, SkillResult};

pub struct SkillRegistry {
    skills: HashMap<String, Arc<dyn KhazixSkill>>,
    by_category: HashMap<String, Vec<String>>,
    by_tag: HashMap<String, Vec<String>>,
}

impl SkillRegistry {
    pub fn new() -> Self {
        Self {
            skills: HashMap::new(),
            by_category: HashMap::new(),
            by_tag: HashMap::new(),
        }
    }
    
    pub fn register<S: KhazixSkill + 'static>(&mut self, skill: S) {
        let id = skill.metadata().id.clone();
        let category = skill.metadata().category.clone();
        
        self.skills.insert(id.clone(), Arc::new(skill));
        
        self.by_category
            .entry(category)
            .or_insert_with(Vec::new)
            .push(id.clone());
        
        for tag in &self.skills.get(&id).map(|s| s.metadata().tags.clone()).unwrap_or_default() {
            self.by_tag
                .entry(tag.clone())
                .or_insert_with(Vec::new)
                .push(id.clone());
        }
    }
    
    pub fn get(&self, id: &str) -> Option<Arc<dyn KhazixSkill>> {
        self.skills.get(id).cloned()
    }
    
    pub fn find_by_input(&self, input: &str) -> Option<Arc<dyn KhazixSkill>> {
        for skill in self.skills.values() {
            if skill.matches(input) {
                return Some(skill.clone());
            }
        }
        None
    }
    
    pub fn all(&self) -> Vec<Arc<dyn KhazixSkill>> {
        self.skills.values().cloned().collect()
    }
    
    pub fn by_category(&self, category: &str) -> Vec<Arc<dyn KhazixSkill>> {
        self.by_category
            .get(category)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.skills.get(id).cloned())
                    .collect()
            })
            .unwrap_or_default()
    }
    
    pub fn search(&self, query: &str) -> Vec<Arc<dyn KhazixSkill>> {
        let query_lower = query.to_lowercase();
        
        self.skills.values()
            .filter(|skill| {
                let m = skill.metadata();
                m.name.to_lowercase().contains(&query_lower) ||
                m.description.to_lowercase().contains(&query_lower) ||
                m.tags.iter().any(|t| t.to_lowercase().contains(&query_lower))
            })
            .cloned()
            .collect()
    }
    
    pub fn count(&self) -> usize {
        self.skills.len()
    }
    
    pub fn contains(&self, id: &str) -> bool {
        self.skills.contains_key(id)
    }
}

impl Default for SkillRegistry {
    fn default() -> Self {
        Self::new()
    }
}
